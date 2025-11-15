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
    AllQuery, BooleanQuery, FuzzyTermQuery, Occur, PhraseQuery, QueryParser, RangeQuery,
    RegexQuery as TantivyRegexQuery, TermQuery,
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
                // Geo queries are not yet implemented in Tantivy integration
                // Return a match all query for now
                Ok(Box::new(AllQuery))
            }

            Query::Script(_script_query) => {
                // Script queries are not yet implemented
                // Return a match all query for now
                Ok(Box::new(AllQuery))
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
}
