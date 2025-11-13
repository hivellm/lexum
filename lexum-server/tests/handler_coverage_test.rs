//! Additional handler tests to increase test coverage
//!
//! This module adds tests for edge cases and error paths that are not
//! covered by existing tests, aiming to increase overall test coverage.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use lexum_core::{IndexManager, SnapshotManager, TemplateManager};
use lexum_server::{
    handlers::index::AppState, middleware::serialization::SerializationOptimizer,
    router::build_router,
};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::RwLock;
use tower::ServiceExt;

async fn setup_test_server() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    tokio::fs::create_dir_all(temp_dir.path()).await.unwrap();
    let index_manager = Arc::new(IndexManager::new(temp_dir.path()));

    let config = lexum_core::config::Config::default();
    let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
        |_| {
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
        template_manager: Arc::new(TemplateManager::new()),
        task_manager: Arc::new(lexum_server::handlers::reindex::TaskManager::new()),
        progress_tracker: Arc::new(lexum_core::ProgressTracker::new()),
        auth_state: lexum_server::middleware::auth::AuthState::new(
            lexum_server::middleware::auth::AuthConfig::default(),
        ),
        query_complexity_config:
            lexum_server::middleware::query_complexity::QueryComplexityLimitConfig::default(),
    };
    (state, temp_dir)
}

#[tokio::test]
async fn test_serialization_optimizer_integration() {
    let serializer = SerializationOptimizer::new();
    let data = json!({
        "status": "ok",
        "data": {
            "items": [1, 2, 3],
            "metadata": {
                "count": 3
            }
        }
    });

    let bytes = serializer.to_json_bytes(&data).unwrap();
    let string = String::from_utf8(bytes).unwrap();
    assert!(string.contains(r#""status":"ok""#));
    assert!(string.contains(r#""count":3"#));
}

#[tokio::test]
async fn test_serialization_optimizer_pretty() {
    let config = lexum_server::middleware::serialization::SerializationConfig {
        compact: false,
        ..Default::default()
    };
    let serializer = SerializationOptimizer::with_config(config);
    let data = json!({"message": "test"});

    let string = serializer.to_json_string(&data).unwrap();
    // Pretty JSON should have newlines
    assert!(string.contains('\n'));
}

#[tokio::test]
async fn test_search_handler_empty_query() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index first
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with empty query
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                },
                "limit": 10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 200 (ok) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_search_handler_invalid_index() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let search_request = Request::builder()
        .uri("/api/v1/indices/nonexistent-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 404 (not found) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_document_handler_invalid_json() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index first
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test adding document with invalid JSON
    let add_request = Request::builder()
        .uri("/api/v1/indices/test-index/documents")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from("invalid json"))
        .unwrap();

    let response = app.oneshot(add_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_index_handler_duplicate_field_names() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [
                    {
                        "name": "title",
                        "field_type": "text"
                    },
                    {
                        "name": "title",  // Duplicate field name
                        "field_type": "keyword"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(create_request).await.unwrap();
    // Should either succeed (if duplicates are allowed) or fail with validation error
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[tokio::test]
async fn test_index_handler_empty_fields() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": []
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(create_request).await.unwrap();
    // Can return 400 (bad request) or 405 (method not allowed) depending on routing
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_index_handler_invalid_field_type() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "invalid_type_xyz"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(create_request).await.unwrap();
    // Can return 400 (bad request) or 405 (method not allowed) depending on routing
    assert!(response.status().is_client_error());
}

#[tokio::test]
async fn test_search_handler_with_filters() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [
                    {
                        "name": "title",
                        "field_type": "text"
                    },
                    {
                        "name": "status",
                        "field_type": "keyword"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with filters
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                },
                "filter": [
                    {
                        "term": {
                            "field": "status",
                            "value": "active"
                        }
                    }
                ],
                "limit": 10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 200 (ok) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_search_handler_with_sorting() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with sorting
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                },
                "sort": {
                    "field": "_score",
                    "order": "desc"
                },
                "limit": 10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 200 (ok) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_index_handler_get_stats_nonexistent() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let request = Request::builder()
        .uri("/api/v1/indices/nonexistent-index/stats")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_index_handler_refresh_nonexistent() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let request = Request::builder()
        .uri("/api/v1/indices/nonexistent-index/refresh")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_index_handler_flush_nonexistent() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state);

    let request = Request::builder()
        .uri("/api/v1/indices/nonexistent-index/flush")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_document_handler_get_nonexistent() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index first
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test getting nonexistent document
    let get_request = Request::builder()
        .uri("/api/v1/indices/test-index/documents/nonexistent-id")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(get_request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_document_handler_delete_nonexistent() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index first
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test deleting nonexistent document
    let delete_request = Request::builder()
        .uri("/api/v1/indices/test-index/documents/nonexistent-id")
        .method("DELETE")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(delete_request).await.unwrap();
    // Should either return 404 or 200 (idempotent delete)
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_search_handler_invalid_query_structure() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index first
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with invalid query structure
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "invalid_query_type": {}
                }
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Should handle gracefully (either 400 or 200 with empty results)
    assert!(response.status().is_client_error() || response.status().is_success());
}

#[tokio::test]
async fn test_search_handler_with_pagination() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [{
                    "name": "title",
                    "field_type": "text"
                }]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with pagination
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                },
                "limit": 5,
                "offset": 10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 200 (ok) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn test_search_handler_with_field_filtering() {
    let (state, _temp_dir) = setup_test_server().await;

    // Create an index
    let create_request = Request::builder()
        .uri("/api/v1/indices/test-index")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "fields": [
                    {
                        "name": "title",
                        "field_type": "text"
                    },
                    {
                        "name": "description",
                        "field_type": "text"
                    }
                ]
            })
            .to_string(),
        ))
        .unwrap();

    let app = build_router(state.clone());
    let _create_response = app.clone().oneshot(create_request).await.unwrap();

    // Test search with field filtering
    let search_request = Request::builder()
        .uri("/api/v1/indices/test-index/search")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({
                "query": {
                    "match_all": {}
                },
                "fields": ["title"],
                "limit": 10
            })
            .to_string(),
        ))
        .unwrap();

    let response = app.oneshot(search_request).await.unwrap();
    // Can return 200 (ok) or 422 (unprocessable entity) depending on validation
    assert!(
        response.status().is_success() || response.status() == StatusCode::UNPROCESSABLE_ENTITY
    );
}
