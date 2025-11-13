//! Buffer pool for reusing allocated buffers
//!
//! This module provides a buffer pool that allows reusing allocated buffers
//! to reduce memory allocations in hot paths.

use std::sync::{Arc, Mutex};

/// Thread-safe buffer pool for reusing Vec buffers
#[derive(Debug, Clone)]
pub struct BufferPool<T> {
    /// Pool of available buffers
    pool: Arc<Mutex<Vec<Vec<T>>>>,
    /// Maximum number of buffers to keep in pool
    max_pool_size: usize,
    /// Initial capacity for new buffers
    default_capacity: usize,
}

impl<T> BufferPool<T> {
    /// Create new buffer pool with default settings
    ///
    /// Defaults:
    /// - Max pool size: 10 buffers
    /// - Default capacity: 100 elements
    pub fn new() -> Self {
        Self::with_settings(10, 100)
    }

    /// Create buffer pool with custom settings
    ///
    /// # Arguments
    /// * `max_pool_size` - Maximum number of buffers to keep in pool
    /// * `default_capacity` - Initial capacity for new buffers
    pub fn with_settings(max_pool_size: usize, default_capacity: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::new())),
            max_pool_size,
            default_capacity,
        }
    }

    /// Get a buffer from the pool or create a new one
    ///
    /// # Returns
    /// A Vec buffer (may be reused from pool or newly allocated)
    pub fn get(&self) -> Vec<T> {
        let mut pool = self.pool.lock().unwrap();
        pool.pop()
            .unwrap_or_else(|| Vec::with_capacity(self.default_capacity))
    }

    /// Return a buffer to the pool for reuse
    ///
    /// The buffer will be cleared before being added to the pool.
    /// If the pool is full, the buffer will be dropped.
    ///
    /// # Arguments
    /// * `buffer` - Buffer to return to pool
    pub fn put(&self, mut buffer: Vec<T>) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            buffer.clear();
            pool.push(buffer);
        }
    }

    /// Get pool statistics
    ///
    /// # Returns
    /// Tuple of (current_pool_size, max_pool_size)
    pub fn stats(&self) -> (usize, usize) {
        let pool = self.pool.lock().unwrap();
        (pool.len(), self.max_pool_size)
    }

    /// Clear all buffers from the pool
    pub fn clear(&self) {
        let mut pool = self.pool.lock().unwrap();
        pool.clear();
    }
}

impl<T> Default for BufferPool<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// String buffer pool for reusing String buffers
#[derive(Debug, Clone)]
pub struct StringBufferPool {
    /// Pool of available string buffers
    pool: Arc<Mutex<Vec<String>>>,
    /// Maximum number of buffers to keep in pool
    max_pool_size: usize,
    /// Initial capacity for new buffers
    default_capacity: usize,
}

impl StringBufferPool {
    /// Create new string buffer pool with default settings
    ///
    /// Defaults:
    /// - Max pool size: 20 buffers
    /// - Default capacity: 256 bytes
    pub fn new() -> Self {
        Self::with_settings(20, 256)
    }

    /// Create string buffer pool with custom settings
    ///
    /// # Arguments
    /// * `max_pool_size` - Maximum number of buffers to keep in pool
    /// * `default_capacity` - Initial capacity for new buffers in bytes
    pub fn with_settings(max_pool_size: usize, default_capacity: usize) -> Self {
        Self {
            pool: Arc::new(Mutex::new(Vec::new())),
            max_pool_size,
            default_capacity,
        }
    }

    /// Get a string buffer from the pool or create a new one
    ///
    /// # Returns
    /// A String buffer (may be reused from pool or newly allocated)
    pub fn get(&self) -> String {
        let mut pool = self.pool.lock().unwrap();
        pool.pop()
            .unwrap_or_else(|| String::with_capacity(self.default_capacity))
    }

    /// Return a string buffer to the pool for reuse
    ///
    /// The buffer will be cleared before being added to the pool.
    /// If the pool is full, the buffer will be dropped.
    ///
    /// # Arguments
    /// * `buffer` - Buffer to return to pool
    pub fn put(&self, mut buffer: String) {
        let mut pool = self.pool.lock().unwrap();
        if pool.len() < self.max_pool_size {
            buffer.clear();
            pool.push(buffer);
        }
    }

    /// Get pool statistics
    ///
    /// # Returns
    /// Tuple of (current_pool_size, max_pool_size)
    pub fn stats(&self) -> (usize, usize) {
        let pool = self.pool.lock().unwrap();
        (pool.len(), self.max_pool_size)
    }

    /// Clear all buffers from the pool
    pub fn clear(&self) {
        let mut pool = self.pool.lock().unwrap();
        pool.clear();
    }
}

impl Default for StringBufferPool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool_get_put() {
        let pool = BufferPool::<i32>::new();
        let buffer = pool.get();
        assert_eq!(buffer.capacity(), 100);
        assert_eq!(buffer.len(), 0);

        pool.put(buffer);
        let stats = pool.stats();
        assert_eq!(stats.0, 1);
    }

    #[test]
    fn test_buffer_pool_reuse() {
        let pool = BufferPool::<i32>::with_settings(5, 50);
        let mut buffer1 = pool.get();
        buffer1.push(1);
        buffer1.push(2);
        pool.put(buffer1);

        let buffer2 = pool.get();
        assert_eq!(buffer2.len(), 0);
        assert!(buffer2.capacity() >= 50);
    }

    #[test]
    fn test_buffer_pool_max_size() {
        let pool = BufferPool::<i32>::with_settings(2, 10);
        pool.put(vec![1, 2, 3]);
        pool.put(vec![4, 5, 6]);
        pool.put(vec![7, 8, 9]); // Should be dropped

        let stats = pool.stats();
        assert_eq!(stats.0, 2);
    }

    #[test]
    fn test_string_buffer_pool_get_put() {
        let pool = StringBufferPool::new();
        let buffer = pool.get();
        assert_eq!(buffer.capacity(), 256);
        assert_eq!(buffer.len(), 0);

        pool.put(buffer);
        let stats = pool.stats();
        assert_eq!(stats.0, 1);
    }

    #[test]
    fn test_string_buffer_pool_reuse() {
        let pool = StringBufferPool::with_settings(5, 128);
        let mut buffer1 = pool.get();
        buffer1.push_str("test");
        pool.put(buffer1);

        let buffer2 = pool.get();
        assert_eq!(buffer2.len(), 0);
        assert!(buffer2.capacity() >= 128);
    }

    #[test]
    fn test_buffer_pool_clear() {
        let pool = BufferPool::<i32>::new();
        pool.put(vec![1, 2, 3]);
        pool.put(vec![4, 5, 6]);

        pool.clear();
        let stats = pool.stats();
        assert_eq!(stats.0, 0);
    }
}
