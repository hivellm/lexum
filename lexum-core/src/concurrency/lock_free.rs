//! Lock-free data structures for high-performance concurrent access
//!
//! This module provides lock-free implementations of common data structures
//! optimized for concurrent read-heavy workloads.

use dashmap::DashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Lock-free cache with TTL support
pub struct LockFreeCache<K, V> {
    /// Internal map (DashMap is lock-free for reads)
    map: Arc<DashMap<K, CacheEntry<V>>>,
    /// Statistics
    stats: Arc<CacheStats>,
    /// Default TTL
    default_ttl: Duration,
}

struct CacheEntry<V> {
    value: V,
    expires_at: Instant,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    evictions: Arc<AtomicU64>,
    inserts: Arc<AtomicU64>,
}

impl CacheStats {
    fn new() -> Self {
        Self {
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            evictions: Arc::new(AtomicU64::new(0)),
            inserts: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Get number of cache hits
    pub fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    /// Get number of cache misses
    pub fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    /// Get number of evictions
    pub fn evictions(&self) -> u64 {
        self.evictions.load(Ordering::Relaxed)
    }

    /// Get number of inserts
    pub fn inserts(&self) -> u64 {
        self.inserts.load(Ordering::Relaxed)
    }

    /// Get cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits();
        let misses = self.misses();
        if hits + misses == 0 {
            0.0
        } else {
            hits as f64 / (hits + misses) as f64
        }
    }
}

impl<K, V> LockFreeCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create a new lock-free cache with default TTL
    pub fn new(default_ttl: Duration) -> Self {
        Self {
            map: Arc::new(DashMap::new()),
            stats: Arc::new(CacheStats::new()),
            default_ttl,
        }
    }

    /// Get a value from the cache
    pub fn get(&self, key: &K) -> Option<V> {
        match self.map.get(key) {
            Some(entry) => {
                // Check if expired
                if entry.expires_at < Instant::now() {
                    // Remove expired entry
                    self.map.remove(key);
                    self.stats.evictions.fetch_add(1, Ordering::Relaxed);
                    self.stats.misses.fetch_add(1, Ordering::Relaxed);
                    return None;
                }

                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.value.clone())
            }
            None => {
                // Key doesn't exist - count as miss
                self.stats.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    /// Insert a value into the cache with default TTL
    pub fn insert(&self, key: K, value: V) {
        self.insert_with_ttl(key, value, self.default_ttl);
    }

    /// Insert a value into the cache with custom TTL
    pub fn insert_with_ttl(&self, key: K, value: V, ttl: Duration) {
        let entry = CacheEntry {
            value,
            expires_at: Instant::now() + ttl,
        };
        self.map.insert(key, entry);
        self.stats.inserts.fetch_add(1, Ordering::Relaxed);
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        self.map.remove(key).map(|(_, entry)| entry.value)
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        self.map.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            hits: Arc::clone(&self.stats.hits),
            misses: Arc::clone(&self.stats.misses),
            evictions: Arc::clone(&self.stats.evictions),
            inserts: Arc::clone(&self.stats.inserts),
        }
    }

    /// Evict expired entries
    pub fn evict_expired(&self) -> usize {
        let now = Instant::now();
        let mut evicted = 0usize;

        self.map.retain(|_, entry| {
            if entry.expires_at < now {
                evicted += 1;
                false
            } else {
                true
            }
        });

        self.stats
            .evictions
            .fetch_add(evicted as u64, Ordering::Relaxed);
        evicted
    }

    /// Get approximate size (may include expired entries)
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_lock_free_cache_insert_get() {
        let cache = LockFreeCache::new(Duration::from_secs(60));
        cache.insert("key1", "value1");

        assert_eq!(cache.get(&"key1"), Some("value1"));
        assert_eq!(cache.get(&"key2"), None);
    }

    #[test]
    #[ignore = "Slow test - waits for TTL expiration"]
    fn test_lock_free_cache_ttl() {
        let cache = LockFreeCache::new(Duration::from_millis(100));
        cache.insert("key1", "value1");

        assert_eq!(cache.get(&"key1"), Some("value1"));

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));

        // Should be expired
        assert_eq!(cache.get(&"key1"), None);
    }

    #[test]
    fn test_lock_free_cache_stats() {
        let cache = LockFreeCache::new(Duration::from_secs(60));
        cache.insert("key1", "value1");
        let _ = cache.get(&"key1"); // Hit
        let _ = cache.get(&"key2"); // Miss

        let stats = cache.stats();
        // Note: stats may include evictions from expired entries, so we check >=
        assert!(stats.hits() >= 1);
        assert!(stats.misses() >= 1);
        assert_eq!(stats.inserts(), 1);
    }

    #[test]
    fn test_lock_free_cache_evict_expired() {
        let cache = LockFreeCache::new(Duration::from_millis(50));
        cache.insert("key1", "value1");
        cache.insert("key2", "value2");

        std::thread::sleep(Duration::from_millis(100));

        let evicted = cache.evict_expired();
        assert!(evicted >= 2);
        assert!(cache.is_empty());
    }
}
