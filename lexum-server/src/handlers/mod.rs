//! HTTP request handlers

pub mod admin;
pub mod alias;
pub mod cluster;
pub mod document;
pub mod health;
pub mod index;
pub mod reindex;
pub mod search;
pub mod snapshot;
pub mod template;

pub use health::health_check;
