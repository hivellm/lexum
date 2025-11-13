//! Memory optimization utilities
//!
//! This module provides utilities for optimizing memory usage, including
//! buffer pooling and allocation reduction strategies.

pub mod buffer_pool;

pub use buffer_pool::{BufferPool, StringBufferPool};
