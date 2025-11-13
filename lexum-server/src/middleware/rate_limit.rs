//! Rate limiting middleware

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower::Layer;
use tower::Service;
use tracing::{debug, warn};

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: usize,
    /// Time window duration
    pub window: Duration,
    /// Whether to use IP-based rate limiting
    pub use_ip: bool,
    /// Whether to use API key-based rate limiting
    pub use_api_key: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            use_ip: true,
            use_api_key: true,
        }
    }
}

/// Rate limit state for a client
#[derive(Clone)]
struct ClientState {
    count: usize,
    window_start: Instant,
}

impl ClientState {
    fn new() -> Self {
        Self {
            count: 1,
            window_start: Instant::now(),
        }
    }

    fn reset_if_expired(&mut self, window: Duration) {
        if self.window_start.elapsed() >= window {
            self.count = 1;
            self.window_start = Instant::now();
        }
    }

    fn increment(&mut self, window: Duration) -> bool {
        self.reset_if_expired(window);
        self.count += 1;
        true
    }

    fn is_allowed(&mut self, max_requests: usize, window: Duration) -> bool {
        self.reset_if_expired(window);
        if self.count > max_requests {
            false
        } else {
            self.increment(window);
            true
        }
    }
}

/// Rate limiting layer
#[derive(Clone)]
pub struct RateLimitLayer {
    config: RateLimitConfig,
    state: Arc<DashMap<String, ClientState>>,
}

impl RateLimitLayer {
    /// Create new rate limiting layer
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            state: Arc::new(DashMap::new()),
        }
    }

    /// Get configuration
    pub fn config(&self) -> &RateLimitConfig {
        &self.config
    }

    /// Extract client identifier from request
    fn get_client_id(&self, headers: &HeaderMap, request: &Request) -> String {
        // Try API key first if enabled
        if self.config.use_api_key {
            if let Some(api_key) = headers.get("X-API-Key") {
                if let Ok(key_str) = api_key.to_str() {
                    if !key_str.is_empty() {
                        return format!("api_key:{key_str}");
                    }
                }
            }
            // Try Authorization header
            if let Some(auth_header) = headers.get("Authorization") {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        if !token.is_empty() {
                            return format!("api_key:{token}");
                        }
                    }
                }
            }
        }

        // Fall back to IP address
        if self.config.use_ip {
            if let Some(ip) = headers.get("X-Forwarded-For") {
                if let Ok(ip_str) = ip.to_str() {
                    // Take first IP if multiple (comma-separated)
                    let ip = ip_str.split(',').next().unwrap_or("").trim();
                    if !ip.is_empty() {
                        return format!("ip:{ip}");
                    }
                }
            }
            // Try X-Real-IP header
            if let Some(ip) = headers.get("X-Real-IP") {
                if let Ok(ip_str) = ip.to_str() {
                    if !ip_str.is_empty() {
                        return format!("ip:{ip_str}");
                    }
                }
            }
            // Fall back to remote_addr if available
            if let Some(addr) = request
                .extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
            {
                return format!("ip:{}", addr.ip());
            }
        }

        // Default fallback
        "default".to_string()
    }

    /// Check if request should be rate limited
    fn check_rate_limit(&self, client_id: &str) -> (bool, usize, usize) {
        let mut state = self
            .state
            .entry(client_id.to_string())
            .or_insert_with(ClientState::new);

        let is_allowed = state.is_allowed(self.config.max_requests, self.config.window);
        let remaining = if is_allowed {
            self.config.max_requests.saturating_sub(state.count)
        } else {
            0
        };
        let reset_at = state.window_start + self.config.window;
        let reset_seconds = reset_at.duration_since(Instant::now()).as_secs();

        (is_allowed, remaining, reset_seconds as usize)
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            layer: self.clone(),
        }
    }
}

/// Rate limiting service
#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    layer: RateLimitLayer,
}

