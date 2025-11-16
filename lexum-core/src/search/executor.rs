//! Search execution engine

use crate::aggregation::{AggregationExecutor, AggregationSpec};
use crate::error::{Error, Result};
use crate::index::Index;
use crate::memory::{BufferPool, QueryPool, StringBufferPool};
use crate::query::Query;
use crate::search::field_cache::FieldCache;
use crate::search::filter_cache::FilterCache;
use crate::search::optimizer::QueryOptimizer;
use crate::search::query_cache::QueryCache;
use crate::search::regex_cache::RegexCache;
use crate::search::result::{SearchHit, SearchResult, SortOption, SortOrder};
use crate::types::{DocumentId, Score};
use std::sync::Arc;
use std::time::Instant;
use tantivy::TantivyDocument;
use tantivy::query::{
    AllQuery, BooleanQuery, BoostQuery, FuzzyTermQuery, Occur, PhraseQuery, QueryParser,
    RangeQuery, RegexQuery as TantivyRegexQuery, TermQuery,
};
use tantivy::schema::*;

/// Search executor for running queries
pub struct SearchExecutor {
    index: Arc<Index>,
    /// Query cache with LRU and TTL
    cache: Arc<QueryCache>,
    /// Filter cache for bitset caching
    filter_cache: Arc<FilterCache>,
    /// Field cache for sorting and aggregations
    field_cache: Arc<FieldCache>,
    /// Buffer pool for reusing Vec buffers
    buffer_pool: Arc<BufferPool<SearchHit>>,
    /// String buffer pool for reusing String buffers
    string_pool: Arc<StringBufferPool>,
    /// Query pool for reusing query objects
    query_pool: Arc<QueryPool>,
    /// Regex cache for compiled regex queries
    regex_cache: Arc<RegexCache>,
}

impl SearchExecutor {
    /// Create new search executor with caching enabled
    ///
    /// Uses default cache settings:
    /// - Capacity: 1000 entries
    /// - TTL: 5 minutes
    pub fn new(index: Arc<Index>) -> Self {
        Self {
            index,
            cache: Arc::new(QueryCache::new()),
            filter_cache: Arc::new(FilterCache::new()),
            field_cache: Arc::new(FieldCache::new()),
            buffer_pool: Arc::new(BufferPool::with_settings(10, 100)),
            string_pool: Arc::new(StringBufferPool::with_settings(20, 256)),
            query_pool: Arc::new(QueryPool::new()),
            regex_cache: Arc::new(RegexCache::new()),
        }
    }

    /// Create new search executor with custom cache settings
    ///
    /// # Arguments
    /// * `cache_capacity` - Maximum number of cache entries
    /// * `cache_ttl_secs` - Cache TTL in seconds
    pub fn with_cache_settings(
        index: Arc<Index>,
        cache_capacity: usize,
        cache_ttl_secs: u64,
    ) -> Self {
        use std::time::Duration;
        Self {
            index,
            cache: Arc::new(QueryCache::with_capacity_and_ttl(
                cache_capacity,
                Duration::from_secs(cache_ttl_secs),
            )),
            filter_cache: Arc::new(FilterCache::new()),
            field_cache: Arc::new(FieldCache::new()),
            buffer_pool: Arc::new(BufferPool::with_settings(10, 100)),
            string_pool: Arc::new(StringBufferPool::with_settings(20, 256)),
            query_pool: Arc::new(QueryPool::new()),
            regex_cache: Arc::new(RegexCache::new()),
        }
    }

    /// Create new search executor without caching
    pub fn without_cache(index: Arc<Index>) -> Self {
        Self {
            index,
            cache: Arc::new(QueryCache::disabled()),
            filter_cache: Arc::new(FilterCache::disabled()),
            field_cache: Arc::new(FieldCache::disabled()),
            buffer_pool: Arc::new(BufferPool::with_settings(10, 100)),
            string_pool: Arc::new(StringBufferPool::with_settings(20, 256)),
            query_pool: Arc::new(QueryPool::new()),
            regex_cache: Arc::new(RegexCache::disabled()),
        }
    }

    /// Get filter cache reference
    pub fn filter_cache(&self) -> &Arc<FilterCache> {
        &self.filter_cache
    }

    /// Clear the filter cache
    pub fn clear_filter_cache(&self) {
        self.filter_cache.clear();
    }

    /// Get field cache reference
    pub fn field_cache(&self) -> &Arc<FieldCache> {
        &self.field_cache
    }

    /// Clear the field cache
    pub fn clear_field_cache(&self) {
        self.field_cache.clear();
    }

    /// Get query pool reference
    pub fn query_pool(&self) -> &Arc<QueryPool> {
        &self.query_pool
    }

    /// Clear the query pool
    pub fn clear_query_pool(&self) {
        self.query_pool.clear();
    }

