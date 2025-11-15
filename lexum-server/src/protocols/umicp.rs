//! UMICP (Universal Microservice Communication Protocol) handler
//! Binary protocol for efficient inter-service communication with multiplexing and flow control

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use crate::handlers::search::SearchRequest;
use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use bincode;
use lexum_core::document::store::BulkOperation;
use lexum_core::types::DocumentId;
use lexum_core::{Query, SearchExecutor};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// UMICP message header for multiplexing
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UmicpHeader {
    /// Request ID for multiplexing (allows multiple concurrent requests)
    request_id: u64,
    /// Flow control token (client must have available tokens to send request)
    flow_token: Option<u32>,
    /// Message type
    message_type: UmicpMessageType,
}

/// UMICP message types
#[derive(Debug, Clone, Serialize, Deserialize)]
enum UmicpMessageType {
    /// Search operation
    Search,
    /// Retrieve document by ID
    Retrieve,
    /// Bulk operations
    Bulk,
    /// Aggregate operation
    Aggregate,
    /// Flow control request (get more tokens)
    FlowControlRequest,
}

/// UMICP request payload
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UmicpRequest {
    /// Header with multiplexing info
    header: UmicpHeader,
    /// Request payload as JSON bytes (bincode doesn't support serde_json::Value directly)
    payload_json: Vec<u8>,
}

/// UMICP response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UmicpResponse {
    /// Request ID (matches request)
    request_id: u64,
    /// Success flag
    success: bool,
    /// Response data
    data: Value,
    /// Error message if failed
    error: Option<String>,
    /// Flow control tokens granted
    flow_tokens: Option<u32>,
}

/// Flow control manager (shared state for connection)
/// Uses semaphore to limit concurrent requests per connection
struct FlowControlManager {
    /// Semaphore for flow control (max concurrent requests)
    semaphore: Arc<Semaphore>,
    /// Maximum tokens per connection
    max_tokens: u32,
}

impl FlowControlManager {
    fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            max_tokens: max_concurrent as u32,
        }
    }

    /// Acquire a token (blocking if no tokens available)
    /// Returns a guard that releases the token when dropped
    async fn acquire_token(&self) -> Result<tokio::sync::SemaphorePermit<'_>, ApiError> {
        self.semaphore
            .acquire()
            .await
            .map_err(|_| ApiError::Internal("Flow control semaphore closed".to_string()))
    }

    /// Get available tokens
    fn available_tokens(&self) -> u32 {
        self.semaphore.available_permits() as u32
    }
}

/// Global flow control manager (shared across all UMICP connections)
/// In a production system, this would be per-connection, but for simplicity
/// we use a global one with reasonable limits
static FLOW_CONTROL: std::sync::OnceLock<FlowControlManager> = std::sync::OnceLock::new();

fn get_flow_control() -> &'static FlowControlManager {
    FLOW_CONTROL.get_or_init(|| FlowControlManager::new(100)) // Max 100 concurrent requests
}

/// UMICP endpoint handler
/// Accepts binary messages with bincode serialization and optional zstd compression
/// Supports multiplexing (multiple concurrent requests) and flow control
pub async fn umicp_handler(State(state): State<AppState>, body: Body) -> ApiResult<Response> {
    let flow_control = get_flow_control();

    // Acquire flow control token (automatically released when permit is dropped)
    let _permit = flow_control.acquire_token().await?;

    // Process request (permit is automatically released when function returns)
    process_umicp_request(state, body).await
}

