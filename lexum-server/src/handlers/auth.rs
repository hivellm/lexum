//! Authentication and API key management handlers

use crate::error::{ApiError, ApiResult};
use crate::handlers::index::AppState;
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request to generate a new API key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateApiKeyRequest {
    /// Optional description for the API key
    #[serde(default)]
    pub description: Option<String>,
    /// Optional expiration time in seconds (0 = no expiration)
    #[serde(default)]
    pub expires_in: Option<u64>,
}

/// Response containing the generated API key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GenerateApiKeyResponse {
    /// The generated API key (only shown once)
    pub api_key: String,
    /// Key identifier
    pub key_id: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Expiration timestamp (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// Request to revoke an API key
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeApiKeyRequest {
    /// API key to revoke
    pub api_key: String,
}

/// Response for API key revocation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RevokeApiKeyResponse {
    /// Whether the key was successfully revoked
    pub revoked: bool,
    /// Message
    pub message: String,
}

/// List of API keys (without exposing the actual keys)
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ApiKeyInfo {
    /// Key identifier
    pub key_id: String,
    /// Key prefix (first 8 characters)
    pub prefix: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Creation timestamp
    pub created_at: String,
    /// Expiration timestamp (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

/// List API keys response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListApiKeysResponse {
    /// List of API keys
    pub keys: Vec<ApiKeyInfo>,
    /// Total number of keys
    pub total: usize,
}

/// Generate a new API key
#[utoipa::path(
    post,
    path = "/api/v1/auth/keys",
    request_body = GenerateApiKeyRequest,
    responses(
        (status = 201, description = "API key generated successfully", body = GenerateApiKeyResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Authentication"
)]
pub async fn generate_api_key(
    State(state): State<AppState>,
    Json(request): Json<GenerateApiKeyRequest>,
) -> ApiResult<(StatusCode, Json<GenerateApiKeyResponse>)> {
    // Generate a secure API key
    let key_id = Uuid::new_v4().to_string();
    let api_key = format!(
        "lexum_{}_{}",
        key_id,
        Uuid::new_v4().to_string().replace('-', "")
    );

    // Add the key to the auth config
    state
        .auth_state
        .update_config(|config| {
            config.add_api_key(api_key.clone());
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to add API key: {e}")))?;

    // Calculate expiration if provided
    let expires_at = request.expires_in.map(|seconds| {
        let expiration = chrono::Utc::now() + chrono::Duration::seconds(seconds as i64);
        expiration.to_rfc3339()
    });

    let response = GenerateApiKeyResponse {
        api_key: api_key.clone(),
        key_id: key_id.clone(),
        description: request.description.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        expires_at,
    };

    tracing::info!(
        key_id = %key_id,
        "API key generated successfully"
    );

    Ok((StatusCode::CREATED, Json(response)))
}

/// Revoke an API key
#[utoipa::path(
    delete,
    path = "/api/v1/auth/keys",
    request_body = RevokeApiKeyRequest,
    responses(
        (status = 200, description = "API key revoked successfully", body = RevokeApiKeyResponse),
        (status = 400, description = "Invalid request", body = ApiError),
        (status = 404, description = "API key not found", body = ApiError),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Authentication"
)]
pub async fn revoke_api_key(
    State(state): State<AppState>,
    Json(request): Json<RevokeApiKeyRequest>,
) -> ApiResult<Json<RevokeApiKeyResponse>> {
    let mut revoked = false;
    state
        .auth_state
        .update_config(|config| {
            revoked = config.remove_api_key(&request.api_key);
        })
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to revoke API key: {e}")))?;

    if revoked {
        tracing::info!("API key revoked successfully");
        Ok(Json(RevokeApiKeyResponse {
            revoked: true,
            message: "API key revoked successfully".to_string(),
        }))
    } else {
        Err(ApiError::InvalidRequest("API key not found".to_string()))
    }
}

/// List all API keys (without exposing the actual keys)
#[utoipa::path(
    get,
    path = "/api/v1/auth/keys",
    responses(
        (status = 200, description = "List of API keys", body = ListApiKeysResponse),
        (status = 500, description = "Internal server error", body = ApiError)
    ),
    tag = "Authentication"
)]
pub async fn list_api_keys(State(state): State<AppState>) -> ApiResult<Json<ListApiKeysResponse>> {
    let config = state.auth_state.get_config().await;
    let keys: Vec<ApiKeyInfo> = config
        .list_api_keys()
        .into_iter()
        .map(|key| {
            let key_id = if key.starts_with("lexum_") {
                key.split('_').nth(1).unwrap_or("unknown").to_string()
            } else {
                Uuid::new_v4().to_string() // Generate ID for legacy keys
            };
            ApiKeyInfo {
                key_id: key_id.clone(),
                prefix: if key.len() >= 8 {
                    key[..8].to_string()
                } else {
                    "****".to_string()
                },
                description: None,
                created_at: chrono::Utc::now().to_rfc3339(), // Approximate for existing keys
                expires_at: None,
            }
        })
        .collect();

    Ok(Json(ListApiKeysResponse {
        total: keys.len(),
        keys,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::index::AppState;

    fn create_test_app_state() -> AppState {
        AppState::default()
    }

    #[lexum_macros::tokio_test]
    async fn test_generate_api_key() {
        let state = create_test_app_state();
        let request = GenerateApiKeyRequest {
            description: Some("Test key".to_string()),
            expires_in: Some(3600),
        };

        let result = generate_api_key(State(state.clone()), Json(request)).await;
        assert!(result.is_ok());

        let (status, response) = result.unwrap();
        assert_eq!(status, StatusCode::CREATED);
        assert!(response.api_key.starts_with("lexum_"));
        assert!(!response.key_id.is_empty());
        assert_eq!(response.description, Some("Test key".to_string()));
        assert!(response.expires_at.is_some());
    }

    #[lexum_macros::tokio_test]
    async fn test_revoke_api_key() {
        let state = create_test_app_state();

        // First generate a key
        let generate_request = GenerateApiKeyRequest {
            description: None,
            expires_in: None,
        };
        let (_, generate_response) = generate_api_key(State(state.clone()), Json(generate_request))
            .await
            .unwrap();

        // Then revoke it
        let revoke_request = RevokeApiKeyRequest {
            api_key: generate_response.api_key.clone(),
        };
        let result = revoke_api_key(State(state.clone()), Json(revoke_request)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.revoked);
        assert_eq!(response.message, "API key revoked successfully");
    }

    #[lexum_macros::tokio_test]
    async fn test_revoke_nonexistent_key() {
        let state = create_test_app_state();
        let request = RevokeApiKeyRequest {
            api_key: "nonexistent-key".to_string(),
        };

        let result = revoke_api_key(State(state), Json(request)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            ApiError::InvalidRequest(msg) => {
                assert!(msg.contains("not found"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[lexum_macros::tokio_test]
    async fn test_list_api_keys() {
        let state = create_test_app_state();

        // Generate a few keys
        for _ in 0..3 {
            let request = GenerateApiKeyRequest {
                description: None,
                expires_in: None,
            };
            let _ = generate_api_key(State(state.clone()), Json(request)).await;
        }

        let result = list_api_keys(State(state)).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.total >= 3); // At least 3 keys (may have default keys)
        assert_eq!(response.keys.len(), response.total);
    }
}
