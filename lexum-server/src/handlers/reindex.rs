//! Reindexing operations endpoints

use axum::{extract::State, response::Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use lexum_core::{document::store::DocumentStore, query::Query, search::SearchExecutor};

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
    /// Wait for completion before returning
    pub wait_for_completion: Option<bool>,
    /// Refresh policy for the destination index
    pub refresh: Option<bool>,
    /// Timeout for the reindex operation
    pub timeout: Option<String>,
    /// How to handle version conflicts
    pub conflicts: Option<String>,
    /// Number of retries on failure
    pub retries: Option<u32>,
    /// Throttle requests per second
    pub requests_per_second: Option<f32>,
    /// Slices configuration for parallel processing
    pub slices: Option<u32>,
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
    /// Number of documents to process per batch
    pub size: Option<u32>,
    /// Sort configuration for consistent ordering
    pub sort: Option<Vec<serde_json::Value>>,
    /// Remote source configuration for cross-cluster reindexing
    pub remote: Option<RemoteSource>,
    /// Scroll timeout for source queries
    pub scroll: Option<String>,
}

/// Remote source configuration for cross-cluster reindexing
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RemoteSource {
    /// Remote cluster host
    pub host: String,
    /// Remote cluster username
    pub username: Option<String>,
    /// Remote cluster password
    pub password: Option<String>,
    /// Remote cluster headers
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Socket timeout
    pub socket_timeout: Option<String>,
    /// Connection timeout
    pub connect_timeout: Option<String>,
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
    /// Pipeline to process documents
    pub pipeline: Option<String>,
    /// Routing value for documents
    pub routing: Option<String>,
    /// Refresh policy for the destination index
    pub refresh: Option<bool>,
    /// Document ID for create operations
    pub id: Option<String>,
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

/// Task manager for tracking reindex operations
#[derive(Debug, Clone)]
pub struct TaskManager {
    tasks: Arc<RwLock<HashMap<String, ReindexTask>>>,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new task and return its ID
    pub async fn create_task(&self, task: ReindexTask) -> String {
        let task_id = task.task_id.clone();
        let mut tasks = self.tasks.write().await;
        tasks.insert(task_id.clone(), task);
        task_id
    }

    /// Get a task by ID
    pub async fn get_task(&self, task_id: &str) -> Option<ReindexTask> {
        let tasks = self.tasks.read().await;
        tasks.get(task_id).cloned()
    }

    /// Update a task's status
    pub async fn update_task(&self, task_id: &str, status: TaskStatus) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.status = status;
            true
        } else {
            false
        }
    }

    /// List all tasks
    pub async fn list_tasks(&self) -> Vec<ReindexTask> {
        let tasks = self.tasks.read().await;
        tasks.values().cloned().collect()
    }

    /// Cancel a task
    pub async fn cancel_task(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.write().await;
        if let Some(task) = tasks.get_mut(task_id) {
            task.cancelled = true;
            true
        } else {
            false
        }
    }
}

