//! IP whitelisting/blacklisting middleware

use axum::extract::Request;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use tower::Layer;
use tower::Service;
use tracing::{debug, warn};

/// IP filter configuration
#[derive(Clone, Debug)]
pub struct IpFilterConfig {
    /// Whitelist of allowed IP addresses
    pub whitelist: HashSet<IpAddr>,
    /// Blacklist of blocked IP addresses
    pub blacklist: HashSet<IpAddr>,
    /// Whether IP filtering is enabled
    pub enabled: bool,
    /// Whether to allow requests when whitelist is empty (default: true)
    pub allow_when_whitelist_empty: bool,
}

impl Default for IpFilterConfig {
    fn default() -> Self {
        Self {
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            enabled: false,
            allow_when_whitelist_empty: true,
        }
    }
}

impl IpFilterConfig {
    /// Create new IP filter config with whitelist
    pub fn with_whitelist(ips: Vec<IpAddr>) -> Self {
        Self {
            whitelist: ips.into_iter().collect(),
            enabled: true,
            allow_when_whitelist_empty: false,
            ..Default::default()
        }
    }

    /// Create new IP filter config with blacklist
    pub fn with_blacklist(ips: Vec<IpAddr>) -> Self {
        Self {
            blacklist: ips.into_iter().collect(),
            enabled: true,
            ..Default::default()
        }
    }

    /// Add IP to whitelist
    pub fn add_to_whitelist(&mut self, ip: IpAddr) {
        self.whitelist.insert(ip);
        self.enabled = true;
    }

    /// Add IP to blacklist
    pub fn add_to_blacklist(&mut self, ip: IpAddr) {
        self.blacklist.insert(ip);
        self.enabled = true;
    }

    /// Remove IP from whitelist
    pub fn remove_from_whitelist(&mut self, ip: &IpAddr) {
        self.whitelist.remove(ip);
    }

    /// Remove IP from blacklist
    pub fn remove_from_blacklist(&mut self, ip: &IpAddr) {
        self.blacklist.remove(ip);
    }

    /// Check if IP is allowed
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        if !self.enabled {
            return true;
        }

        // Check blacklist first
        if self.blacklist.contains(ip) {
            return false;
        }

        // Check whitelist
        if self.whitelist.is_empty() {
            // If whitelist is empty and allow_when_whitelist_empty is true, allow
            return self.allow_when_whitelist_empty;
        }

        // If whitelist is not empty, IP must be in whitelist
        self.whitelist.contains(ip)
    }
}

/// IP filtering layer
#[derive(Clone)]
pub struct IpFilterLayer {
    config: Arc<RwLock<IpFilterConfig>>,
}

impl IpFilterLayer {
    /// Create new IP filtering layer
    pub fn new(config: IpFilterConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
        }
    }

    /// Get configuration (blocking read)
    pub fn config(&self) -> std::sync::RwLockReadGuard<'_, IpFilterConfig> {
        self.config.read().unwrap()
    }

    /// Get mutable configuration (blocking write)
    pub fn config_mut(&self) -> std::sync::RwLockWriteGuard<'_, IpFilterConfig> {
        self.config.write().unwrap()
    }

    /// Extract IP address from request
    fn extract_ip(request: &Request) -> Option<IpAddr> {
        // Try to get IP from X-Forwarded-For header (for proxies)
        if let Some(forwarded_for) = request.headers().get("x-forwarded-for") {
            if let Ok(forwarded_str) = forwarded_for.to_str() {
                // X-Forwarded-For can contain multiple IPs, take the first one
                if let Some(first_ip) = forwarded_str.split(',').next() {
                    if let Ok(ip) = first_ip.trim().parse::<IpAddr>() {
                        return Some(ip);
                    }
                }
            }
        }

        // Try X-Real-IP header
        if let Some(real_ip) = request.headers().get("x-real-ip") {
            if let Ok(real_ip_str) = real_ip.to_str() {
                if let Ok(ip) = real_ip_str.parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }

        // Try to get from remote_addr in extensions (set by axum when using ConnectInfo)
        if let Some(remote_addr) = request.extensions().get::<std::net::SocketAddr>() {
            return Some(remote_addr.ip());
        }

        // Try CF-Connecting-IP header (Cloudflare)
        if let Some(cf_ip) = request.headers().get("cf-connecting-ip") {
            if let Ok(cf_ip_str) = cf_ip.to_str() {
                if let Ok(ip) = cf_ip_str.parse::<IpAddr>() {
                    return Some(ip);
                }
            }
        }

        None
    }

    /// Check if request IP is allowed
    fn check_ip_sync(&self, request: &Request) -> Result<(), IpFilterError> {
        let config = self.config.read().unwrap();

        if !config.enabled {
            return Ok(());
        }

        let ip = Self::extract_ip(request).ok_or(IpFilterError::IpNotFound)?;

        if !config.is_allowed(&ip) {
            warn!(
                ip = %ip,
                "IP address blocked by filter"
            );
            return Err(IpFilterError::IpBlocked);
        }

        debug!(ip = %ip, "IP address allowed");
        Ok(())
    }
}

