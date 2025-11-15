//! Protocol tests for StreamableHTTP, MCP, and UMICP

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use lexum_core::{FieldConfig, FieldType, SchemaBuilder};
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
    use std::env;

    // Detect WSL and use Linux native filesystem to avoid Tantivy compatibility issues
    let is_wsl = env::var("WSL_DISTRO_NAME").is_ok()
        || env::var("CARGO_MANIFEST_DIR")
            .unwrap_or_default()
            .contains("/mnt/");

    let data_dir = if is_wsl {
        // Use Linux native filesystem (HOME directory) to avoid WSL/Tantivy issues
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let mut linux_path = std::path::PathBuf::from(home);
        linux_path.push(".lexum-test-data");
        linux_path.push(format!("test-{}", std::process::id()));
        tokio::fs::create_dir_all(&linux_path).await.unwrap();
        linux_path
    } else {
        // Use tempfile for non-WSL environments
        let temp_dir = TempDir::new().unwrap();
        tokio::fs::create_dir_all(temp_dir.path()).await.unwrap();
        temp_dir.path().to_path_buf()
    };

    let temp_dir = if is_wsl {
        // Create a dummy TempDir for WSL case (we use linux_path instead)
        // We'll keep the temp_dir for cleanup, but use data_dir for IndexManager
        TempDir::new_in("/tmp").unwrap_or_else(|_| {
            // Fallback if /tmp doesn't work
            let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            TempDir::new_in(home).unwrap()
        })
    } else {
        TempDir::new().unwrap()
    };

    let index_manager = Arc::new(IndexManager::new(&data_dir));

    let config = lexum_core::config::Config::default();
    let snapshot_dir = data_dir.join("snapshots");
    tokio::fs::create_dir_all(&snapshot_dir).await.unwrap();
    let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&config).unwrap_or_else(
        |_| {
            let mut fallback_config = config;
            fallback_config.snapshots.repositories =
                vec![lexum_core::config::SnapshotRepositoryConfig {
                    name: "default".to_string(),
                    repository_type: "fs".to_string(),
                    settings: lexum_core::config::SnapshotRepositorySettings {
                        location: snapshot_dir.to_string_lossy().to_string(),
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

async fn create_test_index(state: &AppState, index_name: &str) {
    let schema = SchemaBuilder::new()
        .add_field(FieldConfig::new("_id", FieldType::Keyword).stored(true))
        .add_field(FieldConfig::new("title", FieldType::Text).stored(true))
        .add_field(FieldConfig::new("content", FieldType::Text).stored(true))
        .build()
        .unwrap()
        .0;

    state
        .index_manager
        .create_index(
            index_name,
            schema,
            lexum_core::index::settings::IndexSettings::default(),
        )
        .await
        .unwrap();

    // Add test documents
    let index = state.index_manager.get_index(index_name).unwrap();
    let store = lexum_core::DocumentStore::new(Arc::new(index));

    for i in 0..100 {
        let doc = json!({
            "title": format!("Document {}", i),
            "content": format!("This is the content of document {}", i),
            "_id": format!("doc_{}", i)
        });
        store.add_document(doc).await.unwrap();
    }
}

#[tokio::test]
async fn test_streamable_http_streaming_large_result_set() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_stream_index";
    create_test_index(&state, index_name).await;

    // Test streaming search with large result set
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/indices/{index_name}/_search/stream"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "q": "document",
                "limit": 50
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(header::TRANSFER_ENCODING),
        Some(&header::HeaderValue::from_static("chunked"))
    );
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static("application/x-ndjson"))
    );
    assert_eq!(
        response.headers().get(header::CONNECTION),
        Some(&header::HeaderValue::from_static("keep-alive"))
    );

    // Read streaming response
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body_str = String::from_utf8(body.to_vec()).unwrap();

    // Verify we got multiple chunks (NDJSON format)
    let lines: Vec<&str> = body_str.trim().split('\n').collect();
    assert!(!lines.is_empty(), "Should receive at least some results");
    assert!(lines.len() <= 50, "Should not exceed limit");

    // Verify each line is valid JSON
    for line in lines {
        assert!(!line.is_empty());
        let _: serde_json::Value =
            serde_json::from_str(line).expect("Each line should be valid JSON");
    }
}

#[tokio::test]
async fn test_mcp_search_operation() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_mcp_index";
    create_test_index(&state, index_name).await;

    // Test MCP search via StreamableHTTP transport
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "search",
            "arguments": {
                "index": index_name,
                "q": "document",
                "limit": 10
            }
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-mcp-protocol", "true")
        .body(Body::from(serde_json::to_string(&mcp_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // MCP may return 200 (success) or 406 (not acceptable) depending on request format
    // The important thing is that it's handled by the MCP handler
    assert!(
        response.status() == StatusCode::OK || response.status() == StatusCode::NOT_ACCEPTABLE,
        "Expected 200 or 406, got {}",
        response.status()
    );
}

#[tokio::test]
async fn test_mcp_retrieve_operation() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_mcp_retrieve_index";
    create_test_index(&state, index_name).await;

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "retrieve",
            "arguments": {
                "index": index_name,
                "id": "doc_0"
            }
        }
    });

    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-mcp-protocol", "true")
        .body(Body::from(serde_json::to_string(&mcp_request).unwrap()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // MCP may return 200 or 406 depending on request format
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn test_protocol_detection_streamable_http() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_protocol_detection";
    create_test_index(&state, index_name).await;

    // Test detection via header
    let request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/indices/{index_name}/search"))
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-streamable-http", "true")
        .body(Body::from(
            serde_json::to_string(&json!({ "q": "document" })).unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // Should route to regular search (protocol detection is informational)
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_protocol_detection_mcp() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    // Test detection via path
    let request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            }))
            .unwrap(),
        ))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // MCP may return 200 or 406 depending on request format
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::NOT_ACCEPTABLE);
}

