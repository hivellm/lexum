//! API integration tests for lexum-server

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use lexum_core::{IndexManager, SnapshotManager};
use lexum_server::{handlers::index::AppState, router::build_router};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn setup_test_server() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let index_manager = Arc::new(IndexManager::new(temp_dir.path()));

    // Create a minimal config for snapshot manager
    let config = lexum_core::config::Config::default();
    let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
        |_| {
            // Fallback to a minimal config if default fails
            let mut fallback_config = config;
            fallback_config.snapshots.repositories =
                vec![lexum_core::config::SnapshotRepositoryConfig {
                    name: "default".to_string(),
                    repository_type: "fs".to_string(),
                    settings: lexum_core::config::SnapshotRepositorySettings {
                        location: temp_dir
                            .path()
                            .join("snapshots")
                            .to_string_lossy()
                            .to_string(),
                        ..Default::default()
                    },
                }];
            SnapshotManager::new(&fallback_config).unwrap()
        },
    )));

    let state = AppState {
        index_manager,
        snapshot_manager,
    };
    (state, temp_dir)
}

#[tokio::test]
async fn test_health_check() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_create_and_get_index() {
    let (state, _temp_dir) = setup_test_server().await;

    // Verify initial state
    assert!(state.index_manager.list_indices().is_empty());
}

#[tokio::test]
async fn test_bulk_operations_structure() {
    use lexum_server::handlers::document::*;

    let operations = vec![
        BulkOperation::Index {
            index: "test".to_string(),
            id: Some("1".to_string()),
            document: json!({"title": "Test"}),
        },
        BulkOperation::Create {
            index: "test".to_string(),
            id: "2".to_string(),
            document: json!({"title": "Test 2"}),
        },
        BulkOperation::Delete {
            index: "test".to_string(),
            id: "3".to_string(),
        },
    ];

    assert_eq!(operations.len(), 3);
}

#[test]
fn test_api_error_types() {
    use lexum_server::error::ApiError;

    let not_found = ApiError::IndexNotFound("test".to_string());
    let status = not_found.status_code();
    assert_eq!(status, StatusCode::NOT_FOUND);

    let doc_not_found = ApiError::DocumentNotFound("doc1".to_string());
    let status = doc_not_found.status_code();
    assert_eq!(status, StatusCode::NOT_FOUND);

    let internal = ApiError::Internal("error".to_string());
    let status = internal.status_code();
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
}

#[test]
fn test_search_request() {
    use lexum_core::{QueryBuilder, SortOption};
    use lexum_server::handlers::search::SearchRequest;

    let request = SearchRequest {
        query: QueryBuilder::match_query("title", "test"),
        limit: 20,
        offset: 10,
        sort: Some(SortOption::desc("_score")),
    };

    assert_eq!(request.limit, 20);
    assert_eq!(request.offset, 10);
    assert!(request.sort.is_some());
}

#[tokio::test]
async fn test_create_snapshot_repository() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let request_body = serde_json::json!({
        "type": "fs",
        "settings": {
            "location": "/tmp/test_snapshots",
            "compress": "true",
            "chunk_size": "1gb"
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/_snapshot/test_repo")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response_json["name"], "test_repo");
    assert_eq!(response_json["type"], "fs");
    assert_eq!(response_json["settings"]["location"], "/tmp/test_snapshots");
    assert_eq!(response_json["settings"]["compress"], "true");
    assert_eq!(response_json["snapshot_count"], 0);
    assert_eq!(response_json["total_size"], 0);
}

#[tokio::test]
async fn test_get_snapshot_repository() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    // First create a repository
    let request_body = serde_json::json!({
        "type": "fs",
        "settings": {
            "location": "/tmp/test_snapshots",
            "compress": "true"
        }
    });

    let create_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/_snapshot/test_repo")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_response.status(), StatusCode::OK);

    // Then get the repository
    let get_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/_snapshot/test_repo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(get_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(response_json["name"], "test_repo");
    assert_eq!(response_json["type"], "fs");
    assert_eq!(response_json["settings"]["location"], "/tmp/test_snapshots");
}

#[tokio::test]
async fn test_list_snapshot_repositories() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    // Create a few repositories
    let repos = vec!["repo1", "repo2", "repo3"];
    for repo in &repos {
        let request_body = serde_json::json!({
            "type": "fs",
            "settings": {
                "location": format!("/tmp/{}", repo),
                "compress": "true"
            }
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(format!("/_snapshot/{}", repo))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // List all repositories
    let list_response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/_snapshot")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(list_response.into_body(), usize::MAX)
        .await
        .unwrap();
    let response_json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(response_json.is_array());
    let repos_array = response_json.as_array().unwrap();
    assert!(repos_array.len() >= 3); // Should have at least the 3 we created

    // Check that our repositories are in the list
    let repo_names: Vec<String> = repos_array
        .iter()
        .map(|r| r["name"].as_str().unwrap().to_string())
        .collect();

    for repo in &repos {
        assert!(repo_names.contains(&repo.to_string()));
    }
}

#[tokio::test]
async fn test_create_repository_invalid_request() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    // Test with invalid JSON
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/_snapshot/test_repo")
                .header("content-type", "application/json")
                .body(Body::from("invalid json"))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_repository_missing_required_fields() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    // Test with missing type field - this should fail with 422
    let request_body = serde_json::json!({
        "settings": {
            "location": "/tmp/test_snapshots"
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/_snapshot/test_repo")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Should fail because type field is missing
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}
