//! HTTP request handlers

pub mod document;
pub mod health;
pub mod index;
pub mod search;

pub use health::health_check;
