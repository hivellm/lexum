//! Request batching middleware
//!
//! This module provides request batching functionality to group multiple
//! API requests into a single HTTP request, reducing network overhead.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use tower::Service;

/// Batch request configuration
#[derive(Debug, Clone)]
pub struct BatchRequestConfig {
    /// Maximum number of requests per batch
    pub max_batch_size: usize,
    /// Maximum total size of batch request body (bytes)
    pub max_batch_size_bytes: usize,
    /// Enable request batching
    pub enabled: bool,
}

impl Default for BatchRequestConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_size_bytes: 10 * 1024 * 1024, // 10MB
            enabled: true,
        }
    }
}

/// Individual request in a batch
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

/// Batch request statistics
#[derive(Debug, Clone, Default)]
pub struct BatchStats {
    /// Total batches processed
    pub total_batches: usize,
    /// Total requests batched
    pub total_requests: usize,
    /// Average requests per batch
    pub avg_requests_per_batch: f64,
}

impl BatchStats {
    /// Update statistics with a new batch
    pub fn record_batch(&mut self, batch_size: usize) {
        self.total_batches += 1;
        self.total_requests += batch_size;
        self.avg_requests_per_batch = self.total_requests as f64 / self.total_batches as f64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_request_config_default() {
        let config = BatchRequestConfig::default();
        assert_eq!(config.max_batch_size, 100);
        assert_eq!(config.max_batch_size_bytes, 10 * 1024 * 1024);
        assert!(config.enabled);
    }

    #[test]
    fn test_batch_request_item() {
        let item = BatchRequestItem {
            method: "GET".to_string(),
            path: "/api/v1/indices".to_string(),
            headers: HashMap::new(),
            body: None,
        };
        assert_eq!(item.method, "GET");
        assert_eq!(item.path, "/api/v1/indices");
    }

    #[test]
    fn test_batch_stats() {
        let mut stats = BatchStats::default();
        assert_eq!(stats.total_batches, 0);
        assert_eq!(stats.total_requests, 0);

        stats.record_batch(10);
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.total_requests, 10);
        assert_eq!(stats.avg_requests_per_batch, 10.0);

        stats.record_batch(20);
        assert_eq!(stats.total_batches, 2);
        assert_eq!(stats.total_requests, 30);
        assert_eq!(stats.avg_requests_per_batch, 15.0);
    }

    #[test]
    fn test_batch_request_serialization() {
        let batch = BatchRequest {
            requests: vec![
                BatchRequestItem {
                    method: "GET".to_string(),
                    path: "/api/v1/indices".to_string(),
                    headers: HashMap::new(),
                    body: None,
                },
                BatchRequestItem {
                    method: "POST".to_string(),
                    path: "/api/v1/indices/test-index/documents".to_string(),
                    headers: HashMap::new(),
                    body: Some(serde_json::json!({"title": "test"})),
                },
            ],
        };

        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: BatchRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.requests.len(), 2);
    }
}

