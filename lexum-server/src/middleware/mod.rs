//! Middleware for request processing

pub mod auth;
pub mod connection_pool;
pub mod content_type;
pub mod http2_push;
pub mod ip_filter;
pub mod metrics;
pub mod query_complexity;
pub mod rate_limit;
pub mod request_size;
pub mod serialization;

pub use auth::{
    AuthConfig, AuthState, auth_middleware, create_auth_error_response,
    create_unauthorized_response,
};
pub use connection_pool::{ConnectionPoolConfig, ConnectionPoolStats};
pub use content_type::{ContentTypeValidationConfig, ContentTypeValidationLayer};
pub use http2_push::{Http2PushConfig, Http2PushLayer};
pub use ip_filter::{IpFilterConfig, IpFilterLayer};
pub use query_complexity::{QueryComplexityLimitConfig, QueryComplexityLimitLayer};
pub use rate_limit::{RateLimitConfig, RateLimitLayer};
pub use request_size::{RequestSizeLimitConfig, RequestSizeLimitLayer};
pub use serialization::{OptimizedJson, SerializationConfig, SerializationOptimizer};
