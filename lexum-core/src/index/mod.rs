//! Index management module
//!
//! Provides functionality for creating, managing, and deleting search indices.

pub mod manager;
pub mod settings;

pub use manager::{Index, IndexManager};
pub use settings::IndexSettings;
