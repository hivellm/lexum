//! Search execution engine

use crate::error::{Error, Result};
use crate::index::Index;
use crate::query::Query;
use crate::search::field_cache::FieldCache;
use crate::search::filter_cache::FilterCache;
use crate::search::optimizer::QueryOptimizer;
use crate::search::result::{SearchHit, SearchResult, SortOption, SortOrder};
use crate::types::{DocumentId, Score};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;
use tantivy::TantivyDocument;
use tantivy::query::{
    AllQuery, BooleanQuery, FuzzyTermQuery, Occur, PhraseQuery, QueryParser, RangeQuery,
    RegexQuery as TantivyRegexQuery, TermQuery,
};
use tantivy::schema::*;

/// Cache key for query results
type CacheKey = String;

/// Search executor for running queries
pub struct SearchExecutor {
    index: Arc<Index>,
    /// Query cache (key: query hash, value: cached result)
    cache: Arc<DashMap<CacheKey, SearchResult>>,
    /// Whether caching is enabled
    cache_enabled: bool,
    /// Filter cache for bitset caching
    filter_cache: Arc<FilterCache>,
    /// Field cache for sorting and aggregations
    field_cache: Arc<FieldCache>,
}

impl SearchExecutor {
    /// Create new search executor with caching enabled
    pub fn new(index: Arc<Index>) -> Self {
        Self {
            index,
            cache: Arc::new(DashMap::new()),
            cache_enabled: true,
            filter_cache: Arc::new(FilterCache::new()),
            field_cache: Arc::new(FieldCache::new()),
        }
    }

    /// Create new search executor without caching
    pub fn without_cache(index: Arc<Index>) -> Self {
        Self {
            index,
            cache: Arc::new(DashMap::new()),
            cache_enabled: false,
            filter_cache: Arc::new(FilterCache::disabled()),
            field_cache: Arc::new(FieldCache::disabled()),
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

    /// Clear the query cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Get cache size
    pub fn cache_size(&self) -> usize {
        self.cache.len()
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
        if self.cache_enabled {
            let key = Self::cache_key(&optimized_query, limit, offset, &sort);
            if let Some(cached) = self.cache.get(&key) {
                tracing::debug!(cache_key = %key, "Cache hit");
                return Ok(cached.clone());
            }
        }

        let schema = self.index.schema();
        let index = self.index.clone();
        let query_clone = optimized_query.clone();
        let sort_clone = sort.clone();

        let result = tokio::task::spawn_blocking(move || {
            let reader = index.reader()?;
            let searcher = reader.searcher();

            // Convert our query to Tantivy query
            let tantivy_query = Self::build_tantivy_query(&index.inner, &query_clone)?;

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

            // Convert results
            let mut hits = Vec::new();
            for (score, doc_address) in top_docs.iter() {
                let doc: TantivyDocument = searcher
                    .doc(*doc_address)
                    .map_err(|e| Error::Config(format!("Failed to retrieve document: {e}")))?;

                let source = serde_json::from_str(&doc.to_json(&schema))
                    .map_err(|e| Error::Config(format!("Failed to parse document JSON: {e}")))?;

                hits.push(SearchHit {
                    id: DocumentId::new(format!("doc_{}", doc_address.segment_ord)),
                    score: Score::new(*score),
                    source,
                });
            }

            // Apply efficient in-memory sorting if requested
            if let Some(sort_opt) = sort_clone {
                if sort_opt.field != "_score" {
                    // Try to use field cache for faster sorting
                    let index_name = self.index.name().as_str().to_string();
                    let field_name = &sort_opt.field;

                    // Pre-populate field cache if enabled
                    // Note: We use a hash of the document ID string as the cache key
                    // since DocumentId doesn't expose a numeric value directly
                    if self.field_cache.is_enabled() {
                        for (idx, hit) in hits.iter().enumerate() {
                            // Use index as doc_id for cache (simple approach)
                            let doc_id = idx as u64;
                            // Check if value is already cached
                            if self.field_cache.get(index_name, field_name, doc_id).is_none() {
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
                                    self.field_cache.put(index_name, field_name, doc_id, field_value);
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
            let hits: Vec<SearchHit> = hits.into_iter().skip(offset).take(limit).collect();
            Ok::<SearchResult, Error>(SearchResult::new(hits, total, 0))
        })
        .await
        .map_err(|e| Error::Config(format!("Task join error: {e}")))?;

        let mut result = result?;
        result.took_ms = start.elapsed().as_millis() as u64;

        // Store in cache if enabled
        if self.cache_enabled {
            let key = Self::cache_key(&optimized_query, limit, offset, &sort);
            self.cache.insert(key.clone(), result.clone());
            tracing::debug!(cache_key = %key, cache_size = self.cache.len(), "Cached result");
        }

        Ok(result)
    }

    /// Build Tantivy query from our Query type
    fn build_tantivy_query(
        tantivy_index: &tantivy::Index,
        query: &Query,
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
                    let sub_query = Self::build_tantivy_query(tantivy_index, must)?;
                    clauses.push((Occur::Must, sub_query));
                }

                // Add should clauses
                for should in &bool_query.should {
                    let sub_query = Self::build_tantivy_query(tantivy_index, should)?;
                    clauses.push((Occur::Should, sub_query));
                }

                // Add must_not clauses
                for must_not in &bool_query.must_not {
                    let sub_query = Self::build_tantivy_query(tantivy_index, must_not)?;
                    clauses.push((Occur::MustNot, sub_query));
                }

                // Filter clauses (treat as must for now)
                for filter in &bool_query.filter {
                    let sub_query = Self::build_tantivy_query(tantivy_index, filter)?;
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

                // Use Tantivy's regex query
                let pattern = if regex_query.case_sensitive {
                    regex_query.pattern.clone()
                } else {
                    format!("(?i){}", regex_query.pattern)
                };

                TantivyRegexQuery::from_pattern(&pattern, field)
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
                Self::build_tantivy_query(tantivy_index, nested_query.query.as_ref())
            }

            Query::FunctionScore(func_score_query) => {
                // For now, execute the base query without function scoring
                // In a full implementation, this would apply custom scoring functions
                Self::build_tantivy_query(tantivy_index, func_score_query.query.as_ref())
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

    #[tokio::test]
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
        let result = SearchExecutor::build_tantivy_query(&tantivy_index, &query);
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
        };

        let executor = SearchExecutor::new(Arc::new(index));
        assert_eq!(executor.cache_size(), 0);
        assert!(executor.cache_enabled);
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
        };

        let executor = SearchExecutor::without_cache(Arc::new(index));
        assert_eq!(executor.cache_size(), 0);
        assert!(!executor.cache_enabled);
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
