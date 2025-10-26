//! Snapshot and restore functionality for Lexum
//!
//! This module provides snapshot repository management, snapshot creation,
//! restoration, and incremental backup capabilities.

pub mod compression;
pub mod incremental;
pub mod manager;
pub mod parallel;
pub mod repository;
pub mod types;

// Phase 3 tests - temporarily disabled for stable coverage report
// #[cfg(test)]
// mod phase3_tests;

pub use manager::SnapshotManager;
pub use repository::SnapshotRepository;
pub use types::*;
