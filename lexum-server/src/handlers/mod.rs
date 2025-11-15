//! HTTP request handlers

pub mod admin;
pub mod alias;
pub mod auth;
pub mod batch;
pub mod bottleneck;
pub mod cluster;
pub mod document;
pub mod health;
pub mod index;
pub mod profiling;
pub mod progress;
pub mod progress_bulk;
pub mod reindex;
/// Rollover handler for index management
pub mod rollover;

#[cfg(test)]
mod rollover_test;

#[cfg(test)]
mod admin_test;

pub mod search;
pub mod snapshot;
pub mod suggest;
pub mod template;

pub use health::health_check;
