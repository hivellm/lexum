//! HTTP/2 Push and Resource Preloading
//!
//! This module implements HTTP/2 push hints using Link headers with preload hints.
//! While HTTP/2 Server Push has been deprecated, we use Link headers to suggest
//! resources that clients can prefetch/preload for better performance.

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::response::Response;
use std::sync::Arc;
use tower::Layer;
use tower::Service;

/// Configuration for HTTP/2 push/preload hints
#[derive(Debug, Clone)]
pub struct Http2PushConfig {
    /// Enable HTTP/2 push hints
    pub enabled: bool,
    /// Maximum number of resources to push/preload per request
    pub max_resources: usize,
    /// Push hints for search endpoints (related indices, templates, etc.)
    pub enable_search_hints: bool,
    /// Push hints for document endpoints (related documents, index info)
    pub enable_document_hints: bool,
    /// Push hints for index endpoints (related templates, aliases)
    pub enable_index_hints: bool,
}

impl Default for Http2PushConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_resources: 5,
            enable_search_hints: true,
            enable_document_hints: true,
            enable_index_hints: true,
        }
    }
}

/// HTTP/2 Push Layer
#[derive(Clone)]
pub struct Http2PushLayer {
    config: Arc<Http2PushConfig>,
}

impl Http2PushLayer {
    /// Create new HTTP/2 Push layer
    pub fn new(config: Http2PushConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }
}

impl<S> Layer<S> for Http2PushLayer {
    type Service = Http2PushService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        Http2PushService {
            inner,
            config: Arc::clone(&self.config),
        }
    }
}

/// HTTP/2 Push Service
#[derive(Clone)]
pub struct Http2PushService<S> {
    inner: S,
    config: Arc<Http2PushConfig>,
}

impl<S> Service<Request> for Http2PushService<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        if !self.config.enabled {
            return Box::pin(self.inner.call(req));
        }

        let mut service = self.inner.clone();
        let config = Arc::clone(&self.config);
        let uri = req.uri().clone();

        Box::pin(async move {
            let mut response = service.call(req).await?;

            // Generate push hints based on the request path
            let hints = generate_push_hints(uri.path(), &config);

            if !hints.is_empty() {
                // Add Link headers for preload hints
                let headers = response.headers_mut();
                for hint in hints.iter().take(config.max_resources) {
                    if let Ok(link_value) = HeaderValue::from_str(hint) {
                        headers.append("Link", link_value);
                    }
                }
            }

            Ok(response)
        })
    }
}

/// Generate push hints based on request path
fn generate_push_hints(path: &str, config: &Http2PushConfig) -> Vec<String> {
    let mut hints = Vec::new();

    // Search endpoint hints
    if config.enable_search_hints && path.contains("/search") {
        // Suggest related endpoints that might be needed
        if let Some(index_name) = extract_index_from_path(path) {
            // Push index info endpoint
            hints.push(format!(
                r"</api/v1/indices/{index_name}>; rel=preload; as=fetch"
            ));
            // Push index stats endpoint
            hints.push(format!(
                r"</api/v1/indices/{index_name}/stats>; rel=preload; as=fetch"
            ));
            // Push template endpoint if available
            hints.push(format!(r"</_template/{index_name}>; rel=preload; as=fetch"));
        }
    }

    // Document endpoint hints
    if config.enable_document_hints && path.contains("/documents") {
        if let Some(index_name) = extract_index_from_path(path) {
            // Push index info
            hints.push(format!(
                r"</api/v1/indices/{index_name}>; rel=preload; as=fetch"
            ));
            // Push search endpoint for related documents
            hints.push(format!(
                r"</api/v1/indices/{index_name}/search>; rel=preload; as=fetch"
            ));
        }
    }

    // Index endpoint hints
    if config.enable_index_hints && path.contains("/indices") && !path.contains("/search") {
        if let Some(index_name) = extract_index_from_path(path) {
            // Push template if exists
            hints.push(format!(r"</_template/{index_name}>; rel=preload; as=fetch"));
            // Push aliases
            hints.push(format!(r"</{index_name}/_alias>; rel=preload; as=fetch"));
            // Push stats
            hints.push(format!(
                r"</api/v1/indices/{index_name}/stats>; rel=preload; as=fetch"
            ));
        }
    }

    hints
}

/// Extract index name from path
fn extract_index_from_path(path: &str) -> Option<String> {
    // Patterns: /api/v1/indices/{index}/..., /{index}/_alias, etc.
    let parts: Vec<&str> = path.split('/').collect();

    // Try to find index name after /indices/
    if let Some(indices_pos) = parts.iter().position(|&p| p == "indices") {
        if indices_pos + 1 < parts.len() {
            let index_name = parts[indices_pos + 1];
            // Skip if it's a reserved word
            if ["search", "documents", "stats", "refresh", "flush"].contains(&index_name) {
                return None;
            }
            // Skip if the next segment is a reserved command (like _rollover)
            if indices_pos + 2 < parts.len() {
                let next_segment = parts[indices_pos + 2];
                if next_segment.starts_with('_') && ["_rollover", "_alias"].contains(&next_segment)
                {
                    return None;
                }
            }
            // Valid index name found
            if !index_name.starts_with('_') {
                return Some(index_name.to_string());
            }
        }
    }

    // Try to find index name at the beginning (e.g., /{index}/_alias)
    if parts.len() >= 2
        && !parts[1].is_empty()
        && !parts[1].starts_with('_')
        && !parts[1].starts_with("api")
    {
        return Some(parts[1].to_string());
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_index_from_path() {
        assert_eq!(
            extract_index_from_path("/api/v1/indices/my-index/search"),
            Some("my-index".to_string())
        );
        assert_eq!(
            extract_index_from_path("/api/v1/indices/test-index/documents/123"),
            Some("test-index".to_string())
        );
        assert_eq!(
            extract_index_from_path("/my-index/_alias"),
            Some("my-index".to_string())
        );
        assert_eq!(
            extract_index_from_path("/api/v1/indices/my-index/stats"),
            Some("my-index".to_string())
        );
        assert_eq!(
            extract_index_from_path("/api/v1/indices/my-index/_rollover"),
            None
        );
    }

    #[test]
    fn test_generate_push_hints_search() {
        let config = Http2PushConfig {
            enable_search_hints: true,
            ..Default::default()
        };

        let hints = generate_push_hints("/api/v1/indices/test-index/search", &config);
        assert!(!hints.is_empty());
        assert!(
            hints
                .iter()
                .any(|h| h.contains("/api/v1/indices/test-index"))
        );
    }

    #[test]
    fn test_generate_push_hints_document() {
        let config = Http2PushConfig {
            enable_document_hints: true,
            ..Default::default()
        };

        let hints = generate_push_hints("/api/v1/indices/test-index/documents/123", &config);
        assert!(!hints.is_empty());
        assert!(
            hints
                .iter()
                .any(|h| h.contains("/api/v1/indices/test-index"))
        );
    }

    #[test]
    fn test_generate_push_hints_index() {
        let config = Http2PushConfig {
            enable_index_hints: true,
            ..Default::default()
        };

        let hints = generate_push_hints("/api/v1/indices/test-index", &config);
        assert!(!hints.iter().any(|h| h.contains("/search")));
    }

    #[test]
    fn test_config_default() {
        let config = Http2PushConfig::default();
        assert!(config.enabled);
        assert_eq!(config.max_resources, 5);
        assert!(config.enable_search_hints);
        assert!(config.enable_document_hints);
        assert!(config.enable_index_hints);
    }

    #[test]
    fn test_config_disabled() {
        let config = Http2PushConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(!config.enabled);
    }
}
