//! Field cache for efficient sorting and aggregations
//!
//! This module provides a field cache that stores field values for fast access
//! during sorting and aggregation operations. It caches values from "fast" fields
//! (column-oriented fields) to avoid repeated field value lookups.

use dashmap::DashMap;
use std::sync::Arc;
use tantivy::schema::Field;

/// Field cache key (index name + field name)
type FieldCacheKey = String;

/// Cached field values for a document
#[derive(Debug, Clone)]
pub enum FieldValue {
    /// Integer value
    I64(i64),
    /// Float value
    F64(f64),
    /// String value
    String(String),
    /// Missing/null value
    Missing,
}

impl FieldValue {
    /// Compare two field values for sorting
    pub fn compare(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (FieldValue::I64(a), FieldValue::I64(b)) => a.cmp(b),
            (FieldValue::F64(a), FieldValue::F64(b)) => {
                a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal)
            }
            (FieldValue::String(a), FieldValue::String(b)) => a.cmp(b),
            (FieldValue::Missing, FieldValue::Missing) => std::cmp::Ordering::Equal,
            (FieldValue::Missing, _) => std::cmp::Ordering::Less,
            (_, FieldValue::Missing) => std::cmp::Ordering::Greater,
            // Type mismatch - compare as strings
            _ => self.to_string().cmp(&other.to_string()),
        }
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        match self {
            FieldValue::I64(v) => v.to_string(),
            FieldValue::F64(v) => v.to_string(),
            FieldValue::String(v) => v.clone(),
            FieldValue::Missing => String::new(),
        }
    }
}

/// Field cache for storing field values
#[derive(Debug, Clone)]
pub struct FieldCache {
    /// Cache storage: (index_name:field_name) -> (doc_id -> field_value)
    cache: Arc<DashMap<FieldCacheKey, Arc<DashMap<u64, FieldValue>>>>,
    /// Maximum number of cached fields
    max_fields: usize,
    /// Whether caching is enabled
    enabled: bool,
}

impl FieldCache {
    /// Create new field cache with default settings
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_fields: 100,
            enabled: true,
        }
    }

    /// Create disabled field cache
    pub fn disabled() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_fields: 0,
            enabled: false,
        }
    }

    /// Create field cache with custom max fields
    pub fn with_max_fields(max_fields: usize) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_fields,
            enabled: true,
        }
    }

    /// Generate cache key from index and field name
    fn cache_key(index_name: &str, field_name: &str) -> FieldCacheKey {
        format!("{}:{}", index_name, field_name)
    }

    /// Get field value for a document
    pub fn get(&self, index_name: &str, field_name: &str, doc_id: u64) -> Option<FieldValue> {
        if !self.enabled {
            return None;
        }

        let key = Self::cache_key(index_name, field_name);
        self.cache
            .get(&key)
            .and_then(|field_cache| field_cache.get(&doc_id).map(|v| v.clone()))
    }

    /// Put field value for a document
    pub fn put(
        &self,
        index_name: &str,
        field_name: &str,
        doc_id: u64,
        value: FieldValue,
    ) {
        if !self.enabled {
            return;
        }

        let key = Self::cache_key(index_name, field_name);

        // Evict oldest field cache if we exceed max_fields
        if self.cache.len() >= self.max_fields && !self.cache.contains_key(&key) {
            // Simple eviction: remove first entry (FIFO-like)
            if let Some((old_key, _)) = self.cache.iter().next() {
                self.cache.remove(old_key.key());
            }
        }

        let field_cache = self
            .cache
            .entry(key)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .clone();

        field_cache.insert(doc_id, value);
    }

    /// Clear cache for a specific field
    pub fn clear_field(&self, index_name: &str, field_name: &str) {
        let key = Self::cache_key(index_name, field_name);
        self.cache.remove(&key);
    }

    /// Clear all cached fields
    pub fn clear(&self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> FieldCacheStats {
        let mut total_docs = 0;
        for field_cache in self.cache.iter() {
            total_docs += field_cache.value().len();
        }

        FieldCacheStats {
            cached_fields: self.cache.len(),
            total_cached_values: total_docs,
            max_fields: self.max_fields,
            enabled: self.enabled,
        }
    }

    /// Check if caching is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}

impl Default for FieldCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Field cache statistics
#[derive(Debug, Clone)]
pub struct FieldCacheStats {
    /// Number of cached fields
    pub cached_fields: usize,
    /// Total number of cached field values
    pub total_cached_values: usize,
    /// Maximum number of fields to cache
    pub max_fields: usize,
    /// Whether caching is enabled
    pub enabled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_cache_creation() {
        let cache = FieldCache::new();
        assert!(cache.is_enabled());
        assert_eq!(cache.max_fields, 100);
    }

    #[test]
    fn test_field_cache_disabled() {
        let cache = FieldCache::disabled();
        assert!(!cache.is_enabled());
    }

    #[test]
    fn test_field_cache_put_get() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));

        assert_eq!(
            cache.get("test-index", "price", 1),
            Some(FieldValue::I64(100))
        );
        assert_eq!(
            cache.get("test-index", "price", 2),
            Some(FieldValue::I64(200))
        );
        assert_eq!(cache.get("test-index", "price", 3), None);
    }

    #[test]
    fn test_field_cache_different_types() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "rating", 1, FieldValue::F64(4.5));
        cache.put("test-index", "name", 1, FieldValue::String("test".to_string()));

        assert_eq!(
            cache.get("test-index", "price", 1),
            Some(FieldValue::I64(100))
        );
        assert_eq!(
            cache.get("test-index", "rating", 1),
            Some(FieldValue::F64(4.5))
        );
        assert_eq!(
            cache.get("test-index", "name", 1),
            Some(FieldValue::String("test".to_string()))
        );
    }

    #[test]
    fn test_field_cache_clear() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));

        cache.clear_field("test-index", "price");
        assert_eq!(cache.get("test-index", "price", 1), None);

        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.clear();
        assert_eq!(cache.get("test-index", "price", 1), None);
    }

    #[test]
    fn test_field_cache_stats() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));
        cache.put("test-index", "rating", 1, FieldValue::F64(4.5));

        let stats = cache.stats();
        assert_eq!(stats.cached_fields, 2);
        assert_eq!(stats.total_cached_values, 3);
        assert_eq!(stats.max_fields, 100);
        assert!(stats.enabled);
    }

    #[test]
    fn test_field_value_compare() {
        assert_eq!(
            FieldValue::I64(10).compare(&FieldValue::I64(20)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            FieldValue::F64(1.5).compare(&FieldValue::F64(2.5)),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            FieldValue::String("a".to_string()).compare(&FieldValue::String("b".to_string())),
            std::cmp::Ordering::Less
        );
        assert_eq!(
            FieldValue::Missing.compare(&FieldValue::I64(10)),
            std::cmp::Ordering::Less
        );
    }

    #[test]
    fn test_field_cache_disabled_put_get() {
        let cache = FieldCache::disabled();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        assert_eq!(cache.get("test-index", "price", 1), None);
    }
}

