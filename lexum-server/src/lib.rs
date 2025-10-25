//! Lexum REST API Server
//!
//! Provides a RESTful HTTP API for the Lexum search engine.
//!
//! # Features
//!
//! - Index management (create, delete, list)
//! - Document operations (add, get, update, delete)
//! - Search queries with filtering and pagination
//! - Health check endpoint
//! - Request logging and tracing

#![warn(missing_docs)]
#![warn(clippy::all)]
#![cfg_attr(not(test), deny(unsafe_code))]

/// API error types
pub mod error;

/// HTTP handlers
pub mod handlers;

/// Middleware
pub mod middleware;

/// OpenAPI specification
pub mod openapi;

/// API router
pub mod router;

/// Server configuration
pub mod server;

// Re-export commonly used items
pub use error::{ApiError, ApiResult};
pub use server::Server;
