//! Buffered file helpers for optimized disk I/O.
//!
//! Writing large files with `tokio::fs::write` issues multiple small syscalls,
//! which becomes a bottleneck when generating snapshots or restoring indices.
//! The helpers in this module centralize buffered writes with configurable
//! buffer sizes so that high-volume disk operations follow consistent patterns.

use crate::error::Result;
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncWriteExt, BufWriter};

/// Default write buffer size (512KiB).
pub const DEFAULT_WRITE_BUFFER_SIZE: usize = 512 * 1024;

/// Buffered writer wrapper with configurable buffer size.
#[derive(Debug, Clone)]
pub struct BufferedFileWriter {
    buffer_size: usize,
}

impl Default for BufferedFileWriter {
    fn default() -> Self {
        Self {
            buffer_size: DEFAULT_WRITE_BUFFER_SIZE,
        }
    }
}

impl BufferedFileWriter {
    /// Create a buffered writer with the provided buffer size.
    pub fn with_capacity(buffer_size: usize) -> Self {
        Self { buffer_size }
    }

    /// Write the provided bytes to disk using a buffered writer.
    pub async fn write_all<P: AsRef<Path>>(&self, path: P, data: &[u8]) -> Result<()> {
        let file = File::create(path).await?;
        let mut writer = BufWriter::with_capacity(self.buffer_size, file);
        writer.write_all(data).await?;
        writer.flush().await?;
        Ok(())
    }

    /// Serialize the provided JSON value (pretty-printed) and write it buffered.
    pub async fn write_json_pretty<P: AsRef<Path>>(
        &self,
        path: P,
        value: &serde_json::Value,
    ) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(value)?;
        self.write_all(path, &bytes).await
    }
}
