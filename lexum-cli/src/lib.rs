//! Lexum CLI library

#![warn(missing_docs)]
#![warn(clippy::all)]
#![deny(unsafe_code)]

/// Command implementations
pub mod commands;

/// HTTP client
pub mod client;

/// Output formatting
pub mod formatter;

/// REPL session
pub mod repl;