/// Reindex task information
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReindexTask {
    /// Task ID
    pub task_id: String,
    /// Source index name
    pub source_index: String,
    /// Destination index name
    pub dest_index: String,
    /// Task status
    pub status: TaskStatus,
    /// Start time
    pub start_time: DateTime<Utc>,
    /// End time (if completed)
    pub end_time: Option<DateTime<Utc>>,
    /// Whether the task was cancelled
    pub cancelled: bool,
    /// Error message if failed
    pub error: Option<String>,
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

/// Perform the actual reindex operation
async fn perform_reindex(
    state: AppState,
    request: ReindexRequest,
    task_id: String,
) -> Result<(), String> {
    // Use configured batch size or default
    let batch_size = request.source.size.unwrap_or(100) as usize;
    let mut offset = 0;
    let mut total_processed = 0;
    let mut total_created = 0;
    let mut total_failed = 0;
    let mut total_batches = 0;
    
    // Handle throttling if configured
    let requests_per_second = request.requests_per_second.unwrap_or(0.0);
    let throttle_delay = if requests_per_second > 0.0 {
        std::time::Duration::from_millis((1000.0 / requests_per_second) as u64)
    } else {
        std::time::Duration::from_millis(0)
    };

    // Get source and destination indices
    let source_index = state
        .index_manager
        .get_index(&request.source.index)
        .map_err(|e| format!("Source index not found: {e}"))?;
    let dest_index = state
        .index_manager
        .get_index(&request.dest.index)
        .map_err(|e| format!("Destination index not found: {e}"))?;

    // Create document stores
    let _source_store = DocumentStore::new(Arc::new(source_index));
    let dest_store = DocumentStore::new(Arc::new(dest_index));

    // Create search executor for source index
    let search_executor = SearchExecutor::new(Arc::new(
        state
            .index_manager
            .get_index(&request.source.index)
            .map_err(|e| format!("Failed to get source index: {e}"))?,
    ));

    // Build query - use provided query or match all
    let query = if let Some(query_json) = request.query {
        // Parse custom query from JSON
        serde_json::from_value(query_json).map_err(|e| format!("Invalid query format: {e}"))?
    } else {
        Query::MatchAll
    };

    loop {
        // Check if task was cancelled
        if let Some(task) = state.task_manager.get_task(&task_id).await {
            if task.cancelled {
                tracing::info!("Reindex task {} was cancelled", task_id);
                return Ok(());
            }
        }

        // Search for documents in current batch
        let search_result = search_executor
            .search(query.clone(), batch_size, offset, None)
            .await
            .map_err(|e| format!("Search failed: {e}"))?;

        if search_result.hits.is_empty() {
            // No more documents to process
            break;
        }

        // Process documents in this batch
        let mut batch_created = 0;
        let mut batch_failed = 0;

        for hit in &search_result.hits {
            // Apply field filtering if specified
            let mut document = hit.source.clone();

            if let Some(source_fields) = &request.source.source {
                // Include only specified fields
                let mut filtered_doc = serde_json::Map::new();
                for field in source_fields {
                    if let Some(value) = document.get(field) {
                        filtered_doc.insert(field.clone(), value.clone());
                    }
                }
                document = serde_json::Value::Object(filtered_doc);
            }

            if let Some(exclude_fields) = &request.source.source_excludes {
                // Exclude specified fields
                if let serde_json::Value::Object(ref mut obj) = document {
                    for field in exclude_fields {
                        obj.remove(field);
                    }
                }
            }

            // Apply script transformation if provided
            if let Some(script) = &request.script {
                // For now, we'll skip script execution as it requires a script engine
                // In a real implementation, this would execute the script
                tracing::warn!(
                    "Script transformation not yet implemented, skipping script: {}",
                    script.source
                );
            }

            // Add document to destination index
            let doc_id = hit.id.clone();
            
            // Handle operation type (index vs create)
            let op_type = request.dest.op_type.as_deref().unwrap_or("index");
            let should_create = op_type == "create";
            
            // Use custom document ID if specified
            let final_doc_id = if let Some(custom_id) = &request.dest.id {
                custom_id.clone()
            } else {
                doc_id.to_string()
            };
            
            let result = if should_create {
                // For create operations, we need to check if document exists first
                // For now, we'll use add_document_with_id which will overwrite
                dest_store.add_document_with_id(final_doc_id.clone().into(), document.clone()).await
            } else {
                // For index operations, we can overwrite existing documents
                dest_store.add_document_with_id(final_doc_id.clone().into(), document.clone()).await
            };
            
            match result {
                Ok(_) => {
                    batch_created += 1;
                    total_created += 1;
                }
                Err(e) => {
                    tracing::error!("Failed to add document {}: {}", final_doc_id, e);
                    batch_failed += 1;
                    total_failed += 1;
                }
            }

            // Check max_docs limit
            if let Some(max_docs) = request.max_docs {
                if total_processed >= max_docs {
                    break;
                }
            }
        }

        total_processed += search_result.hits.len() as u64;
        total_batches += 1;

        // Update task status
        let status = TaskStatus {
            total: search_result.total as u64,
            completed: total_processed,
            failed: total_failed,
            cancelled: 0,
            created: total_created,
            deleted: 0,
            noops: 0,
            retries: 0,
            throttled_until_millis: None,
            throttled_until_nanos: None,
        };

        state.task_manager.update_task(&task_id, status).await;

        tracing::info!(
            "Reindex batch {} completed: {} documents processed, {} created, {} failed",
            total_batches,
            search_result.hits.len(),
            batch_created,
            batch_failed
        );

        // Apply throttling if configured
        if throttle_delay > std::time::Duration::from_millis(0) {
            tokio::time::sleep(throttle_delay).await;
        }

        // Move to next batch
        offset += batch_size;

        // Check if we've processed all documents
        if search_result.hits.len() < batch_size {
            break;
        }
    }

    // Mark task as completed
    if let Some(mut task) = state.task_manager.get_task(&task_id).await {
        task.end_time = Some(Utc::now());
        task.status.completed = total_processed;
        task.status.created = total_created;
        task.status.failed = total_failed;
        // Update the task in the manager
        state
            .task_manager
            .update_task(&task_id, task.status.clone())
            .await;
    }

    tracing::info!(
        "Reindex operation {} completed: {} total documents processed, {} created, {} failed",
        task_id,
        total_processed,
        total_created,
        total_failed
    );

    Ok(())
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
    State(state): State<AppState>,
    Json(request): Json<ReindexRequest>,
) -> ApiResult<Json<ReindexResponse>> {
    tracing::info!(
        "Starting reindex operation from '{}' to '{}'",
        request.source.index,
        request.dest.index
    );

    // Validate source index exists
    let source_index = state
        .index_manager
        .get_index(&request.source.index)
        .map_err(|e| {
            ApiError::Validation(format!(
                "Source index '{}' not found: {}",
                request.source.index, e
            ))
        })?;

    // Check if destination index exists, if not create it
    let _dest_index = if state.index_manager.index_exists(&request.dest.index) {
        state
            .index_manager
            .get_index(&request.dest.index)
            .map_err(|e| {
                ApiError::Validation(format!(
                    "Failed to get destination index '{}': {}",
                    request.dest.index, e
                ))
            })?
    } else {
        // Create destination index with same settings as source
        let settings = source_index.settings().clone();
        let schema = source_index.schema();
        state
            .index_manager
            .create_index(&request.dest.index, schema, settings)
            .await
            .map_err(|e| {
                ApiError::Validation(format!(
                    "Failed to create destination index '{}': {}",
                    request.dest.index, e
                ))
            })?
    };

    // Generate unique task ID
    let task_id = format!(
        "reindex_{}_{}_{}",
        request.source.index,
        request.dest.index,
        &Uuid::new_v4().to_string()[..8]
    );

    // Create task
    let task = ReindexTask {
        task_id: task_id.clone(),
        source_index: request.source.index.clone(),
        dest_index: request.dest.index.clone(),
        status: TaskStatus {
            total: 0,
            completed: 0,
            failed: 0,
            cancelled: 0,
            created: 0,
            deleted: 0,
            noops: 0,
            retries: 0,
            throttled_until_millis: None,
            throttled_until_nanos: None,
        },
        start_time: Utc::now(),
        end_time: None,
        cancelled: false,
        error: None,
    };

    // Store task
    state.task_manager.create_task(task).await;

    // Start reindex operation
    let state_clone = state.clone();
    let request_clone = request.clone();
    let task_id_clone = task_id.clone();

    if request.wait_for_completion.unwrap_or(false) {
        // Wait for completion if requested
        if let Err(e) = perform_reindex(state_clone, request_clone, task_id_clone).await {
            tracing::error!("Reindex operation failed: {}", e);
            return Err(ApiError::Internal(format!("Reindex operation failed: {}", e)));
        }
    } else {
        // Run in background
        tokio::spawn(async move {
            if let Err(e) = perform_reindex(state_clone, request_clone, task_id_clone).await {
                tracing::error!("Reindex operation failed: {}", e);
            }
        });
    }

    Ok(Json(ReindexResponse {
        task: task_id,
        acknowledged: true,
        total: 0, // Will be updated as operation progresses
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
    State(state): State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> ApiResult<Json<TaskInfo>> {
    tracing::info!("Getting task information for task: {}", task_id);

    let task = state
        .task_manager
        .get_task(&task_id)
        .await
        .ok_or_else(|| ApiError::InvalidRequest(format!("Task {task_id} not found")))?;

    let running_time = if let Some(end_time) = task.end_time {
        ((end_time.timestamp_millis() - task.start_time.timestamp_millis()) * 1_000_000) as u64
    } else {
        ((Utc::now().timestamp_millis() - task.start_time.timestamp_millis()) * 1_000_000) as u64
    };

    Ok(Json(TaskInfo {
        task_id: task.task_id,
        task_type: "reindex".to_string(),
        action: "indices:data/write/reindex".to_string(),
        description: format!(
            "reindex from [{}] to [{}]",
            task.source_index, task.dest_index
        ),
        status: task.status,
        start_time_in_millis: task.start_time.timestamp_millis() as u64,
        running_time_in_nanos: running_time,
        parent_task_id: None,
        cancellable: !task.cancelled,
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
    State(state): State<AppState>,
    axum::extract::Path(task_id): axum::extract::Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!("Cancelling task: {}", task_id);

    let cancelled = state.task_manager.cancel_task(&task_id).await;

    if cancelled {
        Ok(Json(serde_json::json!({
            "acknowledged": true
        })))
    } else {
        Err(ApiError::InvalidRequest(format!(
            "Task {task_id} not found"
        )))
    }
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
pub async fn list_tasks(State(state): State<AppState>) -> ApiResult<Json<serde_json::Value>> {
    tracing::info!("Listing all tasks");

    let tasks = state.task_manager.list_tasks().await;

    let mut nodes = serde_json::Map::new();
    let mut node_tasks = serde_json::Map::new();

    for task in tasks {
        let task_info = TaskInfo {
            task_id: task.task_id.clone(),
            task_type: "reindex".to_string(),
            action: "indices:data/write/reindex".to_string(),
            description: format!(
                "reindex from [{}] to [{}]",
                task.source_index, task.dest_index
            ),
            status: task.status,
            start_time_in_millis: task.start_time.timestamp_millis() as u64,
            running_time_in_nanos: if let Some(end_time) = task.end_time {
                ((end_time.timestamp_millis() - task.start_time.timestamp_millis()) * 1_000_000)
                    as u64
            } else {
                ((Utc::now().timestamp_millis() - task.start_time.timestamp_millis()) * 1_000_000)
                    as u64
            },
            parent_task_id: None,
            cancellable: !task.cancelled,
            headers: serde_json::json!({}),
        };

        node_tasks.insert(task.task_id, serde_json::to_value(task_info).unwrap());
    }

    nodes.insert("tasks".to_string(), serde_json::Value::Object(node_tasks));

    Ok(Json(serde_json::json!({
        "nodes": {
            "local": {
                "tasks": nodes
            }
        }
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
                size: None,
                sort: None,
                remote: None,
                scroll: None,
            },
            dest: ReindexDestination {
                index: "dest_index".to_string(),
                version_type: None,
                op_type: None,
                pipeline: None,
                routing: None,
                refresh: None,
                id: None,
            },
            script: None,
            max_docs: None,
            query: None,
            wait_for_completion: None,
            refresh: None,
            timeout: None,
            conflicts: None,
            retries: None,
            requests_per_second: None,
            slices: None,
        };
        let request = Request::builder()
            .uri("/_reindex")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Should return 400 because source index doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_reindex_with_enhanced_config() {
        let app = create_test_app();
        let request_body = ReindexRequest {
            source: ReindexSource {
                index: "source_index".to_string(),
                query: Some(serde_json::json!({"match_all": {}})),
                source: Some(vec!["title".to_string(), "content".to_string()]),
                source_excludes: Some(vec!["_id".to_string()]),
                size: Some(50),
                sort: Some(vec![serde_json::json!({"created_at": {"order": "desc"}})]),
                remote: Some(RemoteSource {
                    host: "remote-cluster:9200".to_string(),
                    username: Some("admin".to_string()),
                    password: Some("password".to_string()),
                    headers: None,
                    socket_timeout: Some("30s".to_string()),
                    connect_timeout: Some("10s".to_string()),
                }),
                scroll: Some("5m".to_string()),
            },
            dest: ReindexDestination {
                index: "dest_index".to_string(),
                version_type: Some("external".to_string()),
                op_type: Some("create".to_string()),
                pipeline: Some("my_pipeline".to_string()),
                routing: Some("user_id".to_string()),
                refresh: Some(true),
                id: Some("custom_id".to_string()),
            },
            script: Some(ReindexScript {
                source: "ctx._source.new_field = 'transformed'".to_string(),
                lang: Some("painless".to_string()),
                params: Some(serde_json::json!({"multiplier": 2})),
            }),
            max_docs: Some(1000),
            query: Some(serde_json::json!({"range": {"created_at": {"gte": "2023-01-01"}}})),
            wait_for_completion: Some(true),
            refresh: Some(true),
            timeout: Some("10m".to_string()),
            conflicts: Some("proceed".to_string()),
            retries: Some(3),
            requests_per_second: Some(100.0),
            slices: Some(4),
        };
        
        let request = Request::builder()
            .uri("/_reindex")
            .method("POST")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&request_body).unwrap()))
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        // Should return 400 because source index doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
        // Should return 400 because task doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
        // Should return 400 because task doesn't exist
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
