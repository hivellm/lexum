//! Enhanced bulk operations with progress tracking

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::{Json, extract::State};
use lexum_core::{
    document::ProgressDocumentStore,
    document::store::{BulkOperation, BulkOperationResult},
    progress::{OperationType, ProgressId},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use utoipa::ToSchema;

/// Enhanced bulk request with progress tracking
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProgressBulkRequest {
    /// List of bulk operations
    pub operations: Vec<BulkOperation>,
    /// Whether to track progress (default: true)
    #[serde(default = "default_true")]
    pub track_progress: bool,
    /// Custom progress description
    pub progress_description: Option<String>,
}

/// Enhanced bulk response with progress information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProgressBulkResponse {
    /// Whether all operations succeeded
    pub errors: bool,
    /// Number of operations
    pub took_ms: u64,
    /// Results for each operation
    pub items: Vec<BulkOperationResult>,
    /// Progress ID for tracking (if enabled)
    pub progress_id: Option<String>,
    /// Progress statistics
    pub progress_stats: Option<ProgressStats>,
}

/// Progress statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProgressStats {
    /// Total operations
    pub total: u64,
    /// Completed operations
    pub completed: u64,
    /// Failed operations
    pub failed: u64,
    /// Completion percentage
    pub percentage: f64,
    /// Processing rate (operations per second)
    pub rate: f64,
    /// Estimated time remaining (seconds)
    pub estimated_remaining: Option<u64>,
}

fn default_true() -> bool {
    true
}

/// Enhanced bulk operations with progress tracking
#[utoipa::path(
    post,
    path = "/api/v1/bulk/progress",
    request_body = ProgressBulkRequest,
    responses(
        (status = 200, description = "Bulk operations completed with progress tracking", body = ProgressBulkResponse),
        (status = 400, description = "Invalid request")
    ),
    tag = "Documents"
)]
pub async fn bulk_operations_with_progress(
    State(state): State<AppState>,
    Json(request): Json<ProgressBulkRequest>,
) -> ApiResult<Json<ProgressBulkResponse>> {
    let start = Instant::now();
    let total_operations = request.operations.len() as u64;

    // Group operations by index
    let mut operations_by_index: HashMap<String, Vec<BulkOperation>> = HashMap::new();
    for operation in request.operations {
        let index_name = match &operation {
            BulkOperation::Index { index, .. } => index.clone(),
            BulkOperation::Update { index, .. } => index.clone(),
            BulkOperation::Delete { index, .. } => index.clone(),
        };
        operations_by_index
            .entry(index_name)
            .or_default()
            .push(operation);
    }

    let mut all_results = Vec::new();
    let mut has_errors = false;
    let mut progress_id: Option<ProgressId> = None;

    // Start progress tracking if enabled
    if request.track_progress {
        let description = request
            .progress_description
            .unwrap_or_else(|| format!("Bulk operations on {} indices", operations_by_index.len()));

        progress_id = Some(
            state
                .progress_tracker
                .start_operation(
                    OperationType::BulkOperation,
                    description,
                    total_operations,
                    Some({
                        let mut metadata = HashMap::new();
                        metadata.insert(
                            "index_count".to_string(),
                            serde_json::Value::Number(operations_by_index.len().into()),
                        );
                        metadata.insert(
                            "operation_count".to_string(),
                            serde_json::Value::Number(total_operations.into()),
                        );
                        metadata
                    }),
                )
                .await?,
        );

        state
            .progress_tracker
            .mark_running(progress_id.as_ref().unwrap())
            .await?;
    }

    // Process each index
    for (index_name, operations) in operations_by_index {
        match state.index_manager.get_index(&index_name) {
            Ok(index) => {
                let store =
                    ProgressDocumentStore::new(Arc::new(index), state.progress_tracker.clone());

                let index_results = store
                    .bulk_operations_with_progress(operations, progress_id.clone())
                    .await?;

                all_results.extend(index_results.items);
                if index_results.errors {
                    has_errors = true;
                }
            }
            Err(e) => {
                has_errors = true;
                // Create error results for all operations in this index
                for operation in operations {
                    let error_result = match operation {
                        BulkOperation::Index { id, .. } => BulkOperationResult::Index {
                            index: index_name.clone(),
                            id: id.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        },
                        BulkOperation::Update { id, .. } => BulkOperationResult::Update {
                            index: index_name.clone(),
                            id: id.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        },
                        BulkOperation::Delete { id, .. } => BulkOperationResult::Delete {
                            index: index_name.clone(),
                            id: id.clone(),
                            success: false,
                            error: Some(e.to_string()),
                        },
                    };
                    all_results.push(error_result);
                }
            }
        }
    }

    // Get final progress statistics
    let progress_stats = if let Some(ref pid) = progress_id {
        if let Ok(Some(progress)) = state.progress_tracker.get_progress(pid).await {
            Some(ProgressStats {
                total: progress.metrics.total,
                completed: progress.metrics.completed,
                failed: progress.metrics.failed,
                percentage: progress.metrics.percentage(),
                rate: progress.metrics.rate,
                estimated_remaining: progress.metrics.estimated_remaining,
            })
        } else {
            None
        }
    } else {
        None
    };

    let took_ms = start.elapsed().as_millis() as u64;

    Ok(Json(ProgressBulkResponse {
        errors: has_errors,
        took_ms,
        items: all_results,
        progress_id: progress_id.clone(),
        progress_stats,
    }))
}

/// Get progress for a bulk operation
#[utoipa::path(
    get,
    path = "/api/v1/bulk/progress/{progress_id}",
    responses(
        (status = 200, description = "Progress information retrieved"),
        (status = 404, description = "Progress not found")
    ),
    tag = "Documents"
)]
pub async fn get_bulk_progress(
    State(state): State<AppState>,
    axum::extract::Path(progress_id): axum::extract::Path<String>,
) -> ApiResult<Json<ProgressStats>> {
    let progress_id = ProgressId::from(progress_id);

    match state.progress_tracker.get_progress(&progress_id).await? {
        Some(progress) => {
            let stats = ProgressStats {
                total: progress.metrics.total,
                completed: progress.metrics.completed,
                failed: progress.metrics.failed,
                percentage: progress.metrics.percentage(),
                rate: progress.metrics.rate,
                estimated_remaining: progress.metrics.estimated_remaining,
            };
            Ok(Json(stats))
        }
        None => Err(ApiError::IndexNotFound("Progress not found".to_string())),
    }
}