/// IP filter errors
#[derive(Debug, Clone, Copy)]
pub enum IpFilterError {
    /// IP address not found in request
    IpNotFound,
    /// IP address is blocked
    IpBlocked,
}

impl IpFilterError {
    /// Get HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            IpFilterError::IpNotFound => StatusCode::BAD_REQUEST,
            IpFilterError::IpBlocked => StatusCode::FORBIDDEN,
        }
    }

    /// Get human-readable error message
    pub fn message(&self) -> &'static str {
        match self {
            IpFilterError::IpNotFound => "Could not determine client IP address",
            IpFilterError::IpBlocked => "IP address is not allowed",
        }
    }
}

impl<S> Layer<S> for IpFilterLayer {
    type Service = IpFilterService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        IpFilterService {
            inner,
            layer: self.clone(),
        }
    }
}

/// IP filtering service
#[derive(Clone)]
pub struct IpFilterService<S> {
    inner: S,
    layer: IpFilterLayer,
}

impl<S> Service<Request> for IpFilterService<S>
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
        let layer = self.layer.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            match layer.check_ip_sync(&request) {
                Ok(_) => {
                    let response = inner.call(request).await?;
                    let response: Response<axum::body::Body> = response.into_response();
                    Ok(response)
                }
                Err(error) => {
                    warn!(
                        error = ?error,
                        "IP filter blocked request"
                    );
                    let status = error.status_code();
                    let message = error.message();
                    let response = (
                        status,
                        [("Content-Type", "application/json")],
                        format!(
                            r#"{{"error":{{"type":"ip_filter_error","message":"{}","status":{}}}"#,
                            message,
                            status.as_u16()
                        ),
                    )
                        .into_response();
                    Ok(response)
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_filter_config_default() {
        let config = IpFilterConfig::default();
        assert!(!config.enabled);
        assert!(config.whitelist.is_empty());
        assert!(config.blacklist.is_empty());
        assert!(config.allow_when_whitelist_empty);
    }

    #[test]
    fn test_ip_filter_config_with_whitelist() {
        let ips = vec!["127.0.0.1".parse().unwrap(), "192.168.1.1".parse().unwrap()];
        let config = IpFilterConfig::with_whitelist(ips.clone());
        assert!(config.enabled);
        assert_eq!(config.whitelist.len(), 2);
        assert!(!config.allow_when_whitelist_empty);
    }

    #[test]
    fn test_ip_filter_config_with_blacklist() {
        let ips = vec!["10.0.0.1".parse().unwrap()];
        let config = IpFilterConfig::with_blacklist(ips.clone());
        assert!(config.enabled);
        assert_eq!(config.blacklist.len(), 1);
    }

    #[test]
    fn test_ip_filter_config_add_remove() {
        let mut config = IpFilterConfig::default();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        config.add_to_whitelist(ip);
        assert!(config.whitelist.contains(&ip));
        assert!(config.enabled);

        config.remove_from_whitelist(&ip);
        assert!(!config.whitelist.contains(&ip));
    }

    #[test]
    fn test_ip_filter_config_is_allowed_disabled() {
        let config = IpFilterConfig::default();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(config.is_allowed(&ip)); // Should allow when disabled
    }

    #[test]
    fn test_ip_filter_config_is_allowed_blacklist() {
        let mut config = IpFilterConfig::default();
        let ip: IpAddr = "10.0.0.1".parse().unwrap();
        config.add_to_blacklist(ip);
        assert!(!config.is_allowed(&ip));
    }

    #[test]
    fn test_ip_filter_config_is_allowed_whitelist() {
        let mut config = IpFilterConfig::default();
        let allowed_ip: IpAddr = "127.0.0.1".parse().unwrap();
        let blocked_ip: IpAddr = "192.168.1.1".parse().unwrap();

        config.add_to_whitelist(allowed_ip);
        config.allow_when_whitelist_empty = false;

        assert!(config.is_allowed(&allowed_ip));
        assert!(!config.is_allowed(&blocked_ip));
    }

    #[test]
    fn test_ip_filter_config_whitelist_empty_allows() {
        let config = IpFilterConfig {
            enabled: true,
            whitelist: HashSet::new(),
            blacklist: HashSet::new(),
            allow_when_whitelist_empty: true,
        };
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(config.is_allowed(&ip));
    }

    #[test]
    fn test_ip_filter_error_status_codes() {
        assert_eq!(
            IpFilterError::IpNotFound.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            IpFilterError::IpBlocked.status_code(),
            StatusCode::FORBIDDEN
        );
    }
}
