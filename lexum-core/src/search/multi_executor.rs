//! Multi-index search execution engine

use crate::error::Result;
use crate::index::IndexManager;
use crate::query::Query;
use crate::search::result::{SearchResult, SortOption, SortOrder};
use crate::types::IndexName;
use dashmap::DashMap;
use std::sync::Arc;
use std::time::Instant;

/// Cache key for query results
type CacheKey = String;

/// Multi-index search executor for running queries across multiple indices
pub struct MultiIndexSearchExecutor {
    index_manager: Arc<IndexManager>,
    /// Query cache (key: query hash, value: cached result)
    cache: Arc<DashMap<CacheKey, SearchResult>>,
    /// Whether caching is enabled
    cache_enabled: bool,
}

impl MultiIndexSearchExecutor {
    /// Create new multi-index search executor with caching enabled
    pub fn new(index_manager: Arc<IndexManager>) -> Self {
        Self {
            index_manager,
            cache: Arc::new(DashMap::new()),
            cache_enabled: true,
        }
    }

    /// Create new multi-index search executor without caching
    pub fn without_cache(index_manager: Arc<IndexManager>) -> Self {
        Self {
            index_manager,
            cache: Arc::new(DashMap::new()),
            cache_enabled: false,
        }
    }

    /// Clear the query cache
    pub fn clear_cache(&self) {
        self.cache.clear();
    }

    /// Search across multiple indices
    pub async fn search_multi(
        &self,
        indices: Vec<IndexName>,
        query: Query,
        limit: usize,
        offset: usize,
        sort: Option<SortOption>,
    ) -> Result<SearchResult> {
        let start_time = Instant::now();

        // Generate cache key if caching is enabled
        let cache_key = if self.cache_enabled {
            Self::generate_cache_key(&indices, &query, limit, offset, &sort)
        } else {
            String::new()
        };

        // Check cache first
        if self.cache_enabled && !cache_key.is_empty() {
            if let Some(cached_result) = self.cache.get(&cache_key) {
                tracing::debug!("Cache hit for multi-index search");
                return Ok(cached_result.clone());
            }
        }

        // Search each index and combine results
        let mut all_hits = Vec::new();
        let mut total_hits = 0;

        for index_name in &indices {
            let index = self.index_manager.get_index(index_name.as_str())?;
            let executor = crate::search::SearchExecutor::new(Arc::new(index));

            // Search with higher limit to get more results for proper sorting
            let result = executor.search(query.clone(), limit * 2, 0, None).await?;

            // Add index name to each hit for identification
            let mut hits_with_index = result.hits;
            for hit in &mut hits_with_index {
                // Store the source index name in the hit metadata
                if let serde_json::Value::Object(ref mut source) = hit.source {
                    source.insert(
                        "_index".to_string(),
                        serde_json::Value::String(index_name.as_str().to_string()),
                    );
                }
            }

            all_hits.extend(hits_with_index);
            total_hits += result.total;
        }

        // Sort all hits if sort option is provided
        if let Some(sort_opt) = &sort {
            all_hits.sort_by(|a, b| {
                match sort_opt.field.as_str() {
                    "_score" => {
                        // Sort by score (descending by default)
                        match sort_opt.order {
                            SortOrder::Asc => a
                                .score
                                .value()
                                .partial_cmp(&b.score.value())
                                .unwrap_or(std::cmp::Ordering::Equal),
                            SortOrder::Desc => b
                                .score
                                .value()
                                .partial_cmp(&a.score.value())
                                .unwrap_or(std::cmp::Ordering::Equal),
                        }
                    }
                    field => {
                        // Sort by field value
                        let a_val = Self::extract_field_value(&a.source, field);
                        let b_val = Self::extract_field_value(&b.source, field);

                        let comparison = match (&a_val, &b_val) {
                            (Some(a), Some(b)) => a.cmp(b),
                            (Some(_), None) => std::cmp::Ordering::Greater,
                            (None, Some(_)) => std::cmp::Ordering::Less,
                            (None, None) => std::cmp::Ordering::Equal,
                        };

                        match sort_opt.order {
                            SortOrder::Asc => comparison,
                            SortOrder::Desc => comparison.reverse(),
                        }
                    }
                }
            });
        } else {
            // Default sort by score (descending)
            all_hits.sort_by(|a, b| {
                b.score
                    .value()
                    .partial_cmp(&a.score.value())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Apply offset and limit
        let start = offset.min(all_hits.len());
        let end = (start + limit).min(all_hits.len());
        let hits = all_hits[start..end].to_vec();

        let result = SearchResult {
            hits,
            total: total_hits,
            took_ms: start_time.elapsed().as_millis() as u64,
        };

        // Cache the result if caching is enabled
        if self.cache_enabled && !cache_key.is_empty() {
            self.cache.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    /// Generate cache key for the query
    fn generate_cache_key(
        indices: &[IndexName],
        query: &Query,
        limit: usize,
        offset: usize,
        sort: &Option<SortOption>,
    ) -> String {
        let indices_str = indices
            .iter()
            .map(|i| i.as_str())
            .collect::<Vec<_>>()
            .join(",");

        let sort_str = sort
            .as_ref()
            .map(|s| format!("{}:{}", s.field, s.order))
            .unwrap_or_default();

        format!(
            "multi:{}:{}:{}:{}:{}",
            indices_str,
            serde_json::to_string(query).unwrap_or_default(),
            limit,
            offset,
            sort_str
        )
    }

    /// Extract field value from document source for sorting
    fn extract_field_value(source: &serde_json::Value, field: &str) -> Option<String> {
        if let serde_json::Value::Object(map) = source {
            if let Some(value) = map.get(field) {
                return Some(value.to_string().trim_matches('"').to_string());
            }
        }
        None
    }
}

impl Default for MultiIndexSearchExecutor {
    fn default() -> Self {
        Self::new(Arc::new(IndexManager::new("./data")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::IndexManager;
    use crate::query::MatchQuery;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_multi_index_search_executor_creation() {
        let index_manager = Arc::new(IndexManager::new("./test_data"));
        let executor = MultiIndexSearchExecutor::new(index_manager);
        assert!(executor.cache_enabled);
    }

    #[tokio::test]
    async fn test_multi_index_search_executor_without_cache() {
        let index_manager = Arc::new(IndexManager::new("./test_data"));
        let executor = MultiIndexSearchExecutor::without_cache(index_manager);
        assert!(!executor.cache_enabled);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let index_manager = Arc::new(IndexManager::new("./test_data"));
        let executor = MultiIndexSearchExecutor::new(index_manager);

        let indices = vec![IndexName::new("index1"), IndexName::new("index2")];
        let query = Query::Match(MatchQuery::new("field", "value"));
        let sort = Some(SortOption::new("_score", SortOrder::Desc));

        let cache_key =
            MultiIndexSearchExecutor::generate_cache_key(&indices, &query, 10, 0, &sort);
        assert!(!cache_key.is_empty());
        assert!(cache_key.contains("multi:"));
        assert!(cache_key.contains("index1,index2"));
    }
}
