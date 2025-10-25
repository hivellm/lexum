//! Snapshot and restore handlers

use crate::handlers::index::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::Utc;
use lexum_core::{
    snapshot::{CreateSnapshotRequest, RestoreSnapshotRequest, SnapshotInfo, SnapshotStats},
    types::{RepositoryName, SnapshotName},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Snapshot repository creation request
#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateRepositoryRequest {
    /// Repository type (fs, s3, gcs, azure)
    #[serde(rename = "type")]
    pub repository_type: String,

    /// Repository settings
    pub settings: HashMap<String, String>,
}

/// Snapshot repository response
#[derive(Debug, Serialize, ToSchema)]
pub struct RepositoryResponse {
    /// Repository name
    pub name: String,

    /// Repository type
    #[serde(rename = "type")]
    pub repository_type: String,

    /// Repository settings
    pub settings: HashMap<String, String>,

    /// Number of snapshots
    pub snapshot_count: u32,

    /// Total size in bytes
    pub total_size: u64,
}

/// Snapshot creation response
#[derive(Debug, Serialize, ToSchema)]
pub struct SnapshotResponse {
    /// Snapshot information
    pub snapshot: SnapshotInfo,

    /// Whether the operation was acknowledged
    pub acknowledged: bool,
}

/// Snapshot list response
#[derive(Debug, Serialize, ToSchema)]
pub struct SnapshotListResponse {
    /// List of snapshots
    pub snapshots: Vec<SnapshotInfo>,
}

/// Snapshot statistics response
#[derive(Debug, Serialize, ToSchema)]
pub struct SnapshotStatsResponse {
    /// Snapshot statistics
    pub stats: SnapshotStats,
}

