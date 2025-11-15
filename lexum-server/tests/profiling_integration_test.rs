//! Integration tests for profiling endpoints

use axum::body::Body;
use axum::http::{Request, StatusCode};
use lexum_core::{IndexManager, SnapshotManager, TemplateManager};
use lexum_server::{
    handlers::index::AppState, middleware::http2_push::Http2PushConfig, router::build_router,
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
        metrics: Arc::new(lexum_server::handlers::metrics::PrometheusMetrics::new()),
    };
    (state, temp_dir)
}

#[tokio::test]
async fn test_profiling_start_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request = Request::builder()
        .uri("/_profiling/start?duration_secs=10&sampling_rate=100&cpu_profiling=true&memory_profiling=false")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(status["active"].as_bool().unwrap());
    assert!(status["start_time"].is_string());
}

#[tokio::test]
async fn test_profiling_stop_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request = Request::builder()
        .uri("/_profiling/stop")
        .method("POST")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(result["success"].as_bool().unwrap());
    assert!(result["flamegraph_svg"].is_string());
    assert!(result["statistics"].is_object());
}

#[tokio::test]
async fn test_profiling_status_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request = Request::builder()
        .uri("/_profiling/status")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let status: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(status["active"].is_boolean());
}

#[tokio::test]
async fn test_profiling_flamegraph_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let config = json!({
        "duration_secs": 30,
        "sampling_rate": 200,
        "cpu_profiling": true,
        "memory_profiling": false
    });

    let request = Request::builder()
        .uri("/_profiling/flamegraph")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(config.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(result["success"].as_bool().unwrap());
    assert!(result["flamegraph_svg"].is_string());
}

#[tokio::test]
async fn test_profiling_instructions_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request = Request::builder()
        .uri("/_profiling/instructions")
        .method("GET")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["content-type"], "text/plain");

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let instructions = String::from_utf8(body.to_vec()).unwrap();
    assert!(instructions.contains("flamegraph"));
    assert!(instructions.contains("profiling"));
}

#[tokio::test]
async fn test_bottleneck_analysis_endpoint() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request_body = json!({
        "min_percentage": 5.0,
        "min_samples": 100,
        "include_recommendations": true
    });

    let request = Request::builder()
        .uri("/_profiling/bottlenecks")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(result["success"].as_bool().unwrap());
    assert!(result["bottlenecks"].is_array());
    assert!(result["summary"].is_object());
    assert!(result["recommendations"].is_array());
}

#[tokio::test]
async fn test_bottleneck_analysis_without_recommendations() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state, &Http2PushConfig::default());

    let request_body = json!({
        "min_percentage": 5.0,
        "min_samples": 100,
        "include_recommendations": false
    });

    let request = Request::builder()
        .uri("/_profiling/bottlenecks")
        .method("POST")
        .header("content-type", "application/json")
        .body(Body::from(request_body.to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(result["success"].as_bool().unwrap());
    assert_eq!(result["recommendations"].as_array().unwrap().len(), 0);
}
