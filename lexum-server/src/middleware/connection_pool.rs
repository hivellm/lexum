//! Connection pooling configuration and management
//!
//! This module provides connection pooling configuration for HTTP connections,
//! optimizing connection reuse and reducing connection overhead.

use std::time::Duration;

/// Connection pool configuration
#[derive(Debug, Clone)]
pub struct ConnectionPoolConfig {
    /// Maximum number of idle connections per host
    pub max_idle_per_host: usize,
    /// Maximum total number of idle connections
    pub max_idle_total: usize,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Keep-alive duration
    pub keep_alive: Duration,
    /// Enable HTTP/2
    pub http2_enabled: bool,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            max_idle_per_host: 10,
            max_idle_total: 100,
            idle_timeout: Duration::from_secs(90),
            connect_timeout: Duration::from_secs(10),
            keep_alive: Duration::from_secs(30),
            http2_enabled: false, // HTTP/2 requires additional setup
        }
    }
}

impl ConnectionPoolConfig {
    /// Create new connection pool config with custom settings
    pub fn new(max_idle_per_host: usize, max_idle_total: usize, idle_timeout: Duration) -> Self {
        Self {
            max_idle_per_host,
            max_idle_total,
            idle_timeout,
            ..Default::default()
        }
    }

    /// Enable HTTP/2 support
    pub fn with_http2(mut self) -> Self {
        self.http2_enabled = true;
        self
    }

    /// Set custom keep-alive duration
    pub fn with_keep_alive(mut self, duration: Duration) -> Self {
        self.keep_alive = duration;
        self
    }
}

/// Connection pool statistics
#[derive(Debug, Clone, Default)]
pub struct ConnectionPoolStats {
    /// Current number of idle connections
    pub idle_connections: usize,
    /// Current number of active connections
    pub active_connections: usize,
    /// Total connections created
    pub total_connections: usize,
    /// Connection pool hits (reused connections)
    pub pool_hits: usize,
    /// Connection pool misses (new connections)
    pub pool_misses: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_pool_config_default() {
        let config = ConnectionPoolConfig::default();
        assert_eq!(config.max_idle_per_host, 10);
        assert_eq!(config.max_idle_total, 100);
        assert_eq!(config.idle_timeout, Duration::from_secs(90));
        assert!(!config.http2_enabled);
    }

    #[test]
    fn test_connection_pool_config_custom() {
        let config = ConnectionPoolConfig::new(20, 200, Duration::from_secs(120));
        assert_eq!(config.max_idle_per_host, 20);
        assert_eq!(config.max_idle_total, 200);
        assert_eq!(config.idle_timeout, Duration::from_secs(120));
    }

    #[test]
    fn test_connection_pool_config_http2() {
        let config = ConnectionPoolConfig::default().with_http2();
        assert!(config.http2_enabled);
    }

    #[test]
    fn test_connection_pool_config_keep_alive() {
        let keep_alive = Duration::from_secs(60);
        let config = ConnectionPoolConfig::default().with_keep_alive(keep_alive);
        assert_eq!(config.keep_alive, keep_alive);
    }

    #[test]
    fn test_connection_pool_stats_default() {
        let stats = ConnectionPoolStats::default();
        assert_eq!(stats.idle_connections, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);
        assert_eq!(stats.pool_hits, 0);
        assert_eq!(stats.pool_misses, 0);
    }
}