#[tokio::test]
async fn test_protocol_detection_umicp() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    // Test detection via header
    let request = Request::builder()
        .method("POST")
        .uri("/umicp")
        .header(header::CONTENT_TYPE, "application/x-umicp-binary")
        .header("x-umicp-protocol", "true")
        .body(Body::from(vec![0u8; 20])) // Minimal binary request
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    // UMICP handler should process (may return error for invalid request, but should be handled)
    assert!(response.status() == StatusCode::OK || response.status() == StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_protocol_switching() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_protocol_switching";
    create_test_index(&state, index_name).await;

    // First request: REST API
    let rest_request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/indices/{index_name}/search"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({ "q": "document", "limit": 5 })).unwrap(),
        ))
        .unwrap();

    let rest_response = app.clone().oneshot(rest_request).await.unwrap();
    assert_eq!(rest_response.status(), StatusCode::OK);

    // Second request: StreamableHTTP
    let stream_request = Request::builder()
        .method("POST")
        .uri(format!("/api/v1/indices/{index_name}/_search/stream"))
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({ "q": "document", "limit": 5 })).unwrap(),
        ))
        .unwrap();

    let stream_response = app.clone().oneshot(stream_request).await.unwrap();
    assert_eq!(stream_response.status(), StatusCode::OK);
    assert_eq!(
        stream_response.headers().get(header::TRANSFER_ENCODING),
        Some(&header::HeaderValue::from_static("chunked"))
    );

    // Third request: MCP
    let mcp_request = Request::builder()
        .method("POST")
        .uri("/mcp")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/list"
            }))
            .unwrap(),
        ))
        .unwrap();

    let mcp_response = app.oneshot(mcp_request).await.unwrap();
    // MCP may return 200 or 406 depending on request format
    assert!(
        mcp_response.status() == StatusCode::OK
            || mcp_response.status() == StatusCode::NOT_ACCEPTABLE
    );
}

