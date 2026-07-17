//! Progress tracking for long-running operations
//!
//! This module provides a comprehensive progress tracking system for monitoring
//! the status of long-running operations across the Lexum search engine.

use crate::error::Result;
// Re-export commonly used items

pub mod tracker;
pub mod types;

pub use tracker::ProgressTracker;
pub use types::*;

/// Progress tracking result
pub type ProgressResult<T> = Result<T>;
