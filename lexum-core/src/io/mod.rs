//! I/O utilities for optimizing disk access patterns.
//!
//! This module provides reusable helpers for buffered reads/writes,
//! read-ahead hints, and other disk I/O optimizations.

pub mod buffered;
pub mod read_ahead;

pub use buffered::{BufferedFileWriter, DEFAULT_WRITE_BUFFER_SIZE};
pub use read_ahead::{DEFAULT_READ_AHEAD_SIZE, ReadAheadHint, ReadAheadReader};
