//! Query cache with LRU eviction and TTL support
//!
//! This module provides a query cache that stores search results with:
//! - LRU (Least Recently Used) eviction policy
//! - TTL (Time To Live) expiration
//! - Configurable cache size and TTL

use crate::search::result::SearchResult;
use lru::LruCache;
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Cache entry with expiration time
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached search result
    result: SearchResult,
    /// When this entry expires
    expires_at: Instant,
}

impl CacheEntry {
    /// Create a new cache entry with TTL
    fn new(result: SearchResult, ttl: Duration) -> Self {
        Self {
            result,
            expires_at: Instant::now() + ttl,
        }
    }

    /// Check if entry has expired
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }
}

/// Query cache with LRU eviction and TTL
#[derive(Debug, Clone)]
pub struct QueryCache {
    /// LRU cache storage
    cache: Arc<Mutex<LruCache<String, CacheEntry>>>,
    /// Default TTL for cache entries
    default_ttl: Duration,
    /// Whether caching is enabled
    enabled: bool,
    /// Statistics tracking
    stats: Arc<Mutex<CacheStats>>,
}

/// Internal cache statistics
#[derive(Debug, Default)]
struct CacheStats {
    /// Number of cache hits
    hits: u64,
    /// Number of cache misses
    misses: u64,
    /// Number of entries evicted due to LRU
    lru_evictions: u64,
    /// Number of entries evicted due to expiration
    expired_evictions: u64,
    /// Number of entries added to cache
    inserts: u64,
}

impl QueryCache {
    /// Create new query cache with default settings
    ///
    /// Defaults:
    /// - Max size: 1000 entries
    /// - TTL: 5 minutes
    pub fn new() -> Self {
        Self::with_capacity_and_ttl(1000, Duration::from_secs(300))
    }

