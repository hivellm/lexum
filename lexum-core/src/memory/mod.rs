//! Memory optimization utilities
//!
//! This module provides utilities for optimizing memory usage, including
//! buffer pooling, query object pooling, memory profiling, and allocation reduction strategies.

pub mod buffer_pool;
pub mod profiler;
pub mod query_pool;

pub use buffer_pool::{BufferPool, StringBufferPool};
pub use profiler::{AllocationStats, MemoryProfiler, MemoryReport, MemorySnapshot};
pub use query_pool::QueryPool;
