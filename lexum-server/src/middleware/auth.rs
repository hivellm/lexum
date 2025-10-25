//! Authentication middleware

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{Json, Response};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, warn};

/// API key header name
const API_KEY_HEADER: &str = "X-API-Key";
const AUTHORIZATION_HEADER: &str = "Authorization";

/// Authentication configuration
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// Valid API keys
    pub api_keys: HashSet<String>,
    /// Whether authentication is required
    pub enabled: bool,
    /// Whether to allow anonymous access to certain endpoints
    pub allow_anonymous: bool,
    /// Anonymous endpoints (paths that don't require auth)
    pub anonymous_endpoints: HashSet<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        let mut api_keys = HashSet::new();
        api_keys.insert("dev-key-12345".to_string());
        api_keys.insert("admin-key-67890".to_string());

        let mut anonymous_endpoints = HashSet::new();
        anonymous_endpoints.insert("/health".to_string());
        anonymous_endpoints.insert("/swagger-ui".to_string());
        anonymous_endpoints.insert("/api-docs".to_string());
        anonymous_endpoints.insert("/metrics".to_string());

        Self {
            api_keys,
            enabled: false, // Disabled by default for development
            allow_anonymous: true,
            anonymous_endpoints,
        }
    }
}

impl AuthConfig {
    /// Create new auth config
    pub fn new(api_keys: Vec<String>) -> Self {
        let mut keys = HashSet::new();
        for key in api_keys {
            keys.insert(key);
        }

        let mut anonymous_endpoints = HashSet::new();
        anonymous_endpoints.insert("/health".to_string());
        anonymous_endpoints.insert("/swagger-ui".to_string());
        anonymous_endpoints.insert("/api-docs".to_string());
        anonymous_endpoints.insert("/metrics".to_string());

        Self {
            api_keys: keys,
            enabled: true,
            allow_anonymous: false,
            anonymous_endpoints,
        }
    }

    /// Create auth config from environment variables
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Check if auth is enabled via environment
        if let Ok(enabled) = std::env::var("LEXUM_AUTH_ENABLED") {
            config.enabled = enabled.parse().unwrap_or(false);
        }

        // Load API keys from environment
        if let Ok(keys) = std::env::var("LEXUM_API_KEYS") {
            let keys: Vec<String> = keys.split(',').map(|s| s.trim().to_string()).collect();
            config.api_keys = keys.into_iter().collect();
        }

        // Check if anonymous access is allowed
        if let Ok(allow_anonymous) = std::env::var("LEXUM_ALLOW_ANONYMOUS") {
            config.allow_anonymous = allow_anonymous.parse().unwrap_or(true);
        }

        config
    }

    /// Check if API key is valid
    pub fn is_valid_key(&self, key: &str) -> bool {
        self.api_keys.contains(key)
    }

    /// Check if endpoint allows anonymous access
    pub fn is_anonymous_endpoint(&self, path: &str) -> bool {
        self.anonymous_endpoints.contains(path)
            || path.starts_with("/swagger-ui")
            || path.starts_with("/api-docs")
            || path.starts_with("/metrics")
    }

    /// Add API key
    pub fn add_api_key(&mut self, key: String) {
        self.api_keys.insert(key);
    }

    /// Remove API key
    pub fn remove_api_key(&mut self, key: &str) -> bool {
        self.api_keys.remove(key)
    }

    /// List all API keys (for admin purposes)
    pub fn list_api_keys(&self) -> Vec<String> {
        self.api_keys.iter().cloned().collect()
    }
}

/// Authentication state for the application
#[derive(Clone)]
pub struct AuthState {
    /// Authentication configuration
    pub config: Arc<RwLock<AuthConfig>>,
}