/// Process UMICP request
async fn process_umicp_request(state: AppState, body: Body) -> ApiResult<Response> {
    // Read body bytes
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to read body: {e}")))?;

    if bytes.is_empty() {
        return Err(ApiError::InvalidRequest("Empty request body".to_string()));
    }

    // Parse header (first 8 bytes: compression flag + request ID)
    let compression_flag = bytes[0];
    let request_id = if bytes.len() >= 9 {
        u64::from_le_bytes([
            bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7], bytes[8],
        ])
    } else {
        return Err(ApiError::InvalidRequest("Invalid UMICP header".to_string()));
    };

    // Decompress if needed
    let decompressed = if compression_flag == 1 {
        // Compressed with zstd
        zstd::decode_all(&bytes[9..])
            .map_err(|e| ApiError::InvalidRequest(format!("Failed to decompress: {e}")))?
    } else {
        // Not compressed
        bytes[9..].to_vec()
    };

    // Deserialize request with bincode
    let request: UmicpRequest = bincode::deserialize(&decompressed)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to deserialize: {e}")))?;

    // Verify request ID matches
    if request.header.request_id != request_id {
        return Err(ApiError::InvalidRequest("Request ID mismatch".to_string()));
    }

    // Deserialize JSON payload
    let payload: Value = serde_json::from_slice(&request.payload_json)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to parse JSON payload: {e}")))?;

    // Process request based on message type
    let response_data = match request.header.message_type {
        UmicpMessageType::Search => handle_search_request(&payload, &state).await?,
        UmicpMessageType::Retrieve => handle_retrieve_request(&payload, &state).await?,
        UmicpMessageType::Bulk => handle_bulk_request(&payload, &state).await?,
        UmicpMessageType::Aggregate => handle_aggregate_request(&payload, &state).await?,
        UmicpMessageType::FlowControlRequest => {
            // Return available tokens
            let flow_control = get_flow_control();
            json!({
                "available_tokens": flow_control.available_tokens(),
                "max_tokens": flow_control.max_tokens
            })
        }
    };

    // Build response
    let response = UmicpResponse {
        request_id,
        success: true,
        data: response_data,
        error: None,
        flow_tokens: Some(get_flow_control().available_tokens()),
    };

    // Serialize response with bincode
    let response_bytes = bincode::serialize(&response)
        .map_err(|e| ApiError::Internal(format!("Failed to serialize response: {e}")))?;

    // Compress response (optional - could check client preference)
    let compressed = zstd::encode_all(&response_bytes[..], 3)
        .map_err(|e| ApiError::Internal(format!("Failed to compress response: {e}")))?;

    // Build final response with header
    let mut final_response = Vec::with_capacity(9 + compressed.len());
    final_response.push(1u8); // Compression flag
    final_response.extend_from_slice(&request_id.to_le_bytes()); // Request ID
    final_response.extend_from_slice(&compressed);

    // Return binary response
    Response::builder()
        .status(200)
        .header("Content-Type", "application/x-umicp-binary")
        .header("X-UMICP-Request-ID", request_id.to_string())
        .body(Body::from(final_response))
        .map_err(|e| ApiError::Internal(format!("Failed to create response: {e}")))
}

/// Handle search request
async fn handle_search_request(payload: &Value, state: &AppState) -> Result<Value, ApiError> {
    let index_name = payload
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("Missing index".to_string()))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    if target_indices.is_empty() {
        return Err(ApiError::IndexNotFound(index_name.to_string()));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    // Parse search request
    let search_request: SearchRequest = serde_json::from_value(payload.clone())
        .map_err(|e| ApiError::InvalidRequest(format!("Invalid search request: {e}")))?;

    // Build query
    let query = if let Some(ref q) = search_request.q {
        let text_fields = index.get_text_field_names();
        if text_fields.is_empty() {
            Query::MatchAll
        } else if text_fields.len() == 1 {
            Query::Match(lexum_core::MatchQuery::new(&text_fields[0], q.clone()))
        } else {
            let mut bool_query = lexum_core::BoolQuery::new();
            for field in text_fields {
                bool_query =
                    bool_query.should(Query::Match(lexum_core::MatchQuery::new(&field, q.clone())));
            }
            Query::Bool(bool_query)
        }
    } else if let Some(ref query) = search_request.query {
        query.clone()
    } else {
        Query::MatchAll
    };

    // Execute search
    let executor = SearchExecutor::new(Arc::new(index));
    let result = executor
        .search(
            query,
            search_request.limit,
            search_request.offset,
            search_request.sort,
        )
        .await
        .map_err(|e| ApiError::Internal(format!("Search failed: {e}")))?;

    Ok(json!({
        "hits": result.hits,
        "total": result.total,
        "took": result.took_ms
    }))
}

