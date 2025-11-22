//! Point in Time (PIT) API handler for consistent reads across multiple searches

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use lexum_core::search::{PitId, PointInTimeExecutor, PointInTimeManager, parse_duration};
use lexum_core::{Query as CoreQuery, SearchResult};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::Duration;

/// Create Point in Time request parameters
#[derive(Debug, Deserialize)]
pub struct CreatePitParams {
    /// Keep-alive duration (e.g., "5m", "1h")
    #[serde(default = "default_keep_alive")]
    pub keep_alive: String,
}

fn default_keep_alive() -> String {
    "5m".to_string()
}

/// Create Point in Time response
#[derive(Debug, Serialize)]
pub struct CreatePitResponse {
    /// Point in Time ID
    pub id: PitId,
    /// Creation timestamp (Unix epoch in seconds)
    pub creation_time: u64,
}

/// Create Point in Time handler
/// POST /api/v1/indices/{index}/_pit
pub async fn create_pit(
    State(state): State<AppState>,
    Path(index_name): Path<String>,
    Query(params): Query<CreatePitParams>,
) -> ApiResult<Json<CreatePitResponse>> {
    // Get index
    let index = state.index_manager.get_index(&index_name).map_err(|e| {
        // Convert Validation error for "not found" to IndexNotFound
        if let lexum_core::Error::Validation(ref msg) = e {
            if msg.contains("not found") || msg.contains("does not exist") {
                return ApiError::IndexNotFound(index_name.clone());
            }
        }
        let error_msg = e.to_string();
        if error_msg.contains("not found") || error_msg.contains("does not exist") {
            ApiError::IndexNotFound(index_name.clone())
        } else {
            tracing::error!("Failed to get index '{}': {}", index_name, error_msg);
            ApiError::Core(e)
        }
    })?;

    // Parse keep-alive duration
    let keep_alive = parse_duration(&params.keep_alive)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid keep_alive: {e}")))?;

    // Create PIT
    let manager = get_pit_manager();
    let pit_id = manager
        .create_pit(Arc::new(index), Some(keep_alive))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to create PIT: {e}")))?;

    Ok(Json(CreatePitResponse {
        id: pit_id,
        creation_time: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    }))
}

/// Delete Point in Time handler
/// DELETE /api/v1/_pit/{pit_id}
pub async fn delete_pit(Path(pit_id): Path<String>) -> ApiResult<Json<Value>> {
    let manager = get_pit_manager();
    let deleted = manager.delete_pit(&pit_id).await;

    if deleted {
        Ok(Json(json!({ "acknowledged": true })))
    } else {
        Err(ApiError::InvalidRequest(format!(
            "Point in Time not found: {pit_id}"
        )))
    }
}

/// Extend Point in Time keep-alive handler
/// POST /api/v1/_pit/{pit_id}
pub async fn extend_pit(
    Path(pit_id): Path<String>,
    request: Result<Json<Value>, axum::extract::rejection::JsonRejection>,
) -> ApiResult<Json<Value>> {
    // Convert JsonRejection to ApiError if JSON parsing failed
    let Json(request) = request.map_err(ApiError::from)?;
    let manager = get_pit_manager();

    // Parse keep_alive from request
    let keep_alive_str = request
        .get("keep_alive")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("keep_alive parameter required".to_string()))?;

    let keep_alive = parse_duration(keep_alive_str)
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid keep_alive: {e}")))?;

    manager
        .extend_keep_alive(&pit_id, keep_alive)
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to extend PIT: {e}")))?;

    Ok(Json(json!({ "acknowledged": true })))
}

/// Search with Point in Time handler
/// This is integrated into the main search handler
/// The search handler should check for pit_id in the request
/// Global PIT manager
static PIT_MANAGER: std::sync::OnceLock<Arc<PointInTimeManager>> = std::sync::OnceLock::new();

fn get_pit_manager() -> &'static Arc<PointInTimeManager> {
    PIT_MANAGER.get_or_init(|| {
        Arc::new(PointInTimeManager::new(Duration::from_secs(300))) // Default 5 minutes
    })
}

/// Search request with Point in Time support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWithPitRequest {
    /// Point in Time ID
    #[serde(rename = "pit")]
    pub pit_id: Option<PitId>,
    /// Query (optional if q is provided)
    #[serde(default)]
    pub query: Option<CoreQuery>,
    /// Filter queries (must match but don't affect score)
    #[serde(default)]
    pub filter: Option<Vec<CoreQuery>>,
    /// Limit (default: 10)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset (default: 0)
    #[serde(default)]
    pub offset: usize,
    /// Optional sort specification
    #[serde(default)]
    pub sort: Option<lexum_core::search::SortOption>,
    /// Keep-alive duration for PIT (optional, extends existing PIT)
    #[serde(rename = "keep_alive", skip_serializing_if = "Option::is_none")]
    pub keep_alive: Option<String>,
}

fn default_limit() -> usize {
    10
}

/// Execute search with Point in Time
pub async fn search_with_pit(
    State(_state): State<AppState>,
    Json(request): Json<SearchWithPitRequest>,
) -> ApiResult<Json<SearchResult>> {
    let pit_id = request
        .pit_id
        .as_ref()
        .ok_or_else(|| ApiError::InvalidRequest("pit_id is required".to_string()))?;

    // Extend keep-alive if requested
    if let Some(ref keep_alive_str) = request.keep_alive {
        let keep_alive = parse_duration(keep_alive_str)
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid keep_alive: {e}")))?;
        let manager = get_pit_manager();
        manager
            .extend_keep_alive(pit_id, keep_alive)
            .await
            .map_err(|e| ApiError::InvalidRequest(format!("Failed to extend PIT: {e}")))?;
    }

    // Build query
    let base_query = request.query.unwrap_or(CoreQuery::MatchAll);

    // Apply filters if provided
    let final_query = if let Some(ref filters) = request.filter {
        if !filters.is_empty() {
            let mut bool_query = lexum_core::BoolQuery::new();
            bool_query = bool_query.must(base_query);
            for filter in filters {
                bool_query = bool_query.filter(filter.clone());
            }
            CoreQuery::Bool(bool_query)
        } else {
            base_query
        }
    } else {
        base_query
    };

    // Execute search with PIT
    let manager = get_pit_manager();
    let executor = PointInTimeExecutor::new(Arc::clone(manager));
    let result = executor
        .search_with_pit(
            pit_id,
            final_query,
            request.limit,
            request.offset,
            request.sort,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Search with PIT failed: {e}")))?;

    Ok(Json(result))
}
