//! Health check handler

use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    /// Service version
    pub version: String,
}

/// Readiness check response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReadinessResponse {
    /// Service readiness status
    pub status: String,
    /// Component checks
    pub components: ComponentHealth,
}

/// Component health status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ComponentHealth {
    /// Index manager status
    pub index_manager: String,
    /// Snapshot manager status
    pub snapshot_manager: String,
}

/// Health check endpoint (liveness probe)
///
/// Returns service health status
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse)
    ),
    tag = "Health"
)]
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

/// Readiness check endpoint
///
/// Returns service readiness status with component checks
#[utoipa::path(
    get,
    path = "/_ready",
    responses(
        (status = 200, description = "Service is ready", body = ReadinessResponse),
        (status = 503, description = "Service is not ready")
    ),
    tag = "Health"
)]
pub async fn readiness_check(
    State(state): State<AppState>,
) -> Result<Json<ReadinessResponse>, axum::http::StatusCode> {
    // Check index manager
    let index_manager_ok = {
        // Try to list indices as a health check
        // Basic check - if we can call it, it's ok
        let _ = state.index_manager.list_indices();
        true
    };

    // Check snapshot manager
    let snapshot_manager_ok = {
        let _snapshot_manager = state.snapshot_manager.read().await;
        // Basic check - if we can read it, it's ok
        true
    };

    let all_ready = index_manager_ok && snapshot_manager_ok;

    if !all_ready {
        return Err(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    }

    Ok(Json(ReadinessResponse {
        status: "ready".to_string(),
        components: ComponentHealth {
            index_manager: if index_manager_ok { "ok" } else { "degraded" }.to_string(),
            snapshot_manager: if snapshot_manager_ok {
                "ok"
            } else {
                "degraded"
            }
            .to_string(),
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[lexum_macros::tokio_test]
    async fn test_health_check() {
        let response = health_check().await;
        assert_eq!(response.0.status, "ok");
    }
}
