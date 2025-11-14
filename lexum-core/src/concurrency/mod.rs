//! Concurrency optimization utilities
//!
//! This module provides utilities for optimizing concurrent operations,
//! including thread pool configuration, work stealing, and lock-free data structures.

pub mod lock_free;
pub mod thread_pool;
pub mod work_stealing;

pub use lock_free::LockFreeCache;
pub use thread_pool::{ThreadPoolConfig, ThreadPoolStats};
pub use work_stealing::WorkStealingQueue;