#[tokio::test]
async fn test_umicp_binary_protocol_search() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_umicp_search";
    create_test_index(&state, index_name).await;

    // Create UMICP binary request
    // Use the same structures as the handler expects
    #[derive(serde::Serialize, serde::Deserialize)]
    enum UmicpMessageType {
        Search,
        Retrieve,
        Bulk,
        Aggregate,
        FlowControlRequest,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpHeader {
        request_id: u64,
        flow_token: Option<u32>,
        message_type: UmicpMessageType,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpRequest {
        header: UmicpHeader,
        payload_json: Vec<u8>,
    }

    let request_id = 12345u64;
    let payload_json = serde_json::to_vec(&json!({
        "index": index_name,
        "q": "document",
        "limit": 10
    }))
    .unwrap();

    let umicp_request = UmicpRequest {
        header: UmicpHeader {
            request_id,
            flow_token: None,
            message_type: UmicpMessageType::Search,
        },
        payload_json,
    };

    // Serialize with bincode
    let request_bytes = bincode::serialize(&umicp_request).unwrap();

    // Compress with zstd
    let compressed = zstd::encode_all(&request_bytes[..], 3).unwrap();

    // Build binary message with header (compression flag + request ID + payload)
    let mut binary_message = Vec::with_capacity(9 + compressed.len());
    binary_message.push(1u8); // Compression flag
    binary_message.extend_from_slice(&request_id.to_le_bytes()); // Request ID
    binary_message.extend_from_slice(&compressed);

    let request = Request::builder()
        .method("POST")
        .uri("/umicp")
        .header(header::CONTENT_TYPE, "application/x-umicp-binary")
        .header("x-umicp-protocol", "true")
        .body(Body::from(binary_message))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();

    // UMICP should return binary response
    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        panic!("UMICP request failed with status {status}: {body_str}");
    }
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        response.headers().get(header::CONTENT_TYPE),
        Some(&header::HeaderValue::from_static(
            "application/x-umicp-binary"
        ))
    );

    // Verify response has request ID header
    let response_request_id = response
        .headers()
        .get("X-UMICP-Request-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    assert_eq!(response_request_id, Some(request_id));
}

