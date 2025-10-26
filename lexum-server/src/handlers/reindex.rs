//! Reindexing operations endpoints

use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::handlers::index::AppState;

/// Reindex request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexRequest {
    /// Source index configuration
    pub source: ReindexSource,
    /// Destination index configuration
    pub dest: ReindexDestination,
    /// Script for document transformation (optional)
    pub script: Option<ReindexScript>,
    /// Maximum number of documents to process
    pub max_docs: Option<u64>,
    /// Query to filter source documents
    pub query: Option<serde_json::Value>,
}

/// Reindex source configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexSource {
    /// Source index name
    pub index: String,
    /// Query to filter documents
    pub query: Option<serde_json::Value>,
    /// Fields to include
    pub source: Option<Vec<String>>,
    /// Fields to exclude
    pub source_excludes: Option<Vec<String>>,
}

/// Reindex destination configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexDestination {
    /// Destination index name
    pub index: String,
    /// Version type for conflict resolution
    pub version_type: Option<String>,
    /// Operation type (index, create)
    pub op_type: Option<String>,
}

/// Reindex script for document transformation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexScript {
    /// Script source
    pub source: String,
    /// Script language
    pub lang: Option<String>,
    /// Script parameters
    pub params: Option<serde_json::Value>,
}

/// Reindex response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexResponse {
    /// Task ID for tracking
    pub task: String,
    /// Whether the operation was acknowledged
    pub acknowledged: bool,
    /// Number of documents to be processed
    pub total: u64,
    /// Number of documents processed so far
    pub updated: u64,
    /// Number of documents created
    pub created: u64,
    /// Number of documents that failed
    pub failed: u64,
    /// Number of batches processed
    pub batches: u64,
    /// Version conflicts
    pub version_conflicts: u64,
    /// Number of retries
    pub retries: u64,
    /// Throttled until timestamp
    pub throttled_until_millis: Option<u64>,
    /// Throttled until in nanoseconds
    pub throttled_until_nanos: Option<u64>,
}

/// Task information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskInfo {
    /// Task ID
    pub task_id: String,
    /// Task type
    pub task_type: String,
    /// Task action
    pub action: String,
    /// Task description
    pub description: String,
    /// Task status
    pub status: TaskStatus,
    /// Start time
    pub start_time_in_millis: u64,
    /// Running time in nanoseconds
    pub running_time_in_nanos: u64,
    /// Parent task ID
    pub parent_task_id: Option<String>,
    /// Cancellable
    pub cancellable: bool,
    /// Headers
    pub headers: serde_json::Value,
}

/// Task status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskStatus {
    /// Total number of operations
    pub total: u64,
    /// Number of operations completed
    pub completed: u64,
    /// Number of operations failed
    pub failed: u64,
    /// Number of operations cancelled
    pub cancelled: u64,
    /// Number of operations created
    pub created: u64,
    /// Number of operations deleted
    pub deleted: u64,
    /// Number of operations noops
    pub noops: u64,
    /// Number of retries
    pub retries: u64,
    /// Throttled until timestamp
    pub throttled_until_millis: Option<u64>,
    /// Throttled until in nanoseconds
    pub throttled_until_nanos: Option<u64>,
}

/// Reindex operation
#[utoipa::path(
    post,
    path = "/_reindex",
    request_body = ReindexRequest,
    responses(
        (status = 200, description = "Reindex operation started successfully", body = ReindexResponse),
        (status = 400, description = "Invalid request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Reindexing"
)]
pub async fn reindex(
    State(_state): State<AppState>,
    Json(request): Json<ReindexRequest>,
) -> Result<Json<ReindexResponse>, StatusCode> {
    tracing::info!(
        "Starting reindex operation from '{}' to '{}'",
        request.source.index,
        request.dest.index
    );

    // For now, just return a mock response
    // In a real implementation, this would:
    // 1. Validate source and destination indices
    // 2. Start a background task for reindexing
    // 3. Return a task ID for tracking progress
    // 4. Process documents in batches
    // 5. Apply transformations if script is provided

    let task_id = format!("reindex_{}_{}", request.source.index, request.dest.index);

    Ok(Json(ReindexResponse {
        task: task_id,
        acknowledged: true,
        total: 0, // Would be calculated from source index
        updated: 0,
        created: 0,
        failed: 0,
        batches: 0,
        version_conflicts: 0,
        retries: 0,
        throttled_until_millis: None,
        throttled_until_nanos: None,
    }))
}

/// Get task information
#[utoipa::path(
    get,
    path = "/_tasks/{task_id}",
    responses(
        (status = 200, description = "Task information retrieved successfully", body = TaskInfo),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Tasks"
)]
pub async fn get_task(
    State(_state): State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Result<Json<TaskInfo>, StatusCode> {
    tracing::info!("Getting task information for task: {}", task_id);

    // For now, return a mock task info
    // In a real implementation, this would query the task manager
    Ok(Json(TaskInfo {
        task_id: task_id.clone(),
        task_type: "reindex".to_string(),
        action: "indices:data/write/reindex".to_string(),
        description: format!("reindex from [{}] to [{}]", "source_index", "dest_index"),
        status: TaskStatus {
            total: 1000,
            completed: 500,
            failed: 0,
            cancelled: 0,
            created: 500,
            deleted: 0,
            noops: 0,
            retries: 0,
            throttled_until_millis: None,
            throttled_until_nanos: None,
        },
        start_time_in_millis: chrono::Utc::now().timestamp_millis() as u64,
        running_time_in_nanos: 5000000000, // 5 seconds
        parent_task_id: None,
        cancellable: true,
        headers: serde_json::json!({}),
    }))
}

/// Cancel task
#[utoipa::path(
    post,
    path = "/_tasks/{task_id}/_cancel",
    responses(
        (status = 200, description = "Task cancelled successfully"),
        (status = 404, description = "Task not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Tasks"
)]
pub async fn cancel_task(
    State(_state): State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Cancelling task: {}", task_id);

    // For now, just return success
    // In a real implementation, this would cancel the running task
    Ok(Json(serde_json::json!({
        "acknowledged": true
    })))
}

/// List tasks
#[utoipa::path(
    get,
    path = "/_tasks",
    responses(
        (status = 200, description = "Tasks retrieved successfully")
    ),
    tag = "Tasks"
)]
pub async fn list_tasks(
    State(_state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    tracing::info!("Listing all tasks");

    // For now, return empty task list
    // In a real implementation, this would return all running tasks
    Ok(Json(serde_json::json!({
        "nodes": {}
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    fn create_test_app() -> Router {
        let state = AppState::default();
        Router::new()
            .route("/_reindex", axum::routing::post(reindex))
            .route("/_tasks/{task_id}", axum::routing::get(get_task))
            .route(
                "/_tasks/{task_id}/_cancel",
                axum::routing::post(cancel_task),
            )
            .route("/_tasks", axum::routing::get(list_tasks))
            .with_state(state)
    }

    #[tokio::test]
    async fn test_reindex() {
        let app = create_test_app();
        let request_body = ReindexRequest {
            source: ReindexSource {
                index: "source_index".to_string(),
                query: None,
                source: None,
                source_excludes: None,
            },
            dest: ReindexDestination {
                index: "dest_index".to_string(),
                version_type: None,
                op_type: None,
            },
            script: None,
            max_docs: None,
            query: None,
        };
        let request = Request::builder()
            .uri("/_reindex")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_get_task() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_tasks/test_task_id")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cancel_task() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_tasks/test_task_id/_cancel")
            .method("POST")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let app = create_test_app();
        let request = Request::builder()
            .uri("/_tasks")
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
