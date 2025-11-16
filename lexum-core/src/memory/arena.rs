//! Arena allocation for efficient memory management
//!
//! This module provides arena allocators that allow efficient allocation
//! and deallocation of objects with similar lifetimes, reducing memory
//! fragmentation and allocation overhead.
//!
//! # Safety
//!
//! This module uses unsafe code for low-level memory management.
//! All unsafe operations are carefully bounded and validated.

#![allow(unsafe_code)]

use std::alloc::{Layout, alloc, dealloc};
use std::marker::PhantomData;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Arena allocator for efficient batch allocation
pub struct Arena<T> {
    /// Chunks of memory
    chunks: Arc<parking_lot::Mutex<Vec<Chunk>>>,
    /// Size of each chunk in bytes
    chunk_size: usize,
    /// Alignment requirement
    align: usize,
    /// Phantom data for type
    _marker: PhantomData<T>,
}

struct Chunk {
    /// Pointer to allocated memory
    ptr: NonNull<u8>,
    /// Size of the chunk
    size: usize,
    /// Current offset in the chunk
    offset: AtomicUsize,
}

unsafe impl<T> Send for Arena<T> {}
unsafe impl<T> Sync for Arena<T> {}

impl<T> Arena<T> {
    /// Create a new arena with default chunk size
    ///
    /// Default chunk size: 64KB
    pub fn new() -> Self {
        Self::with_chunk_size(64 * 1024)
    }

    /// Create a new arena with custom chunk size
    ///
    /// # Arguments
    /// * `chunk_size` - Size of each chunk in bytes
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        let align = std::mem::align_of::<T>();
        #[allow(clippy::arc_with_non_send_sync)]
        let chunks = Arc::new(parking_lot::Mutex::new(Vec::new()));

        // Allocate first chunk
        Self::allocate_chunk_internal(&chunks, chunk_size, align);

        Self {
            chunks,
            chunk_size,
            align,
            _marker: PhantomData,
        }
    }

    /// Allocate a new chunk
    fn allocate_chunk_internal(
        chunks: &parking_lot::Mutex<Vec<Chunk>>,
        chunk_size: usize,
        align: usize,
    ) {
        let layout = Layout::from_size_align(chunk_size, align).expect("Invalid layout");

        unsafe {
            let ptr = alloc(layout);
            if ptr.is_null() {
                panic!("Failed to allocate arena chunk");
            }

            let chunk = Chunk {
                ptr: NonNull::new_unchecked(ptr),
                size: chunk_size,
                offset: AtomicUsize::new(0),
            };
            chunks.lock().push(chunk);
        }
    }

    /// Allocate space for a value
    ///
    /// # Returns
    /// A pointer to the allocated space
    pub fn allocate(&self) -> *mut T {
        let size = std::mem::size_of::<T>();
        let align = std::mem::align_of::<T>();

        loop {
            let chunks = self.chunks.lock();

            // Try to allocate in existing chunks
            for chunk in chunks.iter() {
                let offset = chunk.offset.fetch_add(size, Ordering::AcqRel);

                if offset + size <= chunk.size {
                    // Save pointer before dropping lock
                    let chunk_ptr = chunk.ptr.as_ptr();
                    drop(chunks); // Release lock before unsafe operations
                    unsafe {
                        let ptr = chunk_ptr.add(offset);
                        // Ensure alignment
                        let aligned_ptr = Self::align_ptr(ptr, align).cast::<T>();
                        return aligned_ptr;
                    }
                } else {
                    // Reset offset if we went over
                    chunk.offset.store(offset - size, Ordering::Relaxed);
                }
            }

            // No space in existing chunks, allocate new one
            drop(chunks);
            Self::allocate_chunk_internal(&self.chunks, self.chunk_size, self.align);
        }
    }

    /// Align a pointer to the specified alignment
    unsafe fn align_ptr(ptr: *mut u8, align: usize) -> *mut u8 {
        let addr = ptr as usize;
        let aligned = (addr + align - 1) & !(align - 1);
        aligned as *mut u8
    }

    /// Get the number of chunks
    pub fn chunk_count(&self) -> usize {
        self.chunks.lock().len()
    }

    /// Get total allocated memory in bytes
    pub fn total_memory(&self) -> usize {
        self.chunks.lock().len() * self.chunk_size
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for Arena<T> {
    fn drop(&mut self) {
        let chunks = self.chunks.lock();
        for chunk in chunks.iter() {
            unsafe {
                let layout = Layout::from_size_align(chunk.size, self.align).unwrap();
                dealloc(chunk.ptr.as_ptr(), layout);
            }
        }
    }
}

/// Thread-safe arena allocator
pub struct ThreadSafeArena<T> {
    inner: Arc<parking_lot::Mutex<Arena<T>>>,
}

impl<T> ThreadSafeArena<T> {
    /// Create a new thread-safe arena
    pub fn new() -> Self {
        Self {
            inner: Arc::new(parking_lot::Mutex::new(Arena::new())),
        }
    }

    /// Create a new thread-safe arena with custom chunk size
    pub fn with_chunk_size(chunk_size: usize) -> Self {
        Self {
            inner: Arc::new(parking_lot::Mutex::new(Arena::with_chunk_size(chunk_size))),
        }
    }

    /// Allocate space for a value
    pub fn allocate(&self) -> *mut T {
        self.inner.lock().allocate()
    }

    /// Get the number of chunks
    pub fn chunk_count(&self) -> usize {
        self.inner.lock().chunk_count()
    }

    /// Get total allocated memory in bytes
    pub fn total_memory(&self) -> usize {
        self.inner.lock().total_memory()
    }
}

impl<T> Default for ThreadSafeArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ThreadSafeArena<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_creation() {
        let arena = Arena::<u32>::new();
        assert!(arena.chunk_count() > 0);
    }

    #[test]
    fn test_arena_allocate() {
        let arena = Arena::<u32>::new();
        let ptr = arena.allocate();
        unsafe {
            std::ptr::write(ptr, 42);
            assert_eq!(std::ptr::read(ptr), 42);
        }
    }

    #[test]
    fn test_arena_multiple_allocations() {
        let arena = Arena::<u32>::with_chunk_size(1024);
        let mut ptrs = Vec::new();

        for i in 0..10 {
            let ptr = arena.allocate();
            unsafe {
                std::ptr::write(ptr, i);
            }
            ptrs.push(ptr);
        }

        for (i, ptr) in ptrs.iter().enumerate() {
            unsafe {
                assert_eq!(std::ptr::read(*ptr), i as u32);
            }
        }
    }

    #[test]
    fn test_thread_safe_arena() {
        let arena = ThreadSafeArena::<u32>::new();
        let ptr = arena.allocate();
        unsafe {
            std::ptr::write(ptr, 100);
            assert_eq!(std::ptr::read(ptr), 100);
        }
    }

    #[test]
    fn test_thread_safe_arena_clone() {
        let arena1 = ThreadSafeArena::<u32>::new();
        let arena2 = arena1.clone();

        let ptr1 = arena1.allocate();
        let ptr2 = arena2.allocate();

        unsafe {
            std::ptr::write(ptr1, 1);
            std::ptr::write(ptr2, 2);
            assert_eq!(std::ptr::read(ptr1), 1);
            assert_eq!(std::ptr::read(ptr2), 2);
        }
    }
}
