//! Snapshot and restore handlers

use crate::handlers::index::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use lexum_core::{
    config::SnapshotRepositoryConfig,
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
    State(state): State<AppState>,
    Path(repository_name): Path<String>,
    Json(request): Json<CreateRepositoryRequest>,
) -> Result<Json<RepositoryResponse>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name.clone());

    // Convert the request to a SnapshotRepositoryConfig
    let config = SnapshotRepositoryConfig {
        name: repository_name.clone(),
        repository_type: request.repository_type.clone(),
        settings: lexum_core::config::SnapshotRepositorySettings {
            location: request
                .settings
                .get("location")
                .cloned()
                .unwrap_or_default(),
            compress: request
                .settings
                .get("compress")
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            chunk_size: request
                .settings
                .get("chunk_size")
                .cloned()
                .unwrap_or_else(|| "1gb".to_string()),
            max_restore_bytes_per_sec: request
                .settings
                .get("max_restore_bytes_per_sec")
                .cloned()
                .unwrap_or_else(|| "40mb".to_string()),
            max_snapshot_bytes_per_sec: request
                .settings
                .get("max_snapshot_bytes_per_sec")
                .cloned()
                .unwrap_or_else(|| "40mb".to_string()),
            readonly: request
                .settings
                .get("readonly")
                .and_then(|s| s.parse().ok())
                .unwrap_or(false),
            ..Default::default()
        },
    };

    // Create or update the repository
    let mut snapshot_manager = state.snapshot_manager.write().await;
    let repository_info = snapshot_manager
        .create_or_update_repository(repo_name.clone(), config)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert RepositoryInfo to RepositoryResponse
    let response = RepositoryResponse {
        name: repository_info.name.as_str().to_string(),
        repository_type: repository_info.repository_type,
        settings: repository_info.settings,
        snapshot_count: repository_info.snapshot_count,
        total_size: repository_info.total_size,
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
    State(state): State<AppState>,
    Path(repository_name): Path<String>,
) -> Result<Json<RepositoryResponse>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);

    let snapshot_manager = state.snapshot_manager.read().await;
    let info = snapshot_manager
        .get_repository_info(&repo_name)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let response = RepositoryResponse {
        name: info.name.as_str().to_string(),
        repository_type: info.repository_type,
        settings: info.settings,
        snapshot_count: info.snapshot_count,
        total_size: info.total_size,
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
    State(state): State<AppState>,
) -> Result<Json<Vec<RepositoryResponse>>, StatusCode> {
    let snapshot_manager = state.snapshot_manager.read().await;
    let infos = snapshot_manager
        .list_repositories_info()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let responses: Vec<RepositoryResponse> = infos
        .into_iter()
        .map(|info| RepositoryResponse {
            name: info.name.as_str().to_string(),
            repository_type: info.repository_type,
            settings: info.settings,
            snapshot_count: info.snapshot_count,
            total_size: info.total_size,
        })
        .collect();

    Ok(Json(responses))
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

    let snapshot_manager = state.snapshot_manager.read().await;
    let snapshot_info = snapshot_manager
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

    let snapshot_manager = state.snapshot_manager.read().await;
    let snapshot_info = snapshot_manager
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

    let snapshot_manager = state.snapshot_manager.read().await;
    let snapshots = snapshot_manager
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

    let snapshot_manager = state.snapshot_manager.read().await;
    snapshot_manager
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
        (status = 404, description = "Snapshot or repository not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "Snapshots"
)]
pub async fn restore_snapshot(
    State(state): State<AppState>,
    Path((repository_name, snapshot_name)): Path<(String, String)>,
    Json(request): Json<RestoreSnapshotRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let repo_name = RepositoryName::new(repository_name);
    let snap_name = SnapshotName::new(snapshot_name.clone());

    let snapshot_manager = state.snapshot_manager.read().await;
    snapshot_manager
        .restore_snapshot(&repo_name, snap_name, request)
        .await
        .map_err(|e| {
            tracing::error!(
                repository = %repo_name.as_str(),
                snapshot = %snapshot_name,
                error = %e,
                "Failed to restore snapshot"
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let response = serde_json::json!({
        "acknowledged": true,
        "message": "Snapshot restore completed successfully"
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

    let snapshot_manager = state.snapshot_manager.read().await;
    let stats = snapshot_manager
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
    let snapshot_manager = state.snapshot_manager.read().await;
    let stats = snapshot_manager
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
    use lexum_core::types::IndexName;
    use std::collections::HashMap;

    #[lexum_macros::tokio_test]
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

    #[lexum_macros::tokio_test]
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

    #[lexum_macros::tokio_test]
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

    #[lexum_macros::tokio_test]
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

    #[lexum_macros::tokio_test]
    async fn test_create_repository_with_s3_settings() {
        use axum::Json;
        use axum::extract::State;

        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "my-s3-bucket".to_string());
        settings.insert("compress".to_string(), "true".to_string());
        settings.insert("chunk_size".to_string(), "512mb".to_string());

        let request = CreateRepositoryRequest {
            repository_type: "s3".to_string(),
            settings,
        };

        let state = AppState::default();
        let result =
            create_or_update_repository(State(state), Path("s3_repo".to_string()), Json(request))
                .await;

        // S3 is not yet implemented, so this should fail
        assert!(result.is_err());
    }

    #[lexum_macros::tokio_test]
    async fn test_create_repository_with_invalid_settings() {
        use axum::Json;
        use axum::extract::State;

        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "".to_string()); // Empty location should fail

        let request = CreateRepositoryRequest {
            repository_type: "fs".to_string(),
            settings,
        };

        let state = AppState::default();
        let result = create_or_update_repository(
            State(state),
            Path("invalid_repo".to_string()),
            Json(request),
        )
        .await;

        // Should return an error due to invalid settings
        assert!(result.is_err());
    }

    #[lexum_macros::tokio_test]
    async fn test_create_repository_with_default_settings() {
        use axum::Json;
        use axum::extract::State;

        let mut settings = HashMap::new();
        settings.insert("location".to_string(), "/tmp/default_repo".to_string());

        let request = CreateRepositoryRequest {
            repository_type: "fs".to_string(),
            settings,
        };

        let state = AppState::default();
        let result = create_or_update_repository(
            State(state),
            Path("default_repo".to_string()),
            Json(request),
        )
        .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.name, "default_repo");
        assert_eq!(response.repository_type, "fs");
        // Should have default settings
        assert!(response.settings.contains_key("location"));
        assert!(response.settings.contains_key("compress"));
    }

    #[lexum_macros::tokio_test]
    async fn test_update_existing_repository() {
        use axum::Json;
        use axum::extract::State;

        let mut settings1 = HashMap::new();
        settings1.insert("location".to_string(), "/tmp/repo1".to_string());
        settings1.insert("compress".to_string(), "true".to_string());

        let request1 = CreateRepositoryRequest {
            repository_type: "fs".to_string(),
            settings: settings1,
        };

        let state = AppState::default();

        // Create first repository
        let result1 = create_or_update_repository(
            State(state.clone()),
            Path("update_repo".to_string()),
            Json(request1),
        )
        .await;
        assert!(result1.is_ok());

        // Update the same repository with different settings
        let mut settings2 = HashMap::new();
        settings2.insert("location".to_string(), "/tmp/repo2".to_string());
        settings2.insert("compress".to_string(), "false".to_string());
        settings2.insert("chunk_size".to_string(), "2gb".to_string());

        let request2 = CreateRepositoryRequest {
            repository_type: "fs".to_string(),
            settings: settings2,
        };

        let result2 = create_or_update_repository(
            State(state),
            Path("update_repo".to_string()),
            Json(request2),
        )
        .await;

        assert!(result2.is_ok());
        let response = result2.unwrap();
        assert_eq!(response.name, "update_repo");
        assert_eq!(response.repository_type, "fs");
        // Should have updated settings
        assert_eq!(
            response.settings.get("location"),
            Some(&"/tmp/repo2".to_string())
        );
        assert_eq!(
            response.settings.get("compress"),
            Some(&"false".to_string())
        );
        assert_eq!(
            response.settings.get("chunk_size"),
            Some(&"2gb".to_string())
        );
    }

    #[lexum_macros::tokio_test]
    async fn test_restore_snapshot_handler() {
        use axum::Json;
        use axum::extract::State;

        let state = AppState::default();
        let restore_request = RestoreSnapshotRequest::default();

        let result = restore_snapshot(
            State(state),
            Path(("test_repo".to_string(), "test_snapshot".to_string())),
            Json(restore_request),
        )
        .await;

        // Should fail because snapshot doesn't exist
        assert!(result.is_err());
    }

    #[lexum_macros::tokio_test]
    async fn test_restore_snapshot_with_rename() {
        use axum::Json;
        use axum::extract::State;

        let state = AppState::default();
        let restore_request = RestoreSnapshotRequest {
            indices: vec![IndexName::new("index1")],
            rename_pattern: Some("index1".to_string()),
            rename_replacement: Some("restored_index1".to_string()),
            ..Default::default()
        };

        let result = restore_snapshot(
            State(state),
            Path(("test_repo".to_string(), "test_snapshot".to_string())),
            Json(restore_request),
        )
        .await;

        // Should fail because snapshot doesn't exist
        assert!(result.is_err());
    }
}
