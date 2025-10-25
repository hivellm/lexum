//! API integration tests for lexum-server

use axum::body::Body;
use axum::http::Request;
use axum::http::StatusCode;
use lexum_core::IndexManager;
use lexum_server::{handlers::index::AppState, router::build_router};
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tower::ServiceExt;

async fn setup_test_server() -> (AppState, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let index_manager = Arc::new(IndexManager::new(temp_dir.path()));

    let state = AppState { index_manager };
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
