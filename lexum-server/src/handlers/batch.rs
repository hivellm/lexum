//! Request batching handler
//!
//! This handler processes batch requests, executing multiple API requests
//! in a single HTTP call.

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::http::{Method, StatusCode};
use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::handlers::document;

/// Batch request item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestItem {
    /// HTTP method
    pub method: String,
    /// Request path
    pub path: String,
    /// Request headers (optional)
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Request body (optional)
    pub body: Option<Value>,
}

/// Batch request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequest {
    /// List of requests to execute
    pub requests: Vec<BatchRequestItem>,
}

/// Response for a single batched request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponseItem {
    /// HTTP status code
    pub status: u16,
    /// Response headers
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Response body
    pub body: Value,
}

/// Batch response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResponse {
    /// Responses for each request in order
    pub responses: Vec<BatchResponseItem>,
}

/// Batch request handler
///
/// Executes multiple API requests in a single HTTP call.
/// This reduces network overhead by batching multiple operations.
#[utoipa::path(
    post,
    path = "/api/v1/_batch",
    request_body = BatchRequest,
    responses(
        (status = 200, description = "Batch request completed", body = BatchResponse),
        (status = 400, description = "Invalid batch request")
    ),
    tag = "Batch"
)]
pub async fn batch_requests(
    State(state): State<AppState>,
    Json(request): Json<BatchRequest>,
) -> ApiResult<Json<BatchResponse>> {
    // Validate batch size
    if request.requests.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Batch request must contain at least one request".to_string(),
        ));
    }

    if request.requests.len() > 100 {
        return Err(ApiError::InvalidRequest(
            "Batch request cannot contain more than 100 requests".to_string(),
        ));
    }

    let mut responses = Vec::new();

    // Execute each request in the batch
    for item in request.requests {
        let response = execute_batch_item(&state, item).await;
        responses.push(response);
    }

    Ok(Json(BatchResponse { responses }))
}

/// Execute a single batch request item
async fn execute_batch_item(
    state: &AppState,
    item: BatchRequestItem,
) -> BatchResponseItem {
    // Parse method
    let method = match item.method.as_str() {
        "GET" => Method::GET,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "DELETE" => Method::DELETE,
        _ => {
            return BatchResponseItem {
                status: StatusCode::METHOD_NOT_ALLOWED.as_u16(),
                headers: HashMap::new(),
                body: serde_json::json!({
                    "error": format!("Unsupported method: {}", item.method)
                }),
            };
        }
    };

    // Route to appropriate handler based on path
    let result = match item.path.as_str() {
        path if path.starts_with("/api/v1/indices") && method == Method::GET && path.ends_with("/stats") => {
            // Get index stats
            if let Some(index_name) = extract_index_name(path) {
                match state.index_manager.get_index_stats(&index_name).await {
                    Ok(stats) => {
                        let body = serde_json::json!({
                            "name": stats.name,
                            "num_docs": stats.num_docs
                        });
                        Ok((StatusCode::OK, body))
                    }
                    Err(_) => Err(ApiError::IndexNotFound(index_name)),
                }
            } else {
                Err(ApiError::InvalidRequest("Invalid index path".to_string()))
            }
        }
        path if path == "/api/v1/indices" && method == Method::GET => {
            // List indices
            let index_names = state.index_manager.list_indices();
            let mut index_infos = Vec::new();
            for name in index_names {
                if let Ok(stats) = state.index_manager.get_index_stats(&name).await {
                    index_infos.push(serde_json::json!({
                        "name": stats.name,
                        "num_docs": stats.num_docs
                    }));
                }
            }
            Ok((StatusCode::OK, serde_json::json!({ "indices": index_infos })))
        }
        path if path.starts_with("/api/v1/indices/") && path.ends_with("/documents") && method == Method::POST => {
            // Add document
            if let Some(index_name) = extract_index_name(path) {
                if let Some(body) = item.body {
                    match document::add_document_internal(state, &index_name, body).await {
                        Ok(response) => Ok((StatusCode::CREATED, serde_json::json!(response))),
                        Err(e) => Err(e),
                    }
                } else {
                    Err(ApiError::InvalidRequest("Missing request body".to_string()))
                }
            } else {
                Err(ApiError::InvalidRequest("Invalid index path".to_string()))
            }
        }
        _ => {
            // Unsupported path
            Err(ApiError::InvalidRequest(format!(
                "Unsupported batch path: {} {}",
                item.method, item.path
            )))
        }
    };

    match result {
        Ok((status, body)) => BatchResponseItem {
            status: status.as_u16(),
            headers: HashMap::new(),
            body,
        },
        Err(e) => BatchResponseItem {
            status: e.status_code().as_u16(),
            headers: HashMap::new(),
            body: serde_json::json!({
                "error": e.to_string()
            }),
        },
    }
}

/// Extract index name from path
fn extract_index_name(path: &str) -> Option<String> {
    // Path format: /api/v1/indices/{name}/...
    let parts: Vec<&str> = path.split('/').collect();
    if parts.len() >= 4 && parts[1] == "api" && parts[2] == "v1" && parts[3] == "indices" {
        if parts.len() > 4 {
            Some(parts[4].to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Internal helper for adding documents (used by batch handler)
async fn add_document_internal(
    state: &AppState,
    index_name: &str,
    body: Value,
) -> Result<document::AddDocumentResponse, ApiError> {
    use lexum_core::DocumentStore;

    let index = state
        .index_manager
        .get_index(index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    let store = DocumentStore::new(std::sync::Arc::new(index));
    let doc_id = store.add_document(body).await?;

    Ok(document::AddDocumentResponse {
        id: doc_id.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_index_name() {
        assert_eq!(
            extract_index_name("/api/v1/indices/test-index/stats"),
            Some("test-index".to_string())
        );
        assert_eq!(
            extract_index_name("/api/v1/indices/my-index/documents"),
            Some("my-index".to_string())
        );
        assert_eq!(extract_index_name("/api/v1/indices"), None);
        assert_eq!(extract_index_name("/invalid/path"), None);
    }
}

