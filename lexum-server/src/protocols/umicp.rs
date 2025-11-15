//! UMICP (Universal Microservice Communication Protocol) handler
//! Binary protocol for efficient inter-service communication

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::response::Json;
use bincode;
use serde_json::{Value, json};

/// UMICP endpoint handler
/// Accepts binary messages with bincode serialization and optional zstd compression
pub async fn umicp_handler(State(_state): State<AppState>, body: Body) -> ApiResult<Json<Value>> {
    // Read body bytes
    let bytes = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to read body: {e}")))?;

    // Check if compressed (first byte indicates compression)
    let decompressed = if bytes.is_empty() {
        return Err(ApiError::InvalidRequest("Empty request body".to_string()));
    } else if bytes[0] == 1 {
        // Compressed with zstd
        zstd::decode_all(&bytes[1..])
            .map_err(|e| ApiError::InvalidRequest(format!("Failed to decompress: {e}")))?
    } else {
        // Not compressed
        bytes.to_vec()
    };

    // Deserialize with bincode
    let request: Value = bincode::deserialize(&decompressed)
        .map_err(|e| ApiError::InvalidRequest(format!("Failed to deserialize: {e}")))?;

    // Process request (for now, just echo back - full implementation would route to handlers)
    let response = json!({
        "status": "ok",
        "message": "UMICP request received",
        "request": request
    });

    Ok(Json(response))
}