    /// Create query cache with custom capacity and TTL
    ///
    /// # Arguments
    /// * `capacity` - Maximum number of cache entries
    /// * `ttl` - Time to live for cache entries
    pub fn with_capacity_and_ttl(capacity: usize, ttl: Duration) -> Self {
        let capacity = NonZeroUsize::new(capacity.max(1)).unwrap_or(NonZeroUsize::new(1).unwrap());
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(capacity))),
            default_ttl: ttl,
            enabled: true,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Create disabled query cache
    pub fn disabled() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(1).unwrap()))),
            default_ttl: Duration::ZERO,
            enabled: false,
            stats: Arc::new(Mutex::new(CacheStats::default())),
        }
    }

    /// Get cached result if available and not expired
    ///
    /// # Arguments
    /// * `key` - Cache key
    ///
    /// # Returns
    /// * `Some(SearchResult)` if cached and not expired
    /// * `None` if not cached or expired
    pub fn get(&self, key: &str) -> Option<SearchResult> {
        if !self.enabled {
            return None;
        }

        let mut cache = self.cache.lock();
        let mut stats = self.stats.lock();

        // Try to get entry
        if let Some(entry) = cache.get(key) {
            // Check expiration
            if entry.is_expired() {
                // Remove expired entry
                cache.pop(key);
                stats.misses += 1;
                stats.expired_evictions += 1;
                return None;
            }
            // Cache hit
            stats.hits += 1;
            return Some(entry.result.clone());
        }

        // Cache miss
        stats.misses += 1;
        None
    }

    /// Put result in cache with default TTL
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `result` - Search result to cache
    pub fn put(&self, key: String, result: SearchResult) {
        self.put_with_ttl(key, result, self.default_ttl);
    }

    /// Put result in cache with custom TTL
    ///
    /// # Arguments
    /// * `key` - Cache key
    /// * `result` - Search result to cache
    /// * `ttl` - Time to live for this entry
    pub fn put_with_ttl(&self, key: String, result: SearchResult, ttl: Duration) {
        if !self.enabled || ttl.is_zero() {
            return;
        }

        let mut cache = self.cache.lock();
        let mut stats = self.stats.lock();

        // Check if we're replacing an existing entry (LRU eviction)
        let was_full = cache.len() >= cache.cap().get();
        let had_entry = cache.contains(&key);

        let entry = CacheEntry::new(result, ttl);
        cache.put(key, entry);

        stats.inserts += 1;

        // Track LRU evictions (when cache was full and we added a new key)
        if was_full && !had_entry {
            stats.lru_evictions += 1;
        }
    }

    /// Remove entry from cache
    ///
    /// # Arguments
    /// * `key` - Cache key to remove
    pub fn remove(&self, key: &str) {
        let mut cache = self.cache.lock();
        cache.pop(key);
    }

    /// Clear all cache entries
    pub fn clear(&self) {
        let mut cache = self.cache.lock();
        cache.clear();
    }

    /// Get cache size (number of entries)
    pub fn len(&self) -> usize {
        let cache = self.cache.lock();
        cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        let cache = self.cache.lock();
        cache.is_empty()
    }

    /// Get cache capacity
    pub fn capacity(&self) -> usize {
        let cache = self.cache.lock();
        cache.cap().get()
    }

    /// Evict expired entries
    ///
    /// This method removes all expired entries from the cache.
    /// It's recommended to call this periodically to free memory.
    pub fn evict_expired(&self) -> usize {
        if !self.enabled {
            return 0;
        }

        let mut cache = self.cache.lock();
        let mut stats = self.stats.lock();
        let mut evicted = 0;

        // Collect expired keys
        let expired_keys: Vec<String> = cache
            .iter()
            .filter_map(|(key, entry)| {
                if entry.is_expired() {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();

        // Remove expired entries
        for key in expired_keys {
            cache.pop(&key);
            evicted += 1;
            stats.expired_evictions += 1;
        }

        evicted
    }

    /// Get cache statistics
    pub fn stats(&self) -> QueryCacheStats {
        let cache = self.cache.lock();
        let stats = self.stats.lock();
        let mut expired_count = 0;

        for entry in cache.iter() {
            if entry.1.is_expired() {
                expired_count += 1;
            }
        }

        let total_requests = stats.hits + stats.misses;
        let hit_rate = if total_requests > 0 {
            stats.hits as f64 / total_requests as f64
        } else {
            0.0
        };

        QueryCacheStats {
            size: cache.len(),
            capacity: cache.cap().get(),
            expired_entries: expired_count,
            enabled: self.enabled,
            default_ttl_secs: self.default_ttl.as_secs(),
            hits: stats.hits,
            misses: stats.misses,
            hit_rate,
            lru_evictions: stats.lru_evictions,
            expired_evictions: stats.expired_evictions,
            total_inserts: stats.inserts,
        }
    }

    /// Reset cache statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = CacheStats::default();
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get default TTL
    pub fn default_ttl(&self) -> Duration {
        self.default_ttl
    }

    /// Warm up cache with pre-computed results
    ///
    /// This method allows pre-loading the cache with common queries and their results.
    /// Useful for improving initial performance by caching frequently used queries.
    ///
    /// # Arguments
    /// * `entries` - Vector of (cache_key, search_result) pairs to pre-load
    ///
    /// # Returns
    /// Number of entries successfully added to cache
    pub fn warm_up(&self, entries: Vec<(String, SearchResult)>) -> usize {
        if !self.enabled {
            return 0;
        }

        let mut cache = self.cache.lock();
        let mut stats = self.stats.lock();
        let mut added = 0;

        for (key, result) in entries {
            let entry = CacheEntry::new(result, self.default_ttl);
            cache.put(key, entry);
            added += 1;
            stats.inserts += 1;
        }

        added
    }

    /// Warm up cache with pre-computed results and custom TTL
    ///
    /// Similar to `warm_up()` but allows specifying custom TTL for each entry.
    ///
    /// # Arguments
    /// * `entries` - Vector of (cache_key, search_result, ttl) tuples to pre-load
    ///
    /// # Returns
    /// Number of entries successfully added to cache
    pub fn warm_up_with_ttl(&self, entries: Vec<(String, SearchResult, Duration)>) -> usize {
        if !self.enabled {
            return 0;
        }

        let mut cache = self.cache.lock();
        let mut stats = self.stats.lock();
        let mut added = 0;

        for (key, result, ttl) in entries {
            if ttl.is_zero() {
                continue;
            }
            let entry = CacheEntry::new(result, ttl);
            cache.put(key, entry);
            added += 1;
            stats.inserts += 1;
        }

        added
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Query cache statistics
#[derive(Debug, Clone)]
pub struct QueryCacheStats {
    /// Current number of cached entries
    pub size: usize,
    /// Maximum cache capacity
    pub capacity: usize,
    /// Number of expired entries (not yet evicted)
    pub expired_entries: usize,
    /// Whether caching is enabled
    pub enabled: bool,
    /// Default TTL in seconds
    pub default_ttl_secs: u64,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub hit_rate: f64,
    /// Number of entries evicted due to LRU
    pub lru_evictions: u64,
    /// Number of entries evicted due to expiration
    pub expired_evictions: u64,
    /// Total number of entries inserted
    pub total_inserts: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SearchResult;

    fn create_test_result() -> SearchResult {
        SearchResult::new(vec![], 0, 0)
    }

    #[test]
    fn test_query_cache_creation() {
        let cache = QueryCache::new();
        assert!(cache.is_enabled());
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_query_cache_disabled() {
        let cache = QueryCache::disabled();
        assert!(!cache.is_enabled());
    }

    #[test]
    fn test_query_cache_put_get() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result.clone());
        assert_eq!(cache.len(), 1);

        let cached = cache.get("key1");
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().total, result.total);
    }

    #[test]
    fn test_query_cache_expiration() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_millis(100));
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        assert!(cache.get("key1").is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_query_cache_lru_eviction() {
        let cache = QueryCache::with_capacity_and_ttl(2, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result.clone());
        cache.put("key2".to_string(), result.clone());
        cache.put("key3".to_string(), result.clone());

        // key1 should be evicted (LRU)
        assert_eq!(cache.len(), 2);
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
        assert!(cache.get("key3").is_some());
    }

    #[test]
    fn test_query_cache_evict_expired() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_millis(50));
        let result = create_test_result();

        cache.put("key1".to_string(), result.clone());
        cache.put_with_ttl("key2".to_string(), result, Duration::from_secs(60));

        // Wait for key1 to expire
        std::thread::sleep(Duration::from_millis(100));

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
        assert_eq!(cache.len(), 1);
        assert!(cache.get("key1").is_none());
        assert!(cache.get("key2").is_some());
    }

    #[test]
    fn test_query_cache_clear() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn test_query_cache_remove() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        assert_eq!(cache.len(), 1);

        cache.remove("key1");
        assert_eq!(cache.len(), 0);
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn test_query_cache_stats() {
        let cache = QueryCache::with_capacity_and_ttl(100, Duration::from_secs(300));
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        let stats = cache.stats();

        assert_eq!(stats.size, 1);
        assert_eq!(stats.capacity, 100);
        assert_eq!(stats.expired_entries, 0);
        assert!(stats.enabled);
        assert_eq!(stats.default_ttl_secs, 300);
        assert_eq!(stats.total_inserts, 1);
    }

    #[test]
    fn test_query_cache_hit_miss_stats() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        // Insert and retrieve
        cache.put("key1".to_string(), result.clone());
        assert!(cache.get("key1").is_some());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.hit_rate, 1.0);
        assert_eq!(stats.total_inserts, 1);

        // Miss
        assert!(cache.get("key2").is_none());
        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.hit_rate, 0.5);
    }

    #[test]
    fn test_query_cache_eviction_stats() {
        let cache = QueryCache::with_capacity_and_ttl(2, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result.clone());
        cache.put("key2".to_string(), result.clone());
        cache.put("key3".to_string(), result.clone()); // Should evict key1

        let stats = cache.stats();
        assert_eq!(stats.lru_evictions, 1);
        assert_eq!(stats.total_inserts, 3);
    }

    #[test]
    fn test_query_cache_expired_eviction_stats() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_millis(50));
        let result = create_test_result();

        cache.put("key1".to_string(), result);

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(100));

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);

        let stats = cache.stats();
        assert_eq!(stats.expired_evictions, 1);
    }

    #[test]
    fn test_query_cache_reset_stats() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        cache.get("key1");

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.total_inserts, 1);

        cache.reset_stats();
        let stats = cache.stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_inserts, 0);
    }

    #[test]
    fn test_query_cache_disabled_put_get() {
        let cache = QueryCache::disabled();
        let result = create_test_result();

        cache.put("key1".to_string(), result);
        assert!(cache.get("key1").is_none());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_query_cache_warm_up() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result1 = create_test_result();
        let result2 = create_test_result();

        let entries = vec![
            ("key1".to_string(), result1.clone()),
            ("key2".to_string(), result2.clone()),
        ];

        let added = cache.warm_up(entries);
        assert_eq!(added, 2);
        assert_eq!(cache.len(), 2);

        assert!(cache.get("key1").is_some());
        assert!(cache.get("key2").is_some());
    }

    #[test]
    fn test_query_cache_warm_up_with_ttl() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result1 = create_test_result();
        let result2 = create_test_result();

        let entries = vec![
            ("key1".to_string(), result1.clone(), Duration::from_secs(30)),
            (
                "key2".to_string(),
                result2.clone(),
                Duration::from_secs(120),
            ),
        ];

        let added = cache.warm_up_with_ttl(entries);
        assert_eq!(added, 2);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_query_cache_warm_up_disabled() {
        let cache = QueryCache::disabled();
        let result = create_test_result();

        let entries = vec![("key1".to_string(), result)];
        let added = cache.warm_up(entries);
        assert_eq!(added, 0);
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_query_cache_warm_up_zero_ttl() {
        let cache = QueryCache::with_capacity_and_ttl(10, Duration::from_secs(60));
        let result = create_test_result();

        let entries = vec![("key1".to_string(), result, Duration::ZERO)];
        let added = cache.warm_up_with_ttl(entries);
        assert_eq!(added, 0);
        assert_eq!(cache.len(), 0);
    }
}
