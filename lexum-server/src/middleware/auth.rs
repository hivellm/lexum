//! Authentication middleware

use axum::extract::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;

/// API key header name
const API_KEY_HEADER: &str = "X-API-Key";

/// Authentication configuration
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// Valid API keys
    pub api_keys: Vec<String>,
    /// Whether authentication is required
    pub enabled: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_keys: vec!["dev-key-12345".to_string()],
            enabled: false, // Disabled by default for development
        }
    }
}

impl AuthConfig {
    /// Create new auth config
    pub fn new(api_keys: Vec<String>) -> Self {
        Self {
            api_keys,
            enabled: true,
        }
    }

    /// Check if API key is valid
    pub fn is_valid_key(&self, key: &str) -> bool {
        self.api_keys.iter().any(|k| k == key)
    }
}

/// Authentication middleware function
pub async fn auth_middleware(
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // For now, auth is optional - will be configurable
    // Skip auth check on health endpoint
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    // Check for API key in header
    if let Some(api_key) = headers.get(API_KEY_HEADER) {
        if let Ok(key_str) = api_key.to_str() {
            // In production, validate against stored keys
            // For now, accept any non-empty key
            if !key_str.is_empty() {
                tracing::debug!(api_key = "***", "API key authenticated");
                return Ok(next.run(request).await);
            }
        }
    }

    // If no valid key, continue anyway (auth is optional for now)
    Ok(next.run(request).await)
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
    }

    #[test]
    fn test_auth_config_new() {
        let config = AuthConfig::new(vec!["key1".to_string(), "key2".to_string()]);
        assert!(config.enabled);
        assert_eq!(config.api_keys.len(), 2);
    }

    #[test]
    fn test_is_valid_key() {
        let config = AuthConfig::new(vec!["valid-key".to_string()]);
        assert!(config.is_valid_key("valid-key"));
        assert!(!config.is_valid_key("invalid-key"));
    }

    #[test]
    fn test_validate_api_key_disabled() {
        let config = AuthConfig::default();
        let headers = HeaderMap::new();
        assert!(validate_api_key(&headers, &config));
    }

    #[test]
    fn test_validate_api_key_enabled() {
        let config = AuthConfig::new(vec!["test-key".to_string()]);

        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, "test-key".parse().unwrap());

        assert!(validate_api_key(&headers, &config));
    }

    #[test]
    fn test_validate_api_key_invalid() {
        let config = AuthConfig::new(vec!["valid-key".to_string()]);

        let mut headers = HeaderMap::new();
        headers.insert(API_KEY_HEADER, "wrong-key".parse().unwrap());

        assert!(!validate_api_key(&headers, &config));
    }

    #[test]
    fn test_validate_api_key_missing() {
        let config = AuthConfig::new(vec!["test-key".to_string()]);
        let headers = HeaderMap::new();

        assert!(!validate_api_key(&headers, &config));
    }
}