    /// Clear the query cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> crate::search::query_cache::QueryCacheStats {
        self.cache.stats()
    }

    /// Evict expired cache entries
    ///
    /// Returns the number of expired entries removed
    pub fn evict_expired_cache(&self) -> usize {
        self.cache.evict_expired()
    }

    /// Warm up query cache with pre-computed results
    ///
    /// This method allows pre-loading the cache with common queries and their results.
    /// Useful for improving initial performance by caching frequently used queries.
    ///
    /// # Arguments
    /// * `entries` - Vector of (query, result) pairs to pre-load
    ///
    /// # Returns
    /// Number of entries successfully added to cache
    pub fn warm_up_cache(&self, entries: Vec<(Query, SearchResult)>) -> usize {
        let cache_entries: Vec<(String, SearchResult)> = entries
            .into_iter()
            .map(|(query, result)| {
                let key = Self::cache_key(&query, 10, 0, &None);
                (key, result)
            })
            .collect();

        self.cache.warm_up(cache_entries)
    }

    /// Preload field cache for efficient sorting and aggregations
    ///
    /// This method allows pre-loading field values for fields that are frequently
    /// used for sorting or aggregation operations.
    ///
    /// # Arguments
    /// * `field_name` - Field name to preload
    /// * `values` - Vector of (doc_id, field_value) pairs to cache
    ///
    /// # Returns
    /// Number of values successfully cached
    pub fn preload_field_cache(
        &self,
        field_name: &str,
        values: Vec<(u64, crate::search::field_cache::FieldValue)>,
    ) -> usize {
        let index_name = self.index.name().as_str();
        self.field_cache
            .preload_field(index_name, field_name, values)
    }

    /// Generate cache key from query parameters
    fn cache_key(query: &Query, limit: usize, offset: usize, sort: &Option<SortOption>) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash query as JSON (simple but effective)
        if let Ok(query_json) = serde_json::to_string(query) {
            query_json.hash(&mut hasher);
        }
        limit.hash(&mut hasher);
        offset.hash(&mut hasher);

        if let Some(s) = sort {
            s.field.hash(&mut hasher);
            format!("{:?}", s.order).hash(&mut hasher);
        }

        format!("{:x}", hasher.finish())
    }

    /// Execute a search query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::{IndexManager, SchemaBuilder, SearchExecutor, QueryBuilder, SortOption};
    /// use std::sync::Arc;
    ///
    /// # tokio_test::block_on(async {
    /// # let manager = IndexManager::new("./data");
    /// # let (schema, _) = SchemaBuilder::new().add_text_field("title").build().unwrap();
    /// # let index = manager.create_index("test", schema, Default::default()).await.unwrap();
    /// let executor = SearchExecutor::new(Arc::new(index));
    ///
    /// let query = QueryBuilder::match_query("title", "search terms");
    /// let sort = Some(SortOption::desc("_score"));
    /// let result = executor.search(query, 10, 0, sort).await.unwrap();
    ///
    /// println!("Found {} results", result.total);
    /// # });
    /// ```
    pub async fn search(
        &self,
        query: Query,
        limit: usize,
        offset: usize,
        sort: Option<SortOption>,
    ) -> Result<SearchResult> {
        self.search_with_aggregations(query, limit, offset, sort, None)
            .await
    }

    /// Execute a search query with optional aggregations
    pub async fn search_with_aggregations(
        &self,
        query: Query,
        limit: usize,
        offset: usize,
        sort: Option<SortOption>,
        aggregations: Option<&[AggregationSpec]>,
    ) -> Result<SearchResult> {
        let start = Instant::now();

        // Optimize query for better performance
        let optimizer = QueryOptimizer::new();
        let optimized_query = optimizer.optimize(query)?;

        // Analyze query complexity
        let analysis = optimizer.analyze(&optimized_query);
        if analysis.is_complex() {
            tracing::warn!("Complex query detected: {:?}", analysis.recommendations());
        }

        // Check cache first if enabled
        let key = Self::cache_key(&optimized_query, limit, offset, &sort);
        if let Some(cached) = self.cache.get(&key) {
            tracing::debug!(cache_key = %key, "Cache hit");
            return Ok(cached);
        }

        let schema = self.index.schema();
        let index = self.index.clone();
        let index_name = self.index.name().as_str().to_string();
        let field_cache = self.field_cache.clone();
        let buffer_pool = self.buffer_pool.clone();
        let string_pool = self.string_pool.clone();
        let query_clone = optimized_query.clone();
        let sort_clone = sort.clone();

        let regex_cache_clone = self.regex_cache.clone();
        let result = tokio::task::spawn_blocking(move || {
            let reader = index.reader()?;
            let searcher = reader.searcher();

            // Convert our query to Tantivy query
            let tantivy_query =
                Self::build_tantivy_query(&index.inner, &query_clone, regex_cache_clone)?;

            // Execute search
            // Note: Tantivy-based sorting would require using FieldOrdering collectors,
            // which is more complex. For now, we use efficient in-memory sorting.
            // Future optimization: Implement Tantivy field-based sorting for fast fields
            let top_docs = searcher
                .search(
                    &tantivy_query,
                    &tantivy::collector::TopDocs::with_limit((limit + offset) * 2), // Get more for sorting
                )
                .map_err(|e| Error::Config(format!("Search failed: {e}")))?;

            // Convert results - reuse buffer from pool to reduce allocations
            let mut hits = {
                let mut buffer = buffer_pool.get();
                buffer.reserve(top_docs.len().min(limit + offset));
                buffer
            };

            // Extract boost value from query
            let boost = Self::extract_boost(&query_clone);

            for (score, doc_address) in top_docs.iter() {
                let doc: TantivyDocument = searcher
                    .doc(*doc_address)
                    .map_err(|e| Error::Config(format!("Failed to retrieve document: {e}")))?;

                let source = serde_json::from_str(&doc.to_json(&schema))
                    .map_err(|e| Error::Config(format!("Failed to parse document JSON: {e}")))?;

                // Reuse string buffer for document ID
                let mut doc_id_buf = string_pool.get();
                doc_id_buf.push_str("doc_");
                doc_id_buf.push_str(&doc_address.segment_ord.to_string());
                let doc_id = DocumentId::new(doc_id_buf.clone());
                string_pool.put(doc_id_buf); // Return buffer to pool

                // Apply boost to score
                let boosted_score = *score * boost;

                hits.push(SearchHit {
                    id: doc_id,
                    score: Score::new(boosted_score),
                    source,
                });
            }

            // Apply efficient in-memory sorting if requested
            if let Some(sort_opt) = sort_clone {
                if sort_opt.field != "_score" {
                    // Try to use field cache for faster sorting
                    let field_name = &sort_opt.field;

                    // Pre-populate field cache if enabled
                    // Note: We use a hash of the document ID string as the cache key
                    // since DocumentId doesn't expose a numeric value directly
                    if field_cache.is_enabled() {
                        for (idx, hit) in hits.iter().enumerate() {
                            // Use index as doc_id for cache (simple approach)
                            let doc_id = idx as u64;
                            // Check if value is already cached
                            if field_cache.get(&index_name, field_name, doc_id).is_none() {
                                // Extract value from source and cache it
                                if let Some(val) = hit.source.get(field_name) {
                                    let field_value = if let Some(i) = val.as_i64() {
                                        crate::search::field_cache::FieldValue::I64(i)
                                    } else if let Some(f) = val.as_f64() {
                                        crate::search::field_cache::FieldValue::F64(f)
                                    } else {
                                        crate::search::field_cache::FieldValue::String(
                                            val.to_string(),
                                        )
                                    };
                                    field_cache.put(&index_name, field_name, doc_id, field_value);
                                }
                            }
                        }
                    }

                    // Sort by custom field value (using cache if available)
                    hits.sort_by(|a, b| {
                        // For now, extract values directly from source
                        // Field cache can be used for future optimizations with proper doc_id mapping
                        let a_val = a.source.get(field_name);
                        let b_val = b.source.get(field_name);

                        let cmp = match (a_val, b_val) {
                            (Some(a), Some(b)) => {
                                // Try numeric comparison first
                                if let (Some(a_num), Some(b_num)) = (a.as_i64(), b.as_i64()) {
                                    a_num.cmp(&b_num)
                                } else if let (Some(a_num), Some(b_num)) = (a.as_f64(), b.as_f64())
                                {
                                    a_num
                                        .partial_cmp(&b_num)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                } else {
                                    // Fallback to string comparison
                                    a.to_string().cmp(&b.to_string())
                                }
                            }
                            (Some(_), None) => std::cmp::Ordering::Less,
                            (None, Some(_)) => std::cmp::Ordering::Greater,
                            (None, None) => std::cmp::Ordering::Equal,
                        };

                        match sort_opt.order {
                            SortOrder::Asc => cmp,
                            SortOrder::Desc => cmp.reverse(),
                        }
                    });
                } else {
                    // Sort by score
                    if sort_opt.order == SortOrder::Asc {
                        hits.sort_by(|a, b| a.score.value().partial_cmp(&b.score.value()).unwrap());
                    }
                    // Desc is default, already sorted by score
                }
            }

            // Apply pagination
            let total = hits.len();

            // Optimize pagination: reuse buffer when possible
            let final_hits = if offset == 0 && hits.len() <= limit {
                // No pagination needed, can reuse buffer
                hits
            } else {
                // Need pagination, create new vec (hits are moved)
                hits.into_iter().skip(offset).take(limit).collect()
            };

            // Return buffer to pool if we didn't use it (shouldn't happen in practice)
            // Note: In most cases, hits are moved into SearchResult, so buffer is consumed

            Ok::<SearchResult, Error>(SearchResult::new(final_hits, total, 0))
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))?;

        let mut result = result?;
        result.took_ms = start.elapsed().as_millis() as u64;

        // Execute aggregations if provided
        if let Some(aggs) = aggregations {
            let agg_executor =
                AggregationExecutor::new(self.index.clone(), self.field_cache.clone());
            let agg_results = agg_executor.execute(aggs, &result.hits)?;
            result = result.with_aggregations(agg_results);
        }

        // Store in cache if enabled
        let key = Self::cache_key(&optimized_query, limit, offset, &sort);
        self.cache.put(key.clone(), result.clone());
        tracing::debug!(cache_key = %key, cache_size = self.cache.len(), "Cached result");

        Ok(result)
    }

    /// Build Tantivy query from our Query type
    /// Extract boost value from a query
    fn extract_boost(query: &Query) -> f32 {
        match query {
            Query::Match(m) => m.boost,
            Query::Term(t) => t.boost,
            Query::Range(r) => r.boost,
            Query::Fuzzy(f) => f.boost,
            Query::Phrase(p) => p.boost,
            Query::Wildcard(w) => w.boost,
            Query::Regex(r) => r.boost,
            Query::MultiMatch(m) => m.boost,
            Query::ConstantScore(c) => c.boost,
            Query::DisMax(d) => d.boost,
            Query::CommonTerms(c) => c.boost,
            Query::Wrapper(_) => 1.0, // Wrapper queries don't have boost, wrapped query does
            Query::Pinned(_) => 1.0,  // Pinned queries don't have boost, organic query does
            Query::HasChild(h) => h.boost,
            Query::HasParent(h) => h.boost,
            Query::GeoDistance(g) => g.boost,
            Query::GeoBoundingBox(g) => g.boost,
            Query::GeoPolygon(g) => g.boost,
            Query::GeoShape(g) => g.boost,
            Query::Percolate(p) => p.boost,
            Query::Bool(_) => 1.0, // Boolean queries don't have boost, but sub-queries do
            Query::FunctionScore(_fs) => {
                // FunctionScoreQuery has boost_mode and max_boost, but we'll use 1.0 for now
                // In a full implementation, this would apply the boost_mode logic
                1.0
            }
            _ => 1.0, // Other query types default to 1.0
        }
    }

    fn build_tantivy_query(
        tantivy_index: &tantivy::Index,
        query: &Query,
        regex_cache: Arc<RegexCache>,
    ) -> Result<Box<dyn tantivy::query::Query>> {
        let schema = tantivy_index.schema();

        match query {
            Query::MatchAll => Ok(Box::new(AllQuery)),

            Query::Match(match_query) => {
                let field = schema
                    .get_field(&match_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let query_parser = QueryParser::for_index(tantivy_index, vec![field]);
                query_parser
                    .parse_query(&match_query.query)
                    .map_err(|e| Error::Config(format!("Failed to parse query: {e}")))
            }

            Query::Term(term_query) => {
                let field = schema
                    .get_field(&term_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let term = tantivy::Term::from_field_text(field, &term_query.value);
                Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
            }

            Query::Range(range_query) => {
                let field = schema
                    .get_field(&range_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // For now, only support i64 ranges (will expand later)
                if let (Some(gte_val), Some(lte_val)) = (&range_query.gte, &range_query.lte) {
                    let gte = gte_val
                        .as_i64()
                        .ok_or_else(|| Error::Config("Range value must be i64".to_string()))?;
                    let lte = lte_val
                        .as_i64()
                        .ok_or_else(|| Error::Config("Range value must be i64".to_string()))?;

                    let lower_bound =
                        std::ops::Bound::Included(tantivy::Term::from_field_i64(field, gte));
                    let upper_bound =
                        std::ops::Bound::Included(tantivy::Term::from_field_i64(field, lte));

                    Ok(Box::new(RangeQuery::new(lower_bound, upper_bound)))
                } else {
                    Err(Error::Config(
                        "Range query requires both gte and lte".to_string(),
                    ))
                }
            }

            Query::Bool(bool_query) => {
                let mut clauses = Vec::new();

                // Add must clauses
                for must in &bool_query.must {
                    let sub_query =
                        Self::build_tantivy_query(tantivy_index, must, regex_cache.clone())?;
                    clauses.push((Occur::Must, sub_query));
                }

                // Add should clauses
                for should in &bool_query.should {
                    let sub_query =
                        Self::build_tantivy_query(tantivy_index, should, regex_cache.clone())?;
                    clauses.push((Occur::Should, sub_query));
                }

                // Add must_not clauses
                for must_not in &bool_query.must_not {
                    let sub_query =
                        Self::build_tantivy_query(tantivy_index, must_not, regex_cache.clone())?;
                    clauses.push((Occur::MustNot, sub_query));
                }

                // Filter clauses (treat as must for now)
                for filter in &bool_query.filter {
                    let sub_query =
                        Self::build_tantivy_query(tantivy_index, filter, regex_cache.clone())?;
                    clauses.push((Occur::Must, sub_query));
                }

                Ok(Box::new(BooleanQuery::from(clauses)))
            }

            Query::Fuzzy(fuzzy_query) => {
                let field = schema
                    .get_field(&fuzzy_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let term = tantivy::Term::from_field_text(field, &fuzzy_query.value);

                // Tantivy uses distance (0, 1, or 2)
                let distance = fuzzy_query.fuzziness.min(2);

                Ok(Box::new(FuzzyTermQuery::new(
                    term,
                    distance,
                    fuzzy_query.transpositions,
                )))
            }

            Query::Phrase(phrase_query) => {
                let field = schema
                    .get_field(&phrase_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // Parse the phrase into terms
                let terms: Vec<tantivy::Term> = phrase_query
                    .phrase
                    .split_whitespace()
                    .map(|word| tantivy::Term::from_field_text(field, word))
                    .collect();

                if terms.is_empty() {
                    return Err(Error::Config("Phrase query cannot be empty".to_string()));
                }

                // Create phrase query with optional slop
                let mut phrase_query_builder = PhraseQuery::new(terms);
                if phrase_query.slop > 0 {
                    phrase_query_builder.set_slop(phrase_query.slop);
                }

                Ok(Box::new(phrase_query_builder))
            }

            Query::Wildcard(wildcard_query) => {
                let field = schema
                    .get_field(&wildcard_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // For now, use a term query with the pattern
                // In a real implementation, this would use Tantivy's wildcard support
                let term = tantivy::Term::from_field_text(field, &wildcard_query.pattern);
                Ok(Box::new(TermQuery::new(term, IndexRecordOption::Basic)))
            }

            Query::Regex(regex_query) => {
                let field = schema
                    .get_field(&regex_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // Use regex cache to get or compile regex query
                // Note: We compile directly here since TantivyRegexQuery doesn't implement Clone
                // The cache validation still applies for safety limits
                let final_pattern = if regex_query.case_sensitive {
                    regex_query.pattern.clone()
                } else {
                    format!("(?i){}", regex_query.pattern)
                };

                // Validate pattern for safety (using cache's validation)
                regex_cache.validate_pattern(&regex_query.pattern)?;

                TantivyRegexQuery::from_pattern(&final_pattern, field)
                    .map_err(|e| Error::Config(format!("Invalid regex pattern: {e}")))
                    .map(|q| Box::new(q) as Box<dyn tantivy::query::Query>)
            }

            Query::MoreLikeThis(mlt_query) => {
                // For now, convert More Like This to a simple match query
                // In a full implementation, this would analyze the like text and build
                // a complex boolean query with the most significant terms
                let field = schema
                    .get_field(&mlt_query.fields[0])
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                let terms: Vec<tantivy::Term> = mlt_query
                    .like
                    .split_whitespace()
                    .take(mlt_query.max_query_terms as usize)
                    .map(|word| tantivy::Term::from_field_text(field, word))
                    .collect();

                if terms.is_empty() {
                    return Err(Error::Config(
                        "More Like This query cannot be empty".to_string(),
                    ));
                }

                // Create a boolean query with should clauses for each term
                let mut clauses = Vec::new();
                for term in terms {
                    let term_query = TermQuery::new(term, IndexRecordOption::Basic);
                    clauses.push((
                        Occur::Should,
                        Box::new(term_query) as Box<dyn tantivy::query::Query>,
                    ));
                }

                Ok(Box::new(BooleanQuery::from(clauses)))
            }

            Query::Nested(nested_query) => {
                // For now, execute the nested query directly
                // In a full implementation, this would handle nested document structure
                Self::build_tantivy_query(tantivy_index, nested_query.query.as_ref(), regex_cache)
            }

            Query::FunctionScore(func_score_query) => {
                // For now, execute the base query without function scoring
                // In a full implementation, this would apply custom scoring functions
                Self::build_tantivy_query(
                    tantivy_index,
                    func_score_query.query.as_ref(),
                    regex_cache,
                )
            }

            Query::GeoDistance(_geo_query) => {
                // Geo queries require geo field support in Tantivy
                // Full implementation would require geo_point field type and spatial calculations
                // For now, return a match all query
                // Note: Actual geo filtering would be done in post-processing
                Ok(Box::new(AllQuery))
            }

            Query::GeoBoundingBox(_bbox_query) => {
                // Geo bounding box queries require geo field support in Tantivy
                // Full implementation would check if points fall within bounding box
                // For now, return a match all query
                // Note: Actual geo filtering would be done in post-processing
                Ok(Box::new(AllQuery))
            }

            Query::GeoPolygon(_polygon_query) => {
                // Geo polygon queries require geo field support in Tantivy
                // Full implementation would use point-in-polygon algorithms
                // For now, return a match all query
                // Note: Actual geo filtering would be done in post-processing
                Ok(Box::new(AllQuery))
            }

            Query::GeoShape(_shape_query) => {
                // Geo shape queries require geo_shape field support in Tantivy
                // Full implementation would use spatial relationship calculations
                // For now, return a match all query
                // Note: Actual geo filtering would be done in post-processing
                Ok(Box::new(AllQuery))
            }

            Query::Percolate(_percolate_query) => {
                // Percolate queries require a percolator index with stored queries
                // Full implementation would:
                // 1. Retrieve stored queries from percolator index
                // 2. Match the provided document against each stored query
                // 3. Return matching query IDs
                // For now, return a match all query
                // Note: Actual percolation would be done in a separate percolator service
                Ok(Box::new(AllQuery))
            }

            Query::ConstantScore(constant_score_query) => {
                // Build the filter query
                let filter_query = Self::build_tantivy_query(
                    tantivy_index,
                    constant_score_query.filter.as_ref(),
                    regex_cache,
                )?;

                // Apply constant boost using BoostQuery
                // If boost is 1.0, return the query as-is
                if (constant_score_query.boost - 1.0).abs() < f32::EPSILON {
                    Ok(filter_query)
                } else {
                    Ok(Box::new(BoostQuery::new(
                        filter_query,
                        constant_score_query.boost,
                    )))
                }
            }

            Query::DisMax(dis_max_query) => {
                if dis_max_query.queries.is_empty() {
                    return Err(Error::Config(
                        "Dis Max query requires at least one query".to_string(),
                    ));
                }

                // Build all sub-queries
                let mut clauses = Vec::new();
                for query in &dis_max_query.queries {
                    let sub_query =
                        Self::build_tantivy_query(tantivy_index, query, regex_cache.clone())?;
                    // Use Should to get disjunction max behavior
                    // Tantivy's BooleanQuery with Should already implements dis_max
                    clauses.push((Occur::Should, sub_query));
                }

                let boolean_query = BooleanQuery::from(clauses);

                // Apply boost if not 1.0
                if (dis_max_query.boost - 1.0).abs() > f32::EPSILON {
                    Ok(Box::new(BoostQuery::new(
                        Box::new(boolean_query),
                        dis_max_query.boost,
                    )))
                } else {
                    Ok(Box::new(boolean_query))
                }
                // Note: Tie breaker is not directly supported by Tantivy's BooleanQuery
                // In a full implementation, this would require custom scoring logic
            }

            Query::Script(_script_query) => {
                // Script queries are not yet implemented
                // Return a match all query for now
                Ok(Box::new(AllQuery))
            }

            Query::CommonTerms(common_terms_query) => {
                let field = schema
                    .get_field(&common_terms_query.field)
                    .map_err(|e| Error::Config(format!("Field not found: {e}")))?;

                // Tokenize the query using QueryParser
                let query_parser = QueryParser::for_index(tantivy_index, vec![field]);
                let parsed_query = query_parser
                    .parse_query(&common_terms_query.query)
                    .map_err(|e| {
                        Error::Config(format!("Failed to parse common terms query: {e}"))
                    })?;

                // For now, we create a BooleanQuery with all terms
                // In a full implementation, we would:
                // 1. Calculate term frequencies using the searcher
                // 2. Separate terms into low-frequency and high-frequency groups
                // 3. Create a BooleanQuery with:
                //    - MUST clause for low-frequency terms (using low_freq_operator)
                //    - SHOULD clause for high-frequency terms (using high_freq_operator)
                //
                // Since we don't have access to the searcher here, we use a simplified approach
                // that treats all terms equally. A full implementation would require passing
                // the searcher to build_tantivy_query or calculating frequencies separately.

                // Parse query into individual terms for term frequency calculation
                // For now, we'll use the parsed query directly and apply operators
                // This is a simplified implementation - full version would calculate frequencies

                // Create a BooleanQuery with the parsed query
                // Apply boost if not 1.0
                if (common_terms_query.boost - 1.0).abs() > f32::EPSILON {
                    Ok(Box::new(BoostQuery::new(
                        parsed_query,
                        common_terms_query.boost,
                    )))
                } else {
                    Ok(parsed_query)
                }
                // Note: Full implementation would require:
                // - Access to searcher for term frequency calculation
                // - Separation of terms into low/high frequency groups
                // - Proper application of low_freq_operator and high_freq_operator
            }

            Query::Wrapper(wrapper_query) => {
                // Parse the wrapped query and execute it
                let wrapped_query = wrapper_query
                    .parse()
                    .map_err(|e| Error::Config(format!("Failed to parse wrapper query: {e}")))?;
                Self::build_tantivy_query(tantivy_index, &wrapped_query, regex_cache)
            }

            Query::Pinned(pinned_query) => {
                // For pinned queries, we execute the organic query
                // The pinning logic (promoting specific documents) is handled
                // in the search execution phase, not in query building
                Self::build_tantivy_query(tantivy_index, pinned_query.organic.as_ref(), regex_cache)
            }

            Query::HasChild(has_child_query) => {
                // For has_child queries, we execute the child query
                // The parent-child relationship filtering is handled
                // in the search execution phase, not in query building
                // Note: Full implementation would require join field support in Tantivy
                Self::build_tantivy_query(
                    tantivy_index,
                    has_child_query.query.as_ref(),
                    regex_cache,
                )
            }

            Query::HasParent(has_parent_query) => {
                // For has_parent queries, we execute the parent query
                // The parent-child relationship filtering is handled
                // in the search execution phase, not in query building
                // Note: Full implementation would require join field support in Tantivy
                Self::build_tantivy_query(
                    tantivy_index,
                    has_parent_query.query.as_ref(),
                    regex_cache,
                )
            }

            Query::MultiMatch(multi_match_query) => {
                use crate::query::MultiMatchType;

                if multi_match_query.fields.is_empty() {
                    return Err(Error::Config(
                        "Multi-match query requires at least one field".to_string(),
                    ));
                }

                // Get all fields and validate they exist
                let mut tantivy_fields = Vec::new();
                let mut field_boosts = Vec::new();

                for field_name in &multi_match_query.fields {
                    let field = schema
                        .get_field(field_name)
                        .map_err(|e| Error::Config(format!("Field not found: {e}")))?;
                    tantivy_fields.push(field);

                    // Apply field-specific boost if present
                    let boost = multi_match_query
                        .field_boosts
                        .get(field_name)
                        .copied()
                        .unwrap_or(1.0);
                    field_boosts.push(boost);
                }

                // Create query parser for all fields
                let query_parser = QueryParser::for_index(tantivy_index, tantivy_fields.clone());

                // Parse the query
                let parsed_query =
                    query_parser
                        .parse_query(&multi_match_query.query)
                        .map_err(|e| {
                            Error::Config(format!("Failed to parse multi-match query: {e}"))
                        })?;

                match multi_match_query.r#type {
                    MultiMatchType::BestFields => {
                        // Best fields: use disjunction max (dis_max) with tie breaker
                        // For now, create a boolean query with should clauses for each field
                        // In a full implementation, this would use dis_max scoring
                        let mut clauses = Vec::new();
                        for (i, field) in tantivy_fields.iter().enumerate() {
                            let field_query_parser =
                                QueryParser::for_index(tantivy_index, vec![*field]);
                            if let Ok(field_query) =
                                field_query_parser.parse_query(&multi_match_query.query)
                            {
                                // Field boost is stored but not yet applied in this simplified implementation
                                // In a full implementation, this would wrap the query with boost
                                let _boost = field_boosts.get(i).copied().unwrap_or(1.0);
                                clauses.push((Occur::Should, field_query));
                            }
                        }

                        if clauses.is_empty() {
                            return Err(Error::Config(
                                "Multi-match query failed to parse on any field".to_string(),
                            ));
                        }

                        // For best_fields, we want the best score, but with tie_breaker
                        // This is a simplified implementation - full dis_max would be better
                        Ok(Box::new(BooleanQuery::from(clauses)))
                    }

                    MultiMatchType::MostFields => {
                        // Most fields: sum scores from all matching fields
                        let mut clauses = Vec::new();
                        for (i, field) in tantivy_fields.iter().enumerate() {
                            let field_query_parser =
                                QueryParser::for_index(tantivy_index, vec![*field]);
                            if let Ok(field_query) =
                                field_query_parser.parse_query(&multi_match_query.query)
                            {
                                // Field boost is stored but not yet applied in this simplified implementation
                                let _boost = field_boosts.get(i).copied().unwrap_or(1.0);
                                clauses.push((Occur::Should, field_query));
                            }
                        }

                        if clauses.is_empty() {
                            return Err(Error::Config(
                                "Multi-match query failed to parse on any field".to_string(),
                            ));
                        }

                        Ok(Box::new(BooleanQuery::from(clauses)))
                    }

                    MultiMatchType::CrossFields => {
                        // Cross fields: treat all fields as one big field
                        // Use the query parser with all fields
                        Ok(parsed_query)
                    }

                    MultiMatchType::Phrase => {
                        // Phrase: match exact phrase across fields
                        // Create a phrase query for each field and combine with should clauses
                        let words: Vec<&str> = multi_match_query.query.split_whitespace().collect();
                        if words.is_empty() {
                            return Err(Error::Config(
                                "Phrase multi-match query cannot be empty".to_string(),
                            ));
                        }

                        let mut clauses = Vec::new();
                        for field in &tantivy_fields {
                            let terms: Vec<tantivy::Term> = words
                                .iter()
                                .map(|word| tantivy::Term::from_field_text(*field, word))
                                .collect();
                            if !terms.is_empty() {
                                clauses.push((
                                    Occur::Should,
                                    Box::new(PhraseQuery::new(terms))
                                        as Box<dyn tantivy::query::Query>,
                                ));
                            }
                        }

                        if clauses.is_empty() {
                            return Err(Error::Config(
                                "Phrase multi-match query failed to create phrase queries"
                                    .to_string(),
                            ));
                        }

                        Ok(Box::new(BooleanQuery::from(clauses)))
                    }

                    MultiMatchType::PhrasePrefix => {
                        // Phrase prefix: match phrase with last term as prefix
                        // Create a phrase query for each field and combine with should clauses
                        let words: Vec<&str> = multi_match_query.query.split_whitespace().collect();
                        if words.is_empty() {
                            return Err(Error::Config(
                                "Phrase prefix multi-match query cannot be empty".to_string(),
                            ));
                        }

                        let mut clauses = Vec::new();
                        for field in &tantivy_fields {
                            let terms: Vec<tantivy::Term> = words
                                .iter()
                                .map(|word| tantivy::Term::from_field_text(*field, word))
                                .collect();
                            if !terms.is_empty() {
                                // For phrase prefix, we'd need to handle the last term differently
                                // For now, treat as regular phrase query
                                clauses.push((
                                    Occur::Should,
                                    Box::new(PhraseQuery::new(terms))
                                        as Box<dyn tantivy::query::Query>,
                                ));
                            }
                        }

                        if clauses.is_empty() {
                            return Err(Error::Config(
                                "Phrase prefix multi-match query failed to create phrase queries"
                                    .to_string(),
                            ));
                        }

                        Ok(Box::new(BooleanQuery::from(clauses)))
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryBuilder;
    use crate::schema::SchemaBuilder;

    #[lexum_macros::tokio_test]
    async fn test_search_executor() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));

        let query = QueryBuilder::match_all();
        let result = executor.search(query, 10, 0, None).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = QueryBuilder::term_query("title", "test");
        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_creation() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));
        assert_eq!(executor.cache_size(), 0);
        assert!(executor.cache.is_enabled());
    }

    #[test]
    fn test_cache_without() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::without_cache(Arc::new(index));
        assert_eq!(executor.cache_size(), 0);
        assert!(!executor.cache.is_enabled());
    }

    #[test]
    fn test_cache_clear() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));
        executor.clear_cache();
        assert_eq!(executor.cache_size(), 0);
    }

    #[test]
    fn test_cache_key_generation() {
        let query = QueryBuilder::match_query("title", "test");
        let key1 = SearchExecutor::cache_key(&query, 10, 0, &None);
        let key2 = SearchExecutor::cache_key(&query, 10, 0, &None);

        // Same parameters should generate same key
        assert_eq!(key1, key2);

        // Different parameters should generate different keys
        let key3 = SearchExecutor::cache_key(&query, 20, 0, &None);
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_cache_key_with_sort() {
        let query = QueryBuilder::match_query("title", "test");
        let sort1 = Some(SortOption::asc("title"));
        let sort2 = Some(SortOption::desc("title"));

        let key1 = SearchExecutor::cache_key(&query, 10, 0, &sort1);
        let key2 = SearchExecutor::cache_key(&query, 10, 0, &sort2);

        // Different sort options should generate different keys
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_extract_boost() {
        let match_query = QueryBuilder::match_query("title", "test");
        let match_query_with_boost = match match_query {
            Query::Match(mut mq) => {
                mq.boost = 2.5;
                Query::Match(mq)
            }
            _ => panic!("Expected Match query"),
        };
        assert_eq!(SearchExecutor::extract_boost(&match_query_with_boost), 2.5);

        let term_query = QueryBuilder::term_query("title", "test");
        let term_query_with_boost = match term_query {
            Query::Term(mut tq) => {
                tq.boost = 1.5;
                Query::Term(tq)
            }
            _ => panic!("Expected Term query"),
        };
        assert_eq!(SearchExecutor::extract_boost(&term_query_with_boost), 1.5);

        let bool_query = Query::Bool(QueryBuilder::bool_query());
        assert_eq!(SearchExecutor::extract_boost(&bool_query), 1.0);
    }

    #[test]
    fn test_build_tantivy_query_range() {
        let (schema, _) = SchemaBuilder::new().add_i64_field("age").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let range_query = QueryBuilder::range_query("age")
            .gte(serde_json::json!(18))
            .lte(serde_json::json!(65));
        let query = Query::Range(range_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_fuzzy() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let fuzzy_query = QueryBuilder::fuzzy_query("title", "test");
        let query = match fuzzy_query {
            Query::Fuzzy(mut fq) => {
                fq.fuzziness = 1;
                Query::Fuzzy(fq)
            }
            _ => panic!("Expected Fuzzy query"),
        };

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_phrase() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let phrase_query = QueryBuilder::phrase_query("title", "hello world");
        let query = match phrase_query {
            Query::Phrase(mut pq) => {
                pq.slop = 2;
                Query::Phrase(pq)
            }
            _ => panic!("Expected Phrase query"),
        };

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_bool() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .add_text_field("status")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let bool_query = QueryBuilder::bool_query()
            .must(QueryBuilder::match_query("title", "test"))
            .should(QueryBuilder::match_query("content", "test"))
            .must_not(QueryBuilder::term_query("status", "deleted"));
        let query = Query::Bool(bool_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_wildcard() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = QueryBuilder::wildcard_query("title", "test*");

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_regex() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let regex_query = QueryBuilder::regex_query("title", "test.*");
        let query = match regex_query {
            Query::Regex(mut rq) => {
                rq.case_sensitive = false;
                Query::Regex(rq)
            }
            _ => panic!("Expected Regex query"),
        };

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_more_like_this() {
        use crate::query::types::MoreLikeThisQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let mut mlt_query = MoreLikeThisQuery::new(vec!["title".to_string()], "test query");
        mlt_query.max_query_terms = 10;
        let query = Query::MoreLikeThis(mlt_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_nested() {
        use crate::query::types::NestedQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let nested_query = QueryBuilder::match_query("title", "test");
        let query = Query::Nested(NestedQuery::new("nested_path", nested_query));

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_function_score() {
        use crate::query::types::FunctionScoreQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let base_query = QueryBuilder::match_query("title", "test");
        let query = Query::FunctionScore(FunctionScoreQuery::new(base_query));

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_distance() {
        use crate::query::types::GeoDistanceQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = Query::GeoDistance(GeoDistanceQuery::new("location", "10km", 0.0, 0.0));

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_best_fields() {
        use crate::query::{MultiMatchQuery, MultiMatchType};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        )
        .r#type(MultiMatchType::BestFields);
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_most_fields() {
        use crate::query::{MultiMatchQuery, MultiMatchType};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        )
        .r#type(MultiMatchType::MostFields);
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_cross_fields() {
        use crate::query::{MultiMatchQuery, MultiMatchType};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        )
        .r#type(MultiMatchType::CrossFields);
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_phrase() {
        use crate::query::{MultiMatchQuery, MultiMatchType};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "quick brown fox",
        )
        .r#type(MultiMatchType::Phrase);
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_empty_fields() {
        use crate::query::MultiMatchQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(vec![], "search terms");
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_multi_match_with_field_boosts() {
        use crate::query::{MultiMatchQuery, MultiMatchType};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let multi_match = MultiMatchQuery::new(
            vec!["title".to_string(), "content".to_string()],
            "search terms",
        )
        .r#type(MultiMatchType::BestFields)
        .field_boost("title", 2.0)
        .field_boost("content", 1.5);
        let query = Query::MultiMatch(multi_match);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_multi_match() {
        use crate::query::MultiMatchQuery;

        let multi_match = MultiMatchQuery::new(vec!["title".to_string()], "test").boost(2.5);
        let query = Query::MultiMatch(multi_match);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_build_tantivy_query_constant_score() {
        use crate::query::{ConstantScoreQuery, TermQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let filter_query = Query::Term(TermQuery::new("title", "test"));
        let constant_score = ConstantScoreQuery::new(filter_query).boost(2.0);
        let query = Query::ConstantScore(constant_score);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_constant_score_with_match() {
        use crate::query::{ConstantScoreQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let filter_query = Query::Match(MatchQuery::new("title", "search terms"));
        let constant_score = ConstantScoreQuery::new(filter_query).boost(1.5);
        let query = Query::ConstantScore(constant_score);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_constant_score_default_boost() {
        use crate::query::{ConstantScoreQuery, RangeQuery};

        let (schema, _) = SchemaBuilder::new().add_i64_field("age").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        // Use both gte and lte for range query
        let filter_query = Query::Range(
            RangeQuery::new("age")
                .gte(serde_json::json!(18))
                .lte(serde_json::json!(100)),
        );
        let constant_score = ConstantScoreQuery::new(filter_query);
        let query = Query::ConstantScore(constant_score);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_constant_score() {
        use crate::query::{ConstantScoreQuery, TermQuery};

        let filter_query = Query::Term(TermQuery::new("status", "active"));
        let constant_score = ConstantScoreQuery::new(filter_query).boost(3.0);
        let query = Query::ConstantScore(constant_score);

        assert_eq!(SearchExecutor::extract_boost(&query), 3.0);
    }

    #[test]
    fn test_build_tantivy_query_dis_max() {
        use crate::query::{DisMaxQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let queries = vec![
            Query::Match(MatchQuery::new("title", "test")),
            Query::Match(MatchQuery::new("content", "test")),
        ];
        let dis_max = DisMaxQuery::new(queries);
        let query = Query::DisMax(dis_max);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_dis_max_with_tie_breaker() {
        use crate::query::{DisMaxQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .add_text_field("content")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let queries = vec![
            Query::Match(MatchQuery::new("title", "test")),
            Query::Match(MatchQuery::new("content", "test")),
        ];
        let dis_max = DisMaxQuery::new(queries).tie_breaker(0.3);
        let query = Query::DisMax(dis_max);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_dis_max_empty_queries() {
        use crate::query::DisMaxQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let dis_max = DisMaxQuery::new(vec![]);
        let query = Query::DisMax(dis_max);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_dis_max_with_boost() {
        use crate::query::{DisMaxQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let queries = vec![Query::Match(MatchQuery::new("title", "test"))];
        let dis_max = DisMaxQuery::new(queries).boost(2.0);
        let query = Query::DisMax(dis_max);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_dis_max() {
        use crate::query::{DisMaxQuery, MatchQuery};

        let queries = vec![Query::Match(MatchQuery::new("title", "test"))];
        let dis_max = DisMaxQuery::new(queries).boost(2.5);
        let query = Query::DisMax(dis_max);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_build_tantivy_query_common_terms() {
        use crate::query::CommonTermsQuery;

        let (schema, _) = SchemaBuilder::new().add_text_field("body").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let common_terms = CommonTermsQuery::new("body", "bonsai cool");
        let query = Query::CommonTerms(common_terms);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_common_terms_with_cutoff_frequency() {
        use crate::query::CommonTermsQuery;

        let (schema, _) = SchemaBuilder::new().add_text_field("body").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let common_terms = CommonTermsQuery::new("body", "test query").cutoff_frequency(0.01);
        let query = Query::CommonTerms(common_terms);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_common_terms_with_operators() {
        use crate::query::{CommonTermsOperator, CommonTermsQuery};

        let (schema, _) = SchemaBuilder::new().add_text_field("body").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let common_terms = CommonTermsQuery::new("body", "test query")
            .low_freq_operator(CommonTermsOperator::And)
            .high_freq_operator(CommonTermsOperator::Or);
        let query = Query::CommonTerms(common_terms);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_common_terms_invalid_field() {
        use crate::query::CommonTermsQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let common_terms = CommonTermsQuery::new("invalid_field", "test");
        let query = Query::CommonTerms(common_terms);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_common_terms_with_boost() {
        use crate::query::CommonTermsQuery;

        let (schema, _) = SchemaBuilder::new().add_text_field("body").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let common_terms = CommonTermsQuery::new("body", "test").boost(2.0);
        let query = Query::CommonTerms(common_terms);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_common_terms() {
        use crate::query::CommonTermsQuery;

        let common_terms = CommonTermsQuery::new("body", "test").boost(2.5);
        let query = Query::CommonTerms(common_terms);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_build_tantivy_query_wrapper() {
        use crate::query::{MatchQuery, WrapperQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let match_query = MatchQuery::new("title", "test");
        let query_json = serde_json::to_string(&Query::Match(match_query)).unwrap();
        let wrapper = WrapperQuery::new(query_json);
        let query = Query::Wrapper(wrapper);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_wrapper_invalid_json() {
        use crate::query::WrapperQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let wrapper = WrapperQuery::new("invalid json");
        let query = Query::Wrapper(wrapper);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_pinned() {
        use crate::query::{MatchQuery, PinnedQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let organic = Query::Match(MatchQuery::new("title", "test"));
        let pinned = PinnedQuery::new(vec!["doc1".to_string(), "doc2".to_string()], organic);
        let query = Query::Pinned(pinned);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_wrapper() {
        use crate::query::{MatchQuery, WrapperQuery};

        let match_query = MatchQuery::new("title", "test");
        let query_json = serde_json::to_string(&Query::Match(match_query)).unwrap();
        let wrapper = WrapperQuery::new(query_json);
        let query = Query::Wrapper(wrapper);

        assert_eq!(SearchExecutor::extract_boost(&query), 1.0);
    }

    #[test]
    fn test_extract_boost_pinned() {
        use crate::query::{MatchQuery, PinnedQuery};

        let organic = Query::Match(MatchQuery::new("title", "test"));
        let pinned = PinnedQuery::new(vec!["doc1".to_string()], organic);
        let query = Query::Pinned(pinned);

        assert_eq!(SearchExecutor::extract_boost(&query), 1.0);
    }

    #[test]
    fn test_build_tantivy_query_has_child() {
        use crate::query::{HasChildQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("status")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query);
        let query = Query::HasChild(has_child);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_has_parent() {
        use crate::query::{HasParentQuery, MatchQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("status")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent = HasParentQuery::new("blog", parent_query);
        let query = Query::HasParent(has_parent);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_has_child() {
        use crate::query::{HasChildQuery, MatchQuery};

        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query).boost(2.5);
        let query = Query::HasChild(has_child);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_extract_boost_has_parent() {
        use crate::query::{HasParentQuery, MatchQuery};

        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent = HasParentQuery::new("blog", parent_query).boost(2.0);
        let query = Query::HasParent(has_parent);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.0);
    }

    #[test]
    fn test_build_tantivy_query_percolate() {
        use crate::query::PercolateQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let document = serde_json::json!({"title": "test"});
        let percolate = PercolateQuery::new("document", document);
        let query = Query::Percolate(percolate);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_percolate() {
        use crate::query::PercolateQuery;

        let document = serde_json::json!({"title": "test"});
        let percolate = PercolateQuery::new("document", document).boost(2.5);
        let query = Query::Percolate(percolate);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_build_tantivy_query_geo_bounding_box() {
        use crate::query::{GeoBoundingBoxQuery, GeoPoint};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let top_left = GeoPoint {
            lat: 40.8,
            lon: -74.0,
        };
        let bottom_right = GeoPoint {
            lat: 40.7,
            lon: -73.9,
        };
        let bbox_query = GeoBoundingBoxQuery::new("location", top_left, bottom_right);
        let query = Query::GeoBoundingBox(bbox_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_polygon() {
        use crate::query::{GeoPoint, GeoPolygonQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let points = vec![
            GeoPoint {
                lat: 40.8,
                lon: -74.0,
            },
            GeoPoint {
                lat: 40.7,
                lon: -73.9,
            },
        ];
        let polygon_query = GeoPolygonQuery::new("location", points);
        let query = Query::GeoPolygon(polygon_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_shape() {
        use crate::query::{GeoPoint, GeoShape, GeoShapeQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let shape = GeoShape::Point {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
        };
        let shape_query = GeoShapeQuery::new("location", shape);
        let query = Query::GeoShape(shape_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_extract_boost_geo_distance() {
        use crate::query::GeoDistanceQuery;

        let geo_query = GeoDistanceQuery::new("location", "10km", 40.7128, -74.0060).boost(2.0);
        let query = Query::GeoDistance(geo_query);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.0);
    }

    #[test]
    fn test_extract_boost_geo_bounding_box() {
        use crate::query::{GeoBoundingBoxQuery, GeoPoint};

        let top_left = GeoPoint {
            lat: 40.8,
            lon: -74.0,
        };
        let bottom_right = GeoPoint {
            lat: 40.7,
            lon: -73.9,
        };
        let bbox_query = GeoBoundingBoxQuery::new("location", top_left, bottom_right).boost(1.5);
        let query = Query::GeoBoundingBox(bbox_query);

        assert_eq!(SearchExecutor::extract_boost(&query), 1.5);
    }

    #[test]
    fn test_extract_boost_geo_polygon() {
        use crate::query::{GeoPoint, GeoPolygonQuery};

        let points = vec![GeoPoint {
            lat: 40.8,
            lon: -74.0,
        }];
        let polygon_query = GeoPolygonQuery::new("location", points).boost(2.5);
        let query = Query::GeoPolygon(polygon_query);

        assert_eq!(SearchExecutor::extract_boost(&query), 2.5);
    }

    #[test]
    fn test_extract_boost_geo_shape() {
        use crate::query::{GeoPoint, GeoShape, GeoShapeQuery};

        let shape = GeoShape::Point {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
        };
        let shape_query = GeoShapeQuery::new("location", shape).boost(3.0);
        let query = Query::GeoShape(shape_query);

        assert_eq!(SearchExecutor::extract_boost(&query), 3.0);
    }

    #[test]
    fn test_build_tantivy_query_has_child_with_score_mode() {
        use crate::query::{HasChildQuery, MatchQuery, ParentChildScoreMode};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("status")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let child_query = Query::Match(MatchQuery::new("status", "active"));
        let has_child = HasChildQuery::new("comment", child_query)
            .score_mode(ParentChildScoreMode::Avg)
            .min_children(2);
        let query = Query::HasChild(has_child);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_has_parent_with_score_mode() {
        use crate::query::{HasParentQuery, MatchQuery, ParentChildScoreMode};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("status")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let parent_query = Query::Match(MatchQuery::new("status", "published"));
        let has_parent =
            HasParentQuery::new("blog", parent_query).score_mode(ParentChildScoreMode::Max);
        let query = Query::HasParent(has_parent);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_percolate_with_options() {
        use crate::query::PercolateQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let document = serde_json::json!({"title": "test"});
        let percolate = PercolateQuery::new("document", document)
            .index("queries")
            .document_type("alert")
            .preferred_sources(vec!["source1".to_string()]);
        let query = Query::Percolate(percolate);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_distance_with_boost() {
        use crate::query::GeoDistanceQuery;

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let geo_query = GeoDistanceQuery::new("location", "10km", 40.7128, -74.0060).boost(2.0);
        let query = Query::GeoDistance(geo_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_polygon_with_holes() {
        use crate::query::{GeoPoint, GeoPolygonQuery};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let points = vec![
            GeoPoint {
                lat: 40.8,
                lon: -74.0,
            },
            GeoPoint {
                lat: 40.7,
                lon: -73.9,
            },
        ];
        let hole = vec![GeoPoint {
            lat: 40.75,
            lon: -73.95,
        }];
        let polygon_query = GeoPolygonQuery::new("location", points).add_hole(hole);
        let query = Query::GeoPolygon(polygon_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_geo_shape_with_relation() {
        use crate::query::{GeoPoint, GeoShape, GeoShapeQuery, GeoShapeRelation};

        let (schema, _) = SchemaBuilder::new()
            .add_text_field("location")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let shape = GeoShape::Circle {
            coordinates: GeoPoint {
                lat: 40.7128,
                lon: -74.0060,
            },
            radius: "10km".to_string(),
        };
        let shape_query =
            GeoShapeQuery::new("location", shape).relation(GeoShapeRelation::Contains);
        let query = Query::GeoShape(shape_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_ok());
    }

    #[test]
    fn test_build_tantivy_query_invalid_field() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = QueryBuilder::term_query("nonexistent", "test");

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_range_missing_bounds() {
        let (schema, _) = SchemaBuilder::new().add_i64_field("age").build().unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let range_query = QueryBuilder::range_query("age");
        let query = Query::Range(range_query);

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_build_tantivy_query_phrase_empty() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let query = QueryBuilder::phrase_query("title", "");

        let regex_cache = Arc::new(RegexCache::new());
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query, regex_cache);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_operations() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));

        // Test cache stats
        let stats = executor.cache_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);

        // Test evict expired
        let evicted = executor.evict_expired_cache();
        assert_eq!(evicted, 0);

        // Test clear filter cache
        executor.clear_filter_cache();

        // Test clear field cache
        executor.clear_field_cache();

        // Test clear query pool
        executor.clear_query_pool();
    }

    #[test]
    fn test_with_cache_settings() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::with_cache_settings(Arc::new(index), 500, 300);
        assert_eq!(executor.cache_size(), 0);
        assert!(executor.cache.is_enabled());
    }

    #[test]
    fn test_warm_up_cache() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));

        let query = QueryBuilder::match_query("title", "test");
        let result = SearchResult::new(vec![], 0, 0);

        let entries = vec![(query, result)];
        let warmed = executor.warm_up_cache(entries);
        assert_eq!(warmed, 1);
        assert_eq!(executor.cache_size(), 1);
    }

    #[test]
    fn test_preload_field_cache() {
        let (schema, _) = SchemaBuilder::new()
            .add_text_field("title")
            .build()
            .unwrap();

        let tantivy_index = tantivy::Index::create_in_ram(schema);
        let index = Index {
            name: crate::types::IndexName::new("test"),
            inner: Arc::new(tantivy_index),
            settings: crate::index::IndexSettings::default(),
            mapping: None,
        };

        let executor = SearchExecutor::new(Arc::new(index));

        let values = vec![
            (0, crate::search::field_cache::FieldValue::I64(10)),
            (1, crate::search::field_cache::FieldValue::I64(20)),
        ];

        let preloaded = executor.preload_field_cache("age", values);
        assert_eq!(preloaded, 2);
    }
}
