//! Script engine for document transformation
//!
//! This module provides a lightweight script engine for transforming documents
//! during reindexing operations. It supports a Painless-like syntax for
//! common document transformation operations.

pub mod context;
pub mod engine;
pub mod parser;

#[cfg(test)]
mod tests;

pub use context::ScriptContext;
pub use engine::ScriptEngine;
