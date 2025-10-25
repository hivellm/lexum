//! Rate limiting middleware (simplified)

use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Rate limit configuration
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: usize,
    /// Time window duration
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
        }
    }
}

/// Rate limit state for a client
#[derive(Clone)]
struct ClientState {
    count: usize,
    window_start: Instant,
}

/// Rate limiting layer (placeholder for future implementation)
#[derive(Clone)]
pub struct RateLimitLayer {
    config: RateLimitConfig,
    #[allow(dead_code)]
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
}

// Note: Full Tower Layer implementation requires more complex types
// This is a simplified version that can be expanded later

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
        };
        assert_eq!(config.max_requests, 50);
        assert_eq!(config.window, Duration::from_secs(30));
    }

    #[test]
    fn test_rate_limit_layer_creation() {
        let config = RateLimitConfig::default();
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
}
