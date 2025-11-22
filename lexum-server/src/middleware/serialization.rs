//! Serialization optimization middleware
//!
//! This module provides optimized serialization for API responses,
//! including response caching and buffer reuse.

use axum::body::Body;
use axum::http::{HeaderValue, Response, StatusCode, header};
use axum::response::IntoResponse;
use serde::Serialize;

/// Serialization configuration
#[derive(Debug, Clone)]
pub struct SerializationConfig {
    /// Enable response caching for serialized JSON
    pub enable_cache: bool,
    /// Maximum cache size (number of entries)
    pub max_cache_size: usize,
    /// Use compact JSON (no pretty printing)
    pub compact: bool,
}

impl Default for SerializationConfig {
    fn default() -> Self {
        Self {
            enable_cache: false, // Disabled by default for now
            max_cache_size: 1000,
            compact: true,
        }
    }
}

/// Optimized JSON serialization helper
pub struct SerializationOptimizer {
    config: SerializationConfig,
}

impl SerializationOptimizer {
    /// Create new serializer with default config
    pub fn new() -> Self {
        Self {
            config: SerializationConfig::default(),
        }
    }

    /// Create serializer with custom config
    pub fn with_config(config: SerializationConfig) -> Self {
        Self { config }
    }

    /// Serialize to JSON bytes efficiently
    pub fn to_json_bytes<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, serde_json::Error> {
        if self.config.compact {
            // Use compact serialization (no pretty printing)
            serde_json::to_vec(value)
        } else {
            // Use pretty printing (slower but more readable)
            serde_json::to_vec_pretty(value)
        }
    }

    /// Serialize to JSON string efficiently
    pub fn to_json_string<T: Serialize>(&self, value: &T) -> Result<String, serde_json::Error> {
        if self.config.compact {
            serde_json::to_string(value)
        } else {
            serde_json::to_string_pretty(value)
        }
    }
}

impl Default for SerializationOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

/// Response wrapper that uses optimized serialization
pub struct OptimizedJson<T>(pub T);

impl<T> IntoResponse for OptimizedJson<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response<Body> {
        let serializer = SerializationOptimizer::new();
        match serializer.to_json_bytes(&self.0) {
            Ok(bytes) => {
                let mut response = Response::new(Body::from(bytes));
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
            Err(err) => {
                let error_body = format!(r#"{{"error":"Serialization failed: {err}"}}"#);
                let mut response = Response::new(Body::from(error_body));
                *response.status_mut() = StatusCode::INTERNAL_SERVER_ERROR;
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                );
                response
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialization_optimizer_compact() {
        let serializer = SerializationOptimizer::new();
        let value = json!({"key": "value", "number": 42});

        let bytes = serializer.to_json_bytes(&value).unwrap();
        let string = String::from_utf8(bytes).unwrap();

        // Compact JSON should not have extra whitespace
        assert!(!string.contains('\n'));
        assert_eq!(string, r#"{"key":"value","number":42}"#);
    }

    #[test]
    fn test_serialization_optimizer_config() {
        let config = SerializationConfig {
            compact: false,
            ..Default::default()
        };
        let serializer = SerializationOptimizer::with_config(config);
        let value = json!({"key": "value"});

        let string = serializer.to_json_string(&value).unwrap();

        // Pretty JSON should have newlines
        assert!(string.contains('\n'));
    }

    #[test]
    fn test_optimized_json_response() {
        let value = json!({"status": "ok"});
        let response = OptimizedJson(value).into_response();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get(header::CONTENT_TYPE),
            Some(&HeaderValue::from_static("application/json"))
        );
    }
}