/// Handle retrieve request
async fn handle_retrieve_request(payload: &Value, state: &AppState) -> Result<Value, ApiError> {
    let index_name = payload
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("Missing index".to_string()))?;

    let doc_id = payload
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("Missing id".to_string()))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    if target_indices.is_empty() {
        return Err(ApiError::IndexNotFound(index_name.to_string()));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    // Retrieve document
    let store = lexum_core::DocumentStore::new(Arc::new(index));
    let doc = store
        .get_document(&DocumentId::new(doc_id.to_string()))
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to retrieve document: {e}")))?;

    Ok(json!({
        "id": doc_id,
        "source": doc
    }))
}

/// Handle bulk operations request
async fn handle_bulk_request(payload: &Value, state: &AppState) -> Result<Value, ApiError> {
    let operations: Vec<BulkOperation> = payload
        .get("operations")
        .and_then(|v| serde_json::from_value::<Vec<BulkOperation>>(v.clone()).ok())
        .ok_or_else(|| ApiError::InvalidRequest("Missing or invalid operations".to_string()))?;

    if operations.is_empty() {
        return Err(ApiError::InvalidRequest(
            "Empty operations list".to_string(),
        ));
    }

    // Group operations by index
    let mut operations_by_index: HashMap<String, Vec<BulkOperation>> = HashMap::new();
    for op in operations {
        let index_name = match &op {
            BulkOperation::Index { index, .. } => index.clone(),
            BulkOperation::Update { index, .. } => index.clone(),
            BulkOperation::Delete { index, .. } => index.clone(),
        };

        operations_by_index.entry(index_name).or_default().push(op);
    }

    // Process operations for each index
    let mut all_results = Vec::new();
    let mut total_took = 0u64;
    let mut has_errors = false;

    for (index_name, ops) in operations_by_index {
        // Resolve alias
        let target_indices = state
            .index_manager
            .resolve_name(&index_name)
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        if target_indices.is_empty() {
            return Err(ApiError::IndexNotFound(index_name));
        }

        let index = state
            .index_manager
            .get_index(target_indices[0].as_str())
            .map_err(|_| ApiError::IndexNotFound(index_name.clone()))?;

        // Execute bulk operations
        let store = lexum_core::DocumentStore::new(Arc::new(index));
        let result = store
            .bulk_operations(ops)
            .await
            .map_err(|e| ApiError::Internal(format!("Bulk operations failed: {e}")))?;

        total_took += result.took;
        has_errors |= result.errors;
        all_results.extend(result.items);
    }

    Ok(json!({
        "took": total_took,
        "errors": has_errors,
        "items": all_results
    }))
}

/// Handle aggregate request
async fn handle_aggregate_request(payload: &Value, state: &AppState) -> Result<Value, ApiError> {
    let index_name = payload
        .get("index")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::InvalidRequest("Missing index".to_string()))?;

    // Resolve alias
    let target_indices = state
        .index_manager
        .resolve_name(index_name)
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    if target_indices.is_empty() {
        return Err(ApiError::IndexNotFound(index_name.to_string()));
    }

    let index = state
        .index_manager
        .get_index(target_indices[0].as_str())
        .map_err(|_| ApiError::IndexNotFound(index_name.to_string()))?;

    // Build query
    let query = if let Some(query_obj) = payload.get("query") {
        serde_json::from_value(query_obj.clone())
            .map_err(|e| ApiError::InvalidRequest(format!("Invalid query: {e}")))?
    } else {
        Query::MatchAll
    };

    // Parse aggregations
    let aggregations = if let Some(aggs_obj) =
        payload.get("aggregations").and_then(|v| v.as_object())
    {
        let mut agg_specs = Vec::new();
        for (name, spec) in aggs_obj {
            match serde_json::from_value::<lexum_core::aggregation::AggregationSpec>(spec.clone()) {
                Ok(agg_spec) => agg_specs.push(agg_spec),
                Err(e) => {
                    return Err(ApiError::InvalidRequest(format!(
                        "Invalid aggregation '{name}': {e}"
                    )));
                }
            }
        }
        Some(agg_specs)
    } else {
        None
    };

    // Execute search with aggregations
    let executor = SearchExecutor::new(Arc::new(index));
    let result = executor
        .search_with_aggregations(query, 0, 0, None, aggregations.as_deref())
        .await
        .map_err(|e| ApiError::Internal(format!("Aggregation failed: {e}")))?;

    Ok(json!({
        "aggregations": result.aggregations,
        "total": result.total
    }))
}
