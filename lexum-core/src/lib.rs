//! Lexum Core - High-performance distributed search engine
//!
//! This crate provides the core functionality for the Lexum search engine,
//! including configuration management, logging, indexing, and search operations.
//!
//! ## Performance Characteristics
//!
//! Lexum Core is built for high performance with the following characteristics:
//!
//! - **Search Latency**: Sub-millisecond for simple queries, < 3.5ms for complex queries
//! - **Throughput**: 5,000-15,000 queries per second depending on query complexity
//! - **Memory Efficiency**: ~2MB per 1,000 documents with configurable caching
//! - **Indexing Speed**: 5,000-8,000 documents per second with batch operations
//!
//! For detailed performance benchmarks and tuning guidelines, see [PERFORMANCE.md](docs/PERFORMANCE.md).
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

/// Aggregation framework
pub mod aggregation;

/// Document operations
pub mod document;

/// Error types
pub mod error;

/// Index management
pub mod index;

/// Disk I/O utilities
pub mod io;

/// Concurrency optimization utilities
pub mod concurrency;

/// Logging and tracing setup
pub mod logging;

/// Memory optimization utilities
pub mod memory;

/// Performance monitoring and regression detection
pub mod performance;

/// Progress tracking for long-running operations
pub mod progress;

/// Query types and builders
pub mod query;

/// Schema management
pub mod schema;

/// Search execution
pub mod search;

/// Snapshot and restore functionality
pub mod snapshot;

/// Script engine for document transformation
pub mod script;

/// Common types
pub mod types;

// Re-export commonly used items
pub use config::Config;
pub use document::{DocumentStore, ProgressDocumentStore};
pub use error::{Error, Result};
pub use index::{
    AliasAction, AliasConfig, AliasManager, AliasName, AliasOperationsRequest,
    AliasOperationsResponse, Index, IndexAlias, IndexManager, IndexPattern, IndexSettings,
    IndexStats, IndexTemplate, StorageSettings, TemplateManager, TemplateMappings, TemplateName,
    TemplateSettings,
};
pub use progress::{
    OperationType, ProgressId, ProgressInfo, ProgressMetrics, ProgressStatus, ProgressTracker,
};
pub use query::{
    BoolQuery, FuzzyQuery, MatchQuery, MultiMatchOperator, MultiMatchQuery, MultiMatchType,
    PhraseQuery, Query, QueryBuilder, RangeQuery, TermQuery,
};
pub use schema::{FieldConfig, FieldType, SchemaBuilder};
pub use search::{SearchExecutor, SearchHit, SearchResult, SortOption, SortOrder};
pub use snapshot::{
    CreateSnapshotRequest, RestoreSnapshotRequest, SnapshotInfo, SnapshotManager,
    SnapshotRepository, SnapshotStats,
};