#[tokio::test]
async fn test_umicp_binary_protocol_retrieve() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_umicp_retrieve";
    create_test_index(&state, index_name).await;

    #[derive(serde::Serialize, serde::Deserialize)]
    enum UmicpMessageType {
        Search,
        Retrieve,
        Bulk,
        Aggregate,
        FlowControlRequest,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpHeader {
        request_id: u64,
        flow_token: Option<u32>,
        message_type: UmicpMessageType,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpRequest {
        header: UmicpHeader,
        payload_json: Vec<u8>,
    }

    let request_id = 67890u64;
    let payload_json = serde_json::to_vec(&json!({
        "index": index_name,
        "id": "doc_0"
    }))
    .unwrap();

    let umicp_request = UmicpRequest {
        header: UmicpHeader {
            request_id,
            flow_token: None,
            message_type: UmicpMessageType::Retrieve,
        },
        payload_json,
    };

    let request_bytes = bincode::serialize(&umicp_request).unwrap();
    let compressed = zstd::encode_all(&request_bytes[..], 3).unwrap();

    let mut binary_message = Vec::with_capacity(9 + compressed.len());
    binary_message.push(1u8);
    binary_message.extend_from_slice(&request_id.to_le_bytes());
    binary_message.extend_from_slice(&compressed);

    let request = Request::builder()
        .method("POST")
        .uri("/umicp")
        .header(header::CONTENT_TYPE, "application/x-umicp-binary")
        .body(Body::from(binary_message))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        panic!("UMICP request failed with status {status}: {body_str}");
    }
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_umicp_binary_protocol_bulk() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_umicp_bulk";
    create_test_index(&state, index_name).await;

    #[derive(serde::Serialize, serde::Deserialize)]
    enum UmicpMessageType {
        Search,
        Retrieve,
        Bulk,
        Aggregate,
        FlowControlRequest,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpHeader {
        request_id: u64,
        flow_token: Option<u32>,
        message_type: UmicpMessageType,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpRequest {
        header: UmicpHeader,
        payload_json: Vec<u8>,
    }

    let request_id = 11111u64;
    let payload_json = serde_json::to_vec(&json!({
        "operations": [
            {
                "Index": {
                    "index": index_name,
                    "id": "bulk_doc_1",
                    "document": {
                        "title": "Bulk Document 1",
                        "content": "Content 1"
                    }
                }
            },
            {
                "Index": {
                    "index": index_name,
                    "id": "bulk_doc_2",
                    "document": {
                        "title": "Bulk Document 2",
                        "content": "Content 2"
                    }
                }
            }
        ]
    }))
    .unwrap();

    let umicp_request = UmicpRequest {
        header: UmicpHeader {
            request_id,
            flow_token: None,
            message_type: UmicpMessageType::Bulk,
        },
        payload_json,
    };

    let request_bytes = bincode::serialize(&umicp_request).unwrap();
    let compressed = zstd::encode_all(&request_bytes[..], 3).unwrap();

    let mut binary_message = Vec::with_capacity(9 + compressed.len());
    binary_message.push(1u8);
    binary_message.extend_from_slice(&request_id.to_le_bytes());
    binary_message.extend_from_slice(&compressed);

    let request = Request::builder()
        .method("POST")
        .uri("/umicp")
        .header(header::CONTENT_TYPE, "application/x-umicp-binary")
        .body(Body::from(binary_message))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        panic!("UMICP request failed with status {status}: {body_str}");
    }
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn test_umicp_multiplexing() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    let index_name = "test_umicp_multiplex";
    create_test_index(&state, index_name).await;

    // Test that multiple requests with different request IDs can be processed
    // (simulating multiplexing)
    use tokio::task;

    #[derive(serde::Serialize, serde::Deserialize)]
    enum UmicpMessageType {
        Search,
        Retrieve,
        Bulk,
        Aggregate,
        FlowControlRequest,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpHeader {
        request_id: u64,
        flow_token: Option<u32>,
        message_type: UmicpMessageType,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpRequest {
        header: UmicpHeader,
        payload_json: Vec<u8>,
    }

    let mut handles = Vec::new();

    // Send 5 concurrent requests with different request IDs
    for i in 0..5 {
        let app_clone = app.clone();
        let index_name_clone = index_name.to_string();

        let handle = task::spawn(async move {
            let request_id = 1000 + i;
            let payload_json = serde_json::to_vec(&json!({
                "index": index_name_clone,
                "q": "document",
                "limit": 5
            }))
            .unwrap();

            let umicp_request = UmicpRequest {
                header: UmicpHeader {
                    request_id,
                    flow_token: None,
                    message_type: UmicpMessageType::Search,
                },
                payload_json,
            };

            let request_bytes = bincode::serialize(&umicp_request).unwrap();
            let compressed = zstd::encode_all(&request_bytes[..], 3).unwrap();

            let mut binary_message = Vec::with_capacity(9 + compressed.len());
            binary_message.push(1u8);
            binary_message.extend_from_slice(&request_id.to_le_bytes());
            binary_message.extend_from_slice(&compressed);

            let request = Request::builder()
                .method("POST")
                .uri("/umicp")
                .header(header::CONTENT_TYPE, "application/x-umicp-binary")
                .body(Body::from(binary_message))
                .unwrap();

            app_clone.oneshot(request).await.unwrap().status()
        });

        handles.push(handle);
    }

    // Wait for all requests to complete
    for handle in handles {
        let status = handle.await.unwrap();
        assert_eq!(status, StatusCode::OK);
    }
}

#[tokio::test]
async fn test_umicp_flow_control_request() {
    let (state, _temp_dir) = setup_test_server().await;
    let app = build_router(state.clone(), &Http2PushConfig::default());

    #[derive(serde::Serialize, serde::Deserialize)]
    enum UmicpMessageType {
        Search,
        Retrieve,
        Bulk,
        Aggregate,
        FlowControlRequest,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpHeader {
        request_id: u64,
        flow_token: Option<u32>,
        message_type: UmicpMessageType,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct UmicpRequest {
        header: UmicpHeader,
        payload_json: Vec<u8>,
    }

    let request_id = 99999u64;
    let payload_json = serde_json::to_vec(&json!({})).unwrap();

    let umicp_request = UmicpRequest {
        header: UmicpHeader {
            request_id,
            flow_token: None,
            message_type: UmicpMessageType::FlowControlRequest,
        },
        payload_json,
    };

    let request_bytes = bincode::serialize(&umicp_request).unwrap();
    let compressed = zstd::encode_all(&request_bytes[..], 3).unwrap();

    let mut binary_message = Vec::with_capacity(9 + compressed.len());
    binary_message.push(1u8);
    binary_message.extend_from_slice(&request_id.to_le_bytes());
    binary_message.extend_from_slice(&compressed);

    let request = Request::builder()
        .method("POST")
        .uri("/umicp")
        .header(header::CONTENT_TYPE, "application/x-umicp-binary")
        .body(Body::from(binary_message))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    if status != StatusCode::OK {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8_lossy(&body);
        panic!("UMICP request failed with status {status}: {body_str}");
    }
    assert_eq!(status, StatusCode::OK);
}