impl<S> Service<Request> for RateLimitService<S>
where
    S: Service<Request> + Clone + Send + 'static,
    S::Response: IntoResponse,
    S::Future: Send + 'static,
{
    type Response = Response<axum::body::Body>;
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

    fn call(&mut self, request: Request) -> Self::Future {
        let headers = request.headers().clone();
        let client_id = self.layer.get_client_id(&headers, &request);
        let (is_allowed, remaining, reset_seconds) = self.layer.check_rate_limit(&client_id);

        if !is_allowed {
            debug!(
                client_id = %client_id,
                "Rate limit exceeded"
            );
            warn!(
                client_id = %client_id,
                max_requests = self.layer.config.max_requests,
                "Rate limit exceeded for client"
            );

            let mut response: Response<axum::body::Body> =
                StatusCode::TOO_MANY_REQUESTS.into_response();
            if let Ok(header) = HeaderValue::from_str(&self.layer.config.max_requests.to_string()) {
                response.headers_mut().insert("X-RateLimit-Limit", header);
            }
            if let Ok(header) = HeaderValue::from_str(&remaining.to_string()) {
                response
                    .headers_mut()
                    .insert("X-RateLimit-Remaining", header);
            }
            if let Ok(header) = HeaderValue::from_str(&reset_seconds.to_string()) {
                response.headers_mut().insert("X-RateLimit-Reset", header);
            }
            if let Ok(header) = HeaderValue::from_str(&reset_seconds.to_string()) {
                response.headers_mut().insert("Retry-After", header);
            }

            return Box::pin(async move { Ok(response) });
        }

        // Add rate limit headers to successful requests
        let mut inner = self.inner.clone();
        let max_requests_str = self.layer.config.max_requests.to_string();
        let remaining_str = remaining.to_string();
        let reset_seconds_str = reset_seconds.to_string();
        Box::pin(async move {
            let response = inner.call(request).await?;
            let response: Response<axum::body::Body> = response.into_response();
            let (mut parts, body) = response.into_parts();
            if let Ok(header) = HeaderValue::from_str(&max_requests_str) {
                parts.headers.insert("X-RateLimit-Limit", header);
            }
            if let Ok(header) = HeaderValue::from_str(&remaining_str) {
                parts.headers.insert("X-RateLimit-Remaining", header);
            }
            if let Ok(header) = HeaderValue::from_str(&reset_seconds_str) {
                parts.headers.insert("X-RateLimit-Reset", header);
            }
            Ok(Response::from_parts(parts, body))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_requests, 100);
        assert_eq!(config.window, Duration::from_secs(60));
    }

    #[test]
    fn test_rate_limit_config_custom() {
        let config = RateLimitConfig {
            max_requests: 50,
            window: Duration::from_secs(30),
            use_ip: true,
            use_api_key: true,
        };
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window, Duration::from_secs(30));
    }

    #[test]
    fn test_rate_limit_layer_creation() {
        let config = RateLimitConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            use_ip: true,
            use_api_key: true,
        };
        let layer = RateLimitLayer::new(config);
        assert_eq!(layer.config().max_requests, 100);
    }

    #[test]
    fn test_client_state_tracking() {
        let state = ClientState {
            count: 5,
            window_start: Instant::now(),
        };
        assert_eq!(state.count, 5);
    }

    #[test]
    fn test_rate_limit_allowed() {
        let config = RateLimitConfig {
            max_requests: 10,
            window: Duration::from_secs(60),
            use_ip: true,
            use_api_key: true,
        };
        let layer = RateLimitLayer::new(config);

        // First 10 requests should be allowed
        for _ in 0..10 {
            let (allowed, _, _) = layer.check_rate_limit("test_client");
            assert!(allowed, "Request should be allowed");
        }

        // 11th request should be blocked
        let (allowed, remaining, _) = layer.check_rate_limit("test_client");
        assert!(!allowed, "Request should be rate limited");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn test_rate_limit_window_reset() {
        let config = RateLimitConfig {
            max_requests: 5,
            window: Duration::from_millis(100),
            use_ip: true,
            use_api_key: true,
        };
        let layer = RateLimitLayer::new(config);

        // Exhaust rate limit
        for _ in 0..5 {
            let _ = layer.check_rate_limit("test_client");
        }

        // Should be blocked
        let (allowed, _, _) = layer.check_rate_limit("test_client");
        assert!(!allowed);

        // Wait for window to expire
        std::thread::sleep(Duration::from_millis(150));

        // Should be allowed again
        let (allowed, _, _) = layer.check_rate_limit("test_client");
        assert!(allowed);
    }

    #[test]
    fn test_get_client_id_from_api_key() {
        let config = RateLimitConfig::default();
        let layer = RateLimitLayer::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("X-API-Key", "test-key-123".parse().unwrap());

        let request = Request::builder()
            .uri("http://localhost/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let client_id = layer.get_client_id(&headers, &request);
        assert_eq!(client_id, "api_key:test-key-123");
    }

    #[test]
    fn test_get_client_id_from_ip() {
        let config = RateLimitConfig::default();
        let layer = RateLimitLayer::new(config);
        let mut headers = HeaderMap::new();
        headers.insert("X-Forwarded-For", "192.168.1.1".parse().unwrap());

        let request = Request::builder()
            .uri("http://localhost/test")
            .body(axum::body::Body::empty())
            .unwrap();

        let client_id = layer.get_client_id(&headers, &request);
        assert_eq!(client_id, "ip:192.168.1.1");
    }
}