/// Create or update a snapshot repository (PUT /_snapshot/{repository})
#[utoipa::path(
    put,
    path = "/_snapshot/{repository}",
    params(
        ("repository" = String, Path, description = "Repository name")
    ),
    request_body = CreateRepositoryRequest,
    responses(
        (status = 200, description = "Repository created or updated successfully", body = RepositoryResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn create_or_update_repository(
    State(_state): State<AppState>,
    Path(repository_name): Path<String>,
    Json(request): Json<CreateRepositoryRequest>,
) -> Result<Json<RepositoryResponse>, StatusCode> {
    let _repo_name = RepositoryName::new(repository_name.clone());

    // TODO: Implement repository create/update logic
    // This would involve:
    // 1. Check if repository exists
    // 2. If exists: validate and update settings
    // 3. If not exists: create new repository
    // 4. Handle any migration if repository type changes
    // 5. Return appropriate response

    // For now, return a placeholder response
    let response = RepositoryResponse {
        name: repository_name,
        repository_type: request.repository_type,
        settings: request.settings,
        snapshot_count: 0, // TODO: Get actual count from state
        total_size: 0,     // TODO: Get actual size from state
    };

    Ok(Json(response))
}

/// Get repository information
#[utoipa::path(
    get,
    path = "/_snapshot/{repository}",
    params(
        ("repository" = String, Path, description = "Repository name")
    ),
    responses(
        (status = 200, description = "Repository information", body = RepositoryResponse),
        (status = 404, description = "Repository not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn get_repository(
    State(_state): State<AppState>,
    Path(repository_name): Path<String>,
) -> Result<Json<RepositoryResponse>, StatusCode> {
    let _repo_name = RepositoryName::new(repository_name);

    // TODO: Get actual repository from state
    // let snapshot_manager = &state.snapshot_manager;
    // let info = snapshot_manager.get_repository_info(&repo_name).await
    //     .map_err(|_| StatusCode::NOT_FOUND)?;

    // For now, return a placeholder response
    let response = RepositoryResponse {
        name: repo_name.as_str().to_string(),
        repository_type: "fs".to_string(),
        settings: HashMap::new(),
        snapshot_count: 0,
        total_size: 0,
    };

    Ok(Json(response))
}

/// List all repositories
#[utoipa::path(
    get,
    path = "/_snapshot",
    responses(
        (status = 200, description = "List of repositories", body = Vec<RepositoryResponse>),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn list_repositories(
    State(_state): State<AppState>,
) -> Result<Json<Vec<RepositoryResponse>>, StatusCode> {
    // TODO: Get actual repositories from state
    // let snapshot_manager = &state.snapshot_manager;
    // let infos = snapshot_manager.list_repositories_info().await
    //     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // For now, return empty list
    Ok(Json(vec![]))
}

/// Create a snapshot
#[utoipa::path(
    put,
    path = "/_snapshot/{repository}/{snapshot}",
    params(
        ("repository" = String, Path, description = "Repository name"),
        ("snapshot" = String, Path, description = "Snapshot name")
    ),
    request_body = CreateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot created successfully", body = SnapshotResponse),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn create_snapshot(
    State(state): State<AppState>,
    Path((repository_name, snapshot_name)): Path<(String, String)>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotResponse>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);
    let snap_name = SnapshotName::new(snapshot_name);

    let snapshot_info = state.snapshot_manager
        .create_snapshot(&repo_name, snap_name, request)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SnapshotResponse {
        snapshot: snapshot_info,
        acknowledged: true,
    };

    Ok(Json(response))
}

/// Get snapshot information
#[utoipa::path(
    get,
    path = "/_snapshot/{repository}/{snapshot}",
    params(
        ("repository" = String, Path, description = "Repository name"),
        ("snapshot" = String, Path, description = "Snapshot name")
    ),
    responses(
        (status = 200, description = "Snapshot information", body = SnapshotInfo),
        (status = 404, description = "Snapshot not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn get_snapshot(
    State(state): State<AppState>,
    Path((repository_name, snapshot_name)): Path<(String, String)>,
) -> Result<Json<SnapshotInfo>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);
    let snap_name = SnapshotName::new(snapshot_name);

    let snapshot_info = state.snapshot_manager
        .get_snapshot(&repo_name, snap_name)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok(Json(snapshot_info))
}

/// List snapshots in a repository
#[utoipa::path(
    get,
    path = "/_snapshot/{repository}/_all",
    params(
        ("repository" = String, Path, description = "Repository name")
    ),
    responses(
        (status = 200, description = "List of snapshots", body = SnapshotListResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn list_snapshots(
    State(state): State<AppState>,
    Path(repository_name): Path<String>,
) -> Result<Json<SnapshotListResponse>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);

    let snapshots = state.snapshot_manager
        .list_snapshots(&repo_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SnapshotListResponse { snapshots };

    Ok(Json(response))
}

/// Delete a snapshot
#[utoipa::path(
    delete,
    path = "/_snapshot/{repository}/{snapshot}",
    params(
        ("repository" = String, Path, description = "Repository name"),
        ("snapshot" = String, Path, description = "Snapshot name")
    ),
    responses(
        (status = 200, description = "Snapshot deleted successfully"),
        (status = 404, description = "Snapshot not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn delete_snapshot(
    State(state): State<AppState>,
    Path((repository_name, snapshot_name)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);
    let snap_name = SnapshotName::new(snapshot_name);

    state.snapshot_manager
        .delete_snapshot(&repo_name, snap_name)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let response = serde_json::json!({
        "acknowledged": true
    });

    Ok(Json(response))
}

/// Restore from snapshot
#[utoipa::path(
    post,
    path = "/_snapshot/{repository}/{snapshot}/_restore",
    params(
        ("repository" = String, Path, description = "Repository name"),
        ("snapshot" = String, Path, description = "Snapshot name")
    ),
    request_body = RestoreSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot restore initiated successfully"),
        (status = 400, description = "Bad request"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn restore_snapshot(
    State(_state): State<AppState>,
    Path((repository_name, snapshot_name)): Path<(String, String)>,
    Json(_request): Json<RestoreSnapshotRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _repo_name = RepositoryName::new(repository_name);
    let _snap_name = SnapshotName::new(snapshot_name);

    // TODO: Restore actual snapshot from state
    // let snapshot_manager = &state.snapshot_manager;
    // snapshot_manager.restore_snapshot(&repo_name, snap_name, request).await
    //     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = serde_json::json!({
        "acknowledged": true
    });

    Ok(Json(response))
}

/// Get snapshot statistics
#[utoipa::path(
    get,
    path = "/_snapshot/{repository}/_stats",
    params(
        ("repository" = String, Path, description = "Repository name")
    ),
    responses(
        (status = 200, description = "Snapshot statistics", body = SnapshotStatsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn get_snapshot_stats(
    State(state): State<AppState>,
    Path(repository_name): Path<String>,
) -> Result<Json<SnapshotStatsResponse>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);

    let stats = state.snapshot_manager
        .get_repository_stats(&repo_name)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SnapshotStatsResponse { stats };

    Ok(Json(response))
}

/// Get global snapshot statistics
#[utoipa::path(
    get,
    path = "/_snapshot/_stats",
    responses(
        (status = 200, description = "Global snapshot statistics", body = SnapshotStatsResponse),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn get_global_snapshot_stats(
    State(state): State<AppState>,
) -> Result<Json<SnapshotStatsResponse>, StatusCode> {
    let stats = state.snapshot_manager
        .get_global_stats()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = SnapshotStatsResponse { stats };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_create_repository_request_deserialization() {
        let json = r#"{
            "type": "fs",
            "settings": {
                "location": "/tmp/snapshots",
                "compress": "true"
            }
        }"#;

        let request: CreateRepositoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.repository_type, "fs");
        assert_eq!(
            request.settings.get("location"),
            Some(&"/tmp/snapshots".to_string())
        );
        assert_eq!(request.settings.get("compress"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_repository_response_serialization() {
        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "/tmp/snapshots".to_string());

        let response = RepositoryResponse {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings,
            snapshot_count: 5,
            total_size: 1024,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_repo"));
        assert!(json.contains("fs"));
        assert!(json.contains("5"));
    }

    #[tokio::test]
    async fn test_create_or_update_repository_request_deserialization() {
        let json = r#"{
            "type": "s3",
            "settings": {
                "bucket": "my-snapshots",
                "region": "us-west-2",
                "compress": "true"
            }
        }"#;

        let request: CreateRepositoryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.repository_type, "s3");
        assert_eq!(
            request.settings.get("bucket"),
            Some(&"my-snapshots".to_string())
        );
        assert_eq!(
            request.settings.get("region"),
            Some(&"us-west-2".to_string())
        );
        assert_eq!(request.settings.get("compress"), Some(&"true".to_string()));
    }

    #[tokio::test]
    async fn test_create_or_update_repository_handler() {
        use axum::Json;
        use axum::extract::State;

        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "/tmp/snapshots".to_string());
        settings.insert("compress".to_string(), "true".to_string());

        let request = CreateRepositoryRequest {
            repository_type: "fs".to_string(),
            settings,
        };

        let state = AppState::default();
        let result =
            create_or_update_repository(State(state), Path("test_repo".to_string()), Json(request))
                .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.name, "test_repo");
        assert_eq!(response.repository_type, "fs");
        assert_eq!(response.snapshot_count, 0);
        assert_eq!(response.total_size, 0);
    }
}
