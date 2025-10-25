//! HTTP request handlers

pub mod admin;
pub mod document;
pub mod health;
pub mod index;
pub mod search;
pub mod snapshot;

pub use health::health_check;
