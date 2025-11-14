//! Read-ahead optimization for sequential file access.
//!
//! This module provides a read-ahead buffer that pre-fetches data
//! from disk in the background to optimize sequential reads.

use crate::error::{Error, Result};
use std::path::Path;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Default read-ahead buffer size (1MB).
pub const DEFAULT_READ_AHEAD_SIZE: usize = 1024 * 1024;

/// Read-ahead reader that pre-fetches data in the background.
pub struct ReadAheadReader {
    /// Background task handle
    _task: JoinHandle<()>,
    /// Channel receiver for pre-fetched data
    receiver: mpsc::Receiver<Result<Vec<u8>>>,
    /// Current buffer
    current_buffer: Option<Vec<u8>>,
    /// Current position in buffer
    buffer_pos: usize,
    /// Read-ahead buffer size
    buffer_size: usize,
}

impl ReadAheadReader {
    /// Create a new read-ahead reader for the given file.
    ///
    /// The reader will pre-fetch data in chunks of `buffer_size` bytes
    /// to optimize sequential reads.
    pub async fn new<P: AsRef<Path>>(path: P, buffer_size: usize) -> Result<Self> {
        let file = File::open(path).await?;
        let mut reader = BufReader::new(file);
        let (sender, receiver) = mpsc::channel(2); // Buffer 2 chunks ahead

        // Spawn background task to pre-fetch data
        let task = tokio::spawn(async move {
            let mut buffer = vec![0u8; buffer_size];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => {
                        // End of file
                        break;
                    }
                    Ok(bytes_read) => {
                        let chunk = buffer[..bytes_read].to_vec();
                        if sender.send(Ok(chunk)).await.is_err() {
                            break; // Receiver dropped
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(Err(Error::Io(e))).await;
                        break;
                    }
                }
            }
        });

        Ok(Self {
            _task: task,
            receiver,
            current_buffer: None,
            buffer_pos: 0,
            buffer_size,
        })
    }

    /// Read data from the file with read-ahead optimization.
    ///
    /// This method returns data that has been pre-fetched in the background,
    /// reducing latency for sequential reads.
    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        let mut bytes_read = 0;

        while bytes_read < buf.len() {
            // Get next buffer if current is exhausted
            if self.current_buffer.is_none()
                || self.buffer_pos >= self.current_buffer.as_ref().unwrap().len()
            {
                match self.receiver.recv().await {
                    Some(Ok(buffer)) => {
                        self.current_buffer = Some(buffer);
                        self.buffer_pos = 0;
                    }
                    Some(Err(e)) => return Err(e),
                    None => {
                        // No more data available
                        break;
                    }
                }
            }

            // Copy from current buffer
            if let Some(ref buffer) = self.current_buffer {
                let remaining = buffer.len() - self.buffer_pos;
                let to_copy = std::cmp::min(remaining, buf.len() - bytes_read);
                buf[bytes_read..bytes_read + to_copy]
                    .copy_from_slice(&buffer[self.buffer_pos..self.buffer_pos + to_copy]);
                bytes_read += to_copy;
                self.buffer_pos += to_copy;
            }
        }

        Ok(bytes_read)
    }

    /// Read all remaining data from the file.
    pub async fn read_to_end(&mut self) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut buffer = vec![0u8; self.buffer_size];

        loop {
            let bytes_read = self.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            result.extend_from_slice(&buffer[..bytes_read]);
        }

        Ok(result)
    }
}

/// Simple read-ahead hint for file operations.
///
/// On systems that support it (Linux with posix_fadvise),
/// this can hint the OS to pre-fetch data for sequential access.
pub struct ReadAheadHint {
    /// Read-ahead size in bytes
    pub size: usize,
}

impl ReadAheadHint {
    /// Create a new read-ahead hint with the specified size.
    pub fn new(size: usize) -> Self {
        Self { size }
    }

    /// Apply read-ahead hint to a file (if supported by the OS).
    ///
    /// This is a no-op on systems that don't support it.
    pub fn apply_to_file(&self, _file: &File) {
        // On Linux, we could use posix_fadvise here
        // For now, this is a placeholder that can be extended
        // with platform-specific code when needed
        // Note: libc dependency would be needed for actual implementation
    }
}

impl Default for ReadAheadHint {
    fn default() -> Self {
        Self {
            size: DEFAULT_READ_AHEAD_SIZE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_read_ahead_reader() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World! This is a test file for read-ahead optimization.";
        fs::write(&file_path, test_data).await.unwrap();

        let mut reader = ReadAheadReader::new(&file_path, 16).await.unwrap();
        let mut buffer = vec![0u8; test_data.len()];
        let bytes_read = reader.read(&mut buffer).await.unwrap();

        assert_eq!(bytes_read, test_data.len());
        assert_eq!(&buffer[..bytes_read], test_data);
    }

    #[tokio::test]
    async fn test_read_ahead_reader_partial_read() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World!";
        fs::write(&file_path, test_data).await.unwrap();

        let mut reader = ReadAheadReader::new(&file_path, 8).await.unwrap();
        let mut buffer = vec![0u8; 5];
        let bytes_read = reader.read(&mut buffer).await.unwrap();

        assert_eq!(bytes_read, 5);
        assert_eq!(&buffer[..bytes_read], b"Hello");
    }

    #[tokio::test]
    async fn test_read_ahead_reader_read_to_end() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_data = b"Hello, World! This is a test.";
        fs::write(&file_path, test_data).await.unwrap();

        let mut reader = ReadAheadReader::new(&file_path, 16).await.unwrap();
        let result = reader.read_to_end().await.unwrap();

        assert_eq!(result, test_data);
    }

    #[tokio::test]
    async fn test_read_ahead_hint_default() {
        let hint = ReadAheadHint::default();
        assert_eq!(hint.size, DEFAULT_READ_AHEAD_SIZE);
    }

    #[tokio::test]
    async fn test_read_ahead_hint_custom() {
        let hint = ReadAheadHint::new(4096);
        assert_eq!(hint.size, 4096);
    }
}
