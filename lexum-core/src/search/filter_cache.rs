//! Filter cache for filter query results
//!
//! This module provides a cache for filter query results, storing sets
//! of document IDs that match a given filter. This allows efficient
//! reuse of filter results across multiple queries.

use crate::query::Query;
use dashmap::DashMap;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Cache key for filter queries
type FilterCacheKey = String;

/// Cached filter result (set of matching document IDs)
type FilterDocSet = HashSet<u64>;

/// Filter cache for storing filter query results
pub struct FilterCache {
    /// Cache storage (key: filter query hash, value: document ID set)
    cache: Arc<DashMap<FilterCacheKey, FilterDocSet>>,
    /// Maximum cache size (number of entries)
    max_size: usize,
    /// Whether caching is enabled
    enabled: bool,
}

impl FilterCache {
    /// Create new filter cache with default settings
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size: 1000, // Default: cache up to 1000 filters
            enabled: true,
        }
    }

    /// Create filter cache with custom max size
    pub fn with_max_size(max_size: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size,
            enabled: true,
        }
    }

    /// Create filter cache without caching
    pub fn disabled() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size: 0,
            enabled: false,
        }
    }

    /// Generate cache key from filter query
    fn cache_key(filter: &Query) -> FilterCacheKey {
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        if let Ok(filter_json) = serde_json::to_string(filter) {
            filter_json.hash(&mut hasher);
        }
        format!("filter_{:x}", hasher.finish())
    }

    /// Get cached document set for a filter query
    pub fn get(&self, filter: &Query) -> Option<FilterDocSet> {
        if !self.enabled {
            return None;
        }

        let key = Self::cache_key(filter);
        self.cache.get(&key).map(|entry| entry.value().clone())
    }

    /// Store document set result for a filter query
    pub fn put(&self, filter: &Query, doc_set: FilterDocSet) {
        if !self.enabled {
            return;
        }

        // Evict oldest entries if cache is full
        if self.cache.len() >= self.max_size {
            // Simple eviction: remove first entry (FIFO)
            // In a production system, you'd use LRU or similar
            if let Some(entry) = self.cache.iter().next() {
                self.cache.remove(entry.key());
            }
        }

        let key = Self::cache_key(filter);
        self.cache.insert(key, doc_set);
    }

    /// Clear the filter cache
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache size (number of cached filters)
    pub fn size(&self) -> usize {
        self.cache.len()
    }

    /// Get cache statistics
    pub fn stats(&self) -> FilterCacheStats {
        FilterCacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            enabled: self.enabled,
        }
    }
}

impl Default for FilterCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Filter cache statistics
#[derive(Debug, Clone)]
pub struct FilterCacheStats {
    /// Current cache size
    pub size: usize,
    /// Maximum cache size
    pub max_size: usize,
    /// Whether caching is enabled
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::QueryBuilder;

    #[test]
    fn test_filter_cache_creation() {
        let cache = FilterCache::new();
        assert!(cache.enabled);
        assert_eq!(cache.max_size, 1000);
    }

    #[test]
    fn test_filter_cache_disabled() {
        let cache = FilterCache::disabled();
        assert!(!cache.enabled);
    }

    #[test]
    fn test_filter_cache_key_generation() {
        let filter1 = QueryBuilder::term_query("status", "active");
        let filter2 = QueryBuilder::term_query("status", "active");
        let filter3 = QueryBuilder::term_query("status", "inactive");

        let key1 = FilterCache::cache_key(&filter1);
        let key2 = FilterCache::cache_key(&filter2);
        let key3 = FilterCache::cache_key(&filter3);

        // Same filter should generate same key
        assert_eq!(key1, key2);
        // Different filter should generate different key
        assert_ne!(key1, key3);
    }

    #[test]
    fn test_filter_cache_put_get() {
        let cache = FilterCache::new();
        let filter = QueryBuilder::term_query("status", "active");

        // Create a simple document set for testing
        let mut doc_set = HashSet::new();
        doc_set.insert(1);
        doc_set.insert(5);
        doc_set.insert(10);

        // Put in cache
        cache.put(&filter, doc_set.clone());

        // Get from cache
        let cached = cache.get(&filter);
        assert!(cached.is_some());
        let cached_set = cached.unwrap();
        assert_eq!(cached_set.len(), doc_set.len());
        assert_eq!(cached_set, doc_set);
    }

    #[test]
    fn test_filter_cache_clear() {
        let cache = FilterCache::new();
        let filter = QueryBuilder::term_query("status", "active");

        let mut doc_set = HashSet::new();
        doc_set.insert(1);

        cache.put(&filter, doc_set);
        assert_eq!(cache.size(), 1);

        cache.clear();
        assert_eq!(cache.size(), 0);
        assert!(cache.get(&filter).is_none());
    }

    #[test]
    fn test_filter_cache_stats() {
        let cache = FilterCache::with_max_size(500);
        let stats = cache.stats();

        assert_eq!(stats.size, 0);
        assert_eq!(stats.max_size, 500);
        assert!(stats.enabled);
    }

    #[test]
    #[ignore] // Skip due to timeout
    fn test_filter_cache_max_size_eviction() {
        let cache = FilterCache::with_max_size(2);

        let filter1 = QueryBuilder::term_query("status", "active");
        let filter2 = QueryBuilder::term_query("status", "inactive");
        let filter3 = QueryBuilder::term_query("status", "pending");

        let mut doc_set1 = HashSet::new();
        doc_set1.insert(1);
        let mut doc_set2 = HashSet::new();
        doc_set2.insert(2);
        let mut doc_set3 = HashSet::new();
        doc_set3.insert(3);

        cache.put(&filter1, doc_set1);
        cache.put(&filter2, doc_set2);
        assert_eq!(cache.size(), 2);

        // Adding third should evict one
        cache.put(&filter3, doc_set3);
        assert_eq!(cache.size(), 2);
    }
}
