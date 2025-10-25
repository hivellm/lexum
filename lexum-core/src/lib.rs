//! Lexum Core - High-performance distributed search engine
//!
//! This crate provides the core functionality for the Lexum search engine,
//! including configuration management, logging, indexing, and search operations.
//!
//! # Examples
//!
//! ```rust,no_run
//! use lexum_core::config::Config;
//! use lexum_core::logging;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Initialize logging
//!     logging::init()?;
//!
//!     // Load configuration
//!     let config = Config::from_file("config.yml").await?;
//!     
//!     tracing::info!("Lexum started with config: {:?}", config);
//!     Ok(())
//! }
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_code)]

/// Configuration management
pub mod config;

/// Document operations
pub mod document;

/// Error types
pub mod error;

/// Index management
pub mod index;

/// Logging and tracing setup
pub mod logging;

/// Query types and builders
pub mod query;

/// Schema management
pub mod schema;

/// Search execution
pub mod search;

/// Common types
pub mod types;

// Re-export commonly used items
pub use config::Config;
pub use document::DocumentStore;
pub use error::{Error, Result};
pub use index::{Index, IndexManager, IndexSettings, IndexStats};
pub use query::{
    BoolQuery, FuzzyQuery, MatchQuery, PhraseQuery, Query, QueryBuilder, RangeQuery, TermQuery,
};
pub use schema::{FieldConfig, FieldType, SchemaBuilder};
pub use search::{SearchExecutor, SearchHit, SearchResult, SortOption, SortOrder};
