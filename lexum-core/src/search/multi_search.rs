//! Multi-Search (msearch) - Batch search requests

use crate::error::Result;
use crate::index::IndexManager;
use crate::query::Query;
use crate::search::executor::SearchExecutor;
use crate::search::result::{SearchResult, SortOption};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use utoipa::ToSchema;

/// Multi-Search request item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiSearchItem {
    /// Index name (optional, can be specified in header or item)
    #[serde(rename = "_index", skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,
    /// Query to execute
    pub query: Query,
    /// Limit (size)
    #[serde(default = "default_limit")]
    pub size: usize,
    /// Offset (from)
    #[serde(default)]
    pub from: usize,
    /// Sort options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortOption>,
    /// Source filtering
    #[serde(rename = "_source", skip_serializing_if = "Option::is_none")]
    pub source: Option<JsonValue>,
    /// Aggregations
    #[serde(rename = "aggs", skip_serializing_if = "Option::is_none")]
    pub aggregations: Option<JsonValue>,
}

fn default_limit() -> usize {
    10
}

/// Multi-Search request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiSearchRequest {
    /// List of search requests
    pub searches: Vec<MultiSearchItem>,
}

/// Multi-Search response item
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiSearchResponseItem {
    /// Search result
    pub result: SearchResult,
    /// Error (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<MultiSearchError>,
}

/// Multi-Search error
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiSearchError {
    /// Error type
    #[serde(rename = "type")]
    pub error_type: String,
    /// Error reason
    pub reason: String,
}

/// Multi-Search response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MultiSearchResponse {
    /// Response items
    pub responses: Vec<MultiSearchResponseItem>,
    /// Total time taken in milliseconds
    pub took_ms: u64,
}

/// Multi-Search executor
pub struct MultiSearchExecutor {
    index_manager: Arc<IndexManager>,
}

impl MultiSearchExecutor {
    /// Create new Multi-Search executor
    pub fn new(index_manager: Arc<IndexManager>) -> Self {
        Self { index_manager }
    }

    /// Execute multiple search requests
    pub async fn search(&self, request: MultiSearchRequest) -> Result<MultiSearchResponse> {
        let start_time = std::time::Instant::now();
        let mut responses = Vec::new();

        for search_item in request.searches {
            match self.execute_search_item(&search_item).await {
                Ok(result) => {
                    responses.push(MultiSearchResponseItem {
                        result,
                        error: None,
                    });
                }
                Err(e) => {
                    responses.push(MultiSearchResponseItem {
                        result: SearchResult::new(Vec::new(), 0, 0),
                        error: Some(MultiSearchError {
                            error_type: "search_exception".to_string(),
                            reason: e.to_string(),
                        }),
                    });
                }
            }
        }

        Ok(MultiSearchResponse {
            responses,
            took_ms: start_time.elapsed().as_millis() as u64,
        })
    }

    /// Execute a single search item
    async fn execute_search_item(&self, item: &MultiSearchItem) -> Result<SearchResult> {
        // Resolve index name
        let index_name = item.index.as_deref().ok_or_else(|| {
            crate::error::Error::Config("Index name is required for multi-search".to_string())
        })?;

        // Get index
        let index = self.index_manager.get_index(index_name)?;
        let executor = SearchExecutor::new(Arc::new(index));

        // Execute search
        executor
            .search(item.query.clone(), item.size, item.from, item.sort.clone())
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::MatchQuery;

    #[test]
    fn test_multi_search_item_serialization() {
        let item = MultiSearchItem {
            index: Some("test_index".to_string()),
            query: Query::Match(MatchQuery::new("field", "value")),
            size: 10,
            from: 0,
            sort: None,
            source: None,
            aggregations: None,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("query"));
        assert!(json.contains("size"));
    }

    #[test]
    fn test_multi_search_request_serialization() {
        let request = MultiSearchRequest {
            searches: vec![
                MultiSearchItem {
                    index: Some("index1".to_string()),
                    query: Query::Match(MatchQuery::new("field1", "value1")),
                    size: 10,
                    from: 0,
                    sort: None,
                    source: None,
                    aggregations: None,
                },
                MultiSearchItem {
                    index: Some("index2".to_string()),
                    query: Query::Match(MatchQuery::new("field2", "value2")),
                    size: 20,
                    from: 0,
                    sort: None,
                    source: None,
                    aggregations: None,
                },
            ],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("searches"));
    }
}
