//! Progress tracking handlers

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::extract::{Path, Query, State};
use axum::Json;
use axum::http::StatusCode;
use lexum_core::progress::{
    ProgressFilter, ProgressId, ProgressInfo, ProgressStatus, OperationType, ProgressStats,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Get progress information for a specific operation
#[utoipa::path(
    get,
    path = "/api/v1/progress/{progress_id}",
    responses(
        (status = 200, description = "Progress information retrieved", body = ProgressInfo),
        (status = 404, description = "Progress not found")
    ),
    tag = "Progress"
)]
pub async fn get_progress(
    State(state): State<AppState>,
    Path(progress_id): Path<String>,
) -> ApiResult<Json<ProgressInfo>> {
    let progress_id = ProgressId::from(progress_id);
    
    match state.progress_tracker.get_progress(&progress_id).await? {
        Some(progress) => Ok(Json(progress)),
        None => Err(ApiError::IndexNotFound("Progress not found".to_string())),
    }
}

/// List progress sessions with optional filtering
#[utoipa::path(
    get,
    path = "/api/v1/progress",
    responses(
        (status = 200, description = "Progress sessions listed", body = Vec<ProgressInfo>)
    ),
    tag = "Progress"
)]
pub async fn list_progress(
    State(state): State<AppState>,
    Query(params): Query<ProgressListParams>,
) -> ApiResult<Json<Vec<ProgressInfo>>> {
    let filter = ProgressFilter {
        operation_type: params.operation_type,
        status: params.status,
        start_time_after: params.start_time_after,
        start_time_before: params.start_time_before,
        limit: params.limit,
        offset: params.offset,
    };

    let progress_sessions = state.progress_tracker.list_progress(Some(filter)).await?;
    Ok(Json(progress_sessions))
}

/// Get progress statistics
#[utoipa::path(
    get,
    path = "/api/v1/progress/stats",
    responses(
        (status = 200, description = "Progress statistics retrieved", body = ProgressStats)
    ),
    tag = "Progress"
)]
pub async fn get_progress_stats(
    State(state): State<AppState>,
) -> ApiResult<Json<ProgressStats>> {
    let stats = state.progress_tracker.get_stats().await?;
    Ok(Json(stats))
}

/// Cancel a progress operation
#[utoipa::path(
    post,
    path = "/api/v1/progress/{progress_id}/cancel",
    responses(
        (status = 200, description = "Operation cancelled successfully"),
        (status = 404, description = "Progress not found")
    ),
    tag = "Progress"
)]
pub async fn cancel_progress(
    State(state): State<AppState>,
    Path(progress_id): Path<String>,
) -> ApiResult<StatusCode> {
    let progress_id = ProgressId::from(progress_id);
    
    state.progress_tracker.cancel_operation(&progress_id).await?;
    Ok(StatusCode::OK)
}

/// Pause a progress operation
#[utoipa::path(
    post,
    path = "/api/v1/progress/{progress_id}/pause",
    responses(
        (status = 200, description = "Operation paused successfully"),
        (status = 404, description = "Progress not found")
    ),
    tag = "Progress"
)]
pub async fn pause_progress(
    State(state): State<AppState>,
    Path(progress_id): Path<String>,
) -> ApiResult<StatusCode> {
    let progress_id = ProgressId::from(progress_id);
    
    state.progress_tracker.pause_operation(&progress_id).await?;
    Ok(StatusCode::OK)
}

/// Resume a paused progress operation
#[utoipa::path(
    post,
    path = "/api/v1/progress/{progress_id}/resume",
    responses(
        (status = 200, description = "Operation resumed successfully"),
        (status = 404, description = "Progress not found")
    ),
    tag = "Progress"
)]
pub async fn resume_progress(
    State(state): State<AppState>,
    Path(progress_id): Path<String>,
) -> ApiResult<StatusCode> {
    let progress_id = ProgressId::from(progress_id);
    
    state.progress_tracker.resume_operation(&progress_id).await?;
    Ok(StatusCode::OK)
}

/// Delete a progress session
#[utoipa::path(
    delete,
    path = "/api/v1/progress/{progress_id}",
    responses(
        (status = 200, description = "Progress session deleted successfully"),
        (status = 404, description = "Progress not found")
    ),
    tag = "Progress"
)]
pub async fn delete_progress(
    State(state): State<AppState>,
    Path(progress_id): Path<String>,
) -> ApiResult<StatusCode> {
    let progress_id = ProgressId::from(progress_id);
    
    state.progress_tracker.delete_progress(&progress_id).await?;
    Ok(StatusCode::OK)
}

/// Clean up old progress sessions
#[utoipa::path(
    post,
    path = "/api/v1/progress/cleanup",
    responses(
        (status = 200, description = "Cleanup completed", body = CleanupResponse)
    ),
    tag = "Progress"
)]
pub async fn cleanup_progress(
    State(state): State<AppState>,
    Query(params): Query<CleanupParams>,
) -> ApiResult<Json<CleanupResponse>> {
    let cleaned_count = state
        .progress_tracker
        .cleanup_old_sessions(params.max_age_hours.unwrap_or(24))
        .await?;

    Ok(Json(CleanupResponse {
        cleaned_sessions: cleaned_count,
    }))
}

/// Parameters for listing progress sessions
#[derive(Debug, Deserialize, ToSchema)]
pub struct ProgressListParams {
    /// Filter by operation type
    pub operation_type: Option<OperationType>,
    /// Filter by status
    pub status: Option<ProgressStatus>,
    /// Filter by start time (after)
    pub start_time_after: Option<chrono::DateTime<chrono::Utc>>,
    /// Filter by start time (before)
    pub start_time_before: Option<chrono::DateTime<chrono::Utc>>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Parameters for cleanup operation
#[derive(Debug, Deserialize, ToSchema)]
pub struct CleanupParams {
    /// Maximum age in hours for sessions to keep
    pub max_age_hours: Option<u64>,
}

/// Response for cleanup operation
#[derive(Debug, Serialize, ToSchema)]
pub struct CleanupResponse {
    /// Number of sessions cleaned up
    pub cleaned_sessions: usize,
}