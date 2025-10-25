//! Search handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::{Query, SearchExecutor, SearchResult, SortOption};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Search request
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchRequest {
    /// Query
    pub query: Query,
    /// Limit (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset (default: 0)
    #[serde(default)]
    pub offset: usize,
    /// Optional sort specification
    #[serde(default)]
    pub sort: Option<SortOption>,
}

fn default_limit() -> usize {
    10
}

/// Search handler
#[utoipa::path(
    post,
    path = "/api/v1/indices/{index_name}/search",
    params(
        ("index_name" = String, Path, description = "Index name")
    ),
    request_body = SearchRequest,
    responses(
        (status = 200, description = "Search completed successfully", body = SearchResult),
        (status = 404, description = "Index not found"),
        (status = 400, description = "Invalid request")
    ),
    tag = "Search"
)]
pub async fn search(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Json(request): Json<SearchRequest>,
) -> ApiResult<Json<SearchResult>> {
    let index = state
        .index_manager
        .get_index(&index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

    let executor = SearchExecutor::new(Arc::new(index));
    let result = executor
        .search(request.query, request.limit, request.offset, request.sort)
        .await?;

    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexum_core::{MatchQuery, Query, SortOrder};

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 10);
    }

    #[test]
    fn test_search_request_serialization() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query: query.clone(),
            limit: 20,
            offset: 5,
            sort: Some(SortOption::new("title", SortOrder::Asc)),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.limit, deserialized.limit);
        assert_eq!(request.offset, deserialized.offset);
        assert!(request.sort.is_some());
        assert!(deserialized.sort.is_some());
    }

    #[test]
    fn test_search_request_defaults() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query,
            limit: 10,  // default
            offset: 0,  // default
            sort: None, // default
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: SearchRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.limit, 10);
        assert_eq!(deserialized.offset, 0);
        assert!(deserialized.sort.is_none());
    }

    #[test]
    fn test_search_request_with_custom_limits() {
        let query = Query::Match(MatchQuery::new(
            "title".to_string(),
            "test query".to_string(),
        ));
        let request = SearchRequest {
            query,
            limit: 50,
            offset: 100,
            sort: None,
        };

        assert_eq!(request.limit, 50);
        assert_eq!(request.offset, 100);
    }
}
