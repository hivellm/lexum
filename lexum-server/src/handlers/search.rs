//! Search handler

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, State};
use lexum_core::{Query, SearchExecutor, SearchResult};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    /// Query
    pub query: Query,
    /// Limit (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    10
}

/// Search handler
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
        .search(request.query, request.limit, request.offset)
        .await?;

    Ok(Json(result))
}