impl AuthState {
    /// Create a new authentication state
    pub fn new(config: AuthConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Get the current authentication configuration
    pub async fn get_config(&self) -> AuthConfig {
        self.config.read().await.clone()
    }

    /// Update the authentication configuration
    pub async fn update_config<F>(&self, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut AuthConfig),
    {
        let mut config = self.config.write().await;
        f(&mut config);
        Ok(())
    }
}

/// Authentication middleware function
pub async fn auth_middleware(
    State(auth_state): State<AuthState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let config = auth_state.get_config().await;

    // If auth is disabled, allow all requests
    if !config.enabled {
        return Ok(next.run(request).await);
    }

    let path = request.uri().path();

    // Check if this is an anonymous endpoint
    if config.is_anonymous_endpoint(path) {
        debug!("Anonymous endpoint accessed: {}", path);
        return Ok(next.run(request).await);
    }

    // Extract API key from headers
    let api_key = extract_api_key(&headers);

    match api_key {
        Some(key) => {
            if config.is_valid_key(&key) {
                debug!(api_key = "***", "API key authenticated for path: {}", path);
                Ok(next.run(request).await)
            } else {
                error!("Invalid API key provided for path: {}", path);
                Err(StatusCode::UNAUTHORIZED)
            }
        }
        None => {
            if config.allow_anonymous {
                warn!("No API key provided for protected endpoint: {}", path);
                Ok(next.run(request).await)
            } else {
                error!("No API key provided for protected endpoint: {}", path);
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

/// Extract API key from request headers
fn extract_api_key(headers: &HeaderMap) -> Option<String> {
    // Try X-API-Key header first
    if let Some(api_key) = headers.get(API_KEY_HEADER) {
        if let Ok(key_str) = api_key.to_str() {
            if !key_str.is_empty() {
                return Some(key_str.to_string());
            }
        }
    }

    // Try Authorization header with Bearer token
    if let Some(auth_header) = headers.get(AUTHORIZATION_HEADER) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }

    None
}

/// Create authentication error response
pub fn create_auth_error_response(message: &str) -> Json<Value> {
    Json(json!({
        "error": {
            "type": "authentication_error",
            "message": message,
            "status": 401
        }
    }))
}

/// Create unauthorized response
pub fn create_unauthorized_response() -> (StatusCode, Json<Value>) {
    (
        StatusCode::UNAUTHORIZED,
        create_auth_error_response("Authentication required"),
    )
}

/// Check if request has valid API key
pub fn validate_api_key(headers: &HeaderMap, config: &AuthConfig) -> bool {
    if !config.enabled {
        return true;
    }

    if let Some(api_key) = headers.get(API_KEY_HEADER) {
        if let Ok(key_str) = api_key.to_str() {
            return config.is_valid_key(key_str);
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert!(!config.enabled); // Disabled by default
        assert!(!config.api_keys.is_empty());
        assert!(config.allow_anonymous);
    }

    #[test]
    fn test_auth_config_new() {
        let config = AuthConfig::new(vec!["key1".to_string(), "key2".to_string()]);
        assert!(config.enabled);
        assert_eq!(config.api_keys.len(), 2);
        assert!(!config.allow_anonymous);
    }

    #[test]
    fn test_auth_config_from_env() {
        unsafe {
            std::env::set_var("LEXUM_AUTH_ENABLED", "true");
            std::env::set_var("LEXUM_API_KEYS", "env-key1,env-key2,env-key3");
            std::env::set_var("LEXUM_ALLOW_ANONYMOUS", "false");
        }

        let config = AuthConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.api_keys.len(), 3);
        assert!(config.api_keys.contains("env-key1"));
        assert!(config.api_keys.contains("env-key2"));
        assert!(config.api_keys.contains("env-key3"));
        assert!(!config.allow_anonymous);

        // Clean up
        unsafe {
            std::env::remove_var("LEXUM_AUTH_ENABLED");
            std::env::remove_var("LEXUM_API_KEYS");
            std::env::remove_var("LEXUM_ALLOW_ANONYMOUS");
        }
    }

    #[test]
    fn test_is_valid_key() {
        let config = AuthConfig::new(vec!["valid-key".to_string()]);
        assert!(config.is_valid_key("valid-key"));
        assert!(!config.is_valid_key("invalid-key"));
    }

    #[test]
    fn test_is_anonymous_endpoint() {
        let config = AuthConfig::default();
        assert!(config.is_anonymous_endpoint("/health"));
        assert!(config.is_anonymous_endpoint("/swagger-ui"));
        assert!(config.is_anonymous_endpoint("/api-docs/openapi.json"));
        assert!(config.is_anonymous_endpoint("/metrics"));
        assert!(!config.is_anonymous_endpoint("/search"));
        assert!(!config.is_anonymous_endpoint("/indices"));
    }

    #[test]
    fn test_add_remove_api_key() {
        let mut config = AuthConfig::new(vec!["key1".to_string()]);
        assert_eq!(config.api_keys.len(), 1);

        config.add_api_key("key2".to_string());
        assert_eq!(config.api_keys.len(), 2);
        assert!(config.is_valid_key("key2"));

        assert!(config.remove_api_key("key1"));
        assert_eq!(config.api_keys.len(), 1);
        assert!(!config.is_valid_key("key1"));

        assert!(!config.remove_api_key("nonexistent"));
    }

    #[test]
    fn test_list_api_keys() {
        let config = AuthConfig::new(vec!["key1".to_string(), "key2".to_string()]);
        let keys = config.list_api_keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_x_api_key_header() {
        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, "test-key".parse().unwrap());

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_from_authorization_header() {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION_HEADER, "Bearer test-token".parse().unwrap());

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_api_key_priority() {
        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, "x-api-key".parse().unwrap());
        headers.insert(AUTHORIZATION_HEADER, "Bearer bearer-token".parse().unwrap());

        let api_key = extract_api_key(&headers);
        // X-API-Key should take priority
        assert_eq!(api_key, Some("x-api-key".to_string()));
    }

    #[test]
    fn test_extract_api_key_empty() {
        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, "".parse().unwrap());

        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, None);
    }

    #[test]
    fn test_extract_api_key_missing() {
        let headers = HeaderMap::new();
        let api_key = extract_api_key(&headers);
        assert_eq!(api_key, None);
    }

    #[test]
    fn test_auth_state() {
        let config = AuthConfig::new(vec!["test-key".to_string()]);
        let auth_state = AuthState::new(config);

        // Test getting config
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = auth_state.get_config().await;
            assert!(config.enabled);
            assert_eq!(config.api_keys.len(), 1);
        });
    }

    #[test]
    fn test_create_auth_error_response() {
        let response = create_auth_error_response("Test error");
        let json = response.0; // Extract inner Value from Json<Value>

        assert_eq!(json["error"]["type"], "authentication_error");
        assert_eq!(json["error"]["message"], "Test error");
        assert_eq!(json["error"]["status"], 401);
    }

    #[test]
    fn test_create_unauthorized_response() {
        let (status, response) = create_unauthorized_response();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        let json = response.0; // Extract inner Value from Json<Value>
        assert_eq!(json["error"]["type"], "authentication_error");
        assert_eq!(json["error"]["message"], "Authentication required");
    }
}
