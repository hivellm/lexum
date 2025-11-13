//! Field cache for efficient sorting and aggregations
//!
//! This module provides a field cache that stores field values for fast access
//! during sorting and aggregation operations. It caches values from "fast" fields
//! (column-oriented fields) to avoid repeated field value lookups.

use dashmap::DashMap;
use std::fmt;
use std::sync::Arc;

/// Field cache key (index name + field name)
type FieldCacheKey = String;

/// Cached field values for a document
#[derive(Debug, Clone, PartialEq)]
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
            _ => format!("{self}").cmp(&format!("{other}")),
        }
    }
}

impl fmt::Display for FieldValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldValue::I64(v) => write!(f, "{v}"),
            FieldValue::F64(v) => write!(f, "{v}"),
            FieldValue::String(v) => write!(f, "{v}"),
            FieldValue::Missing => write!(f, ""),
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
        format!("{index_name}:{field_name}")
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
    pub fn put(&self, index_name: &str, field_name: &str, doc_id: u64, value: FieldValue) {
        if !self.enabled {
            return;
        }

        let key = Self::cache_key(index_name, field_name);

        // Evict oldest field cache if we exceed max_fields
        if self.cache.len() >= self.max_fields && !self.cache.contains_key(&key) {
            // Simple eviction: remove first entry (FIFO-like)
            if let Some(entry) = self.cache.iter().next() {
                self.cache.remove(entry.key());
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

    /// Get all cached values for a field (for aggregations)
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name
    ///
    /// # Returns
    /// Vector of (doc_id, field_value) pairs
    pub fn get_all_values(&self, index_name: &str, field_name: &str) -> Vec<(u64, FieldValue)> {
        if !self.enabled {
            return Vec::new();
        }

        let key = Self::cache_key(index_name, field_name);
        if let Some(field_cache) = self.cache.get(&key) {
            field_cache
                .iter()
                .map(|entry| (*entry.key(), entry.value().clone()))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get values for multiple documents (for aggregations)
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name
    /// * `doc_ids` - Document IDs to retrieve
    ///
    /// # Returns
    /// Vector of field values (in same order as doc_ids, None for missing)
    pub fn get_values_for_docs(
        &self,
        index_name: &str,
        field_name: &str,
        doc_ids: &[u64],
    ) -> Vec<Option<FieldValue>> {
        if !self.enabled {
            return vec![None; doc_ids.len()];
        }

        let key = Self::cache_key(index_name, field_name);
        if let Some(field_cache) = self.cache.get(&key) {
            doc_ids
                .iter()
                .map(|doc_id| field_cache.get(doc_id).map(|v| v.clone()))
                .collect()
        } else {
            vec![None; doc_ids.len()]
        }
    }

    /// Compute statistics for numeric field values (for aggregations)
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name
    ///
    /// # Returns
    /// Aggregation stats (min, max, sum, avg, count) for numeric values
    pub fn compute_stats(
        &self,
        index_name: &str,
        field_name: &str,
    ) -> Option<FieldAggregationStats> {
        if !self.enabled {
            return None;
        }

        let values = self.get_all_values(index_name, field_name);
        if values.is_empty() {
            return None;
        }

        let mut count = 0;
        let mut sum_i64 = 0i64;
        let mut sum_f64 = 0.0f64;
        let mut min_i64: Option<i64> = None;
        let mut max_i64: Option<i64> = None;
        let mut min_f64: Option<f64> = None;
        let mut max_f64: Option<f64> = None;
        let mut has_i64 = false;
        let mut has_f64 = false;

        for (_, value) in values {
            match value {
                FieldValue::I64(v) => {
                    count += 1;
                    has_i64 = true;
                    sum_i64 += v;
                    min_i64 = Some(min_i64.map_or(v, |m| m.min(v)));
                    max_i64 = Some(max_i64.map_or(v, |m| m.max(v)));
                }
                FieldValue::F64(v) => {
                    count += 1;
                    has_f64 = true;
                    sum_f64 += v;
                    min_f64 = Some(min_f64.map_or(v, |m| m.min(v)));
                    max_f64 = Some(max_f64.map_or(v, |m| m.max(v)));
                }
                FieldValue::String(_) | FieldValue::Missing => {
                    // Skip non-numeric values for stats
                }
            }
        }

        if count == 0 {
            return None;
        }

        Some(FieldAggregationStats {
            count,
            min_i64,
            max_i64,
            sum_i64: if has_i64 { Some(sum_i64) } else { None },
            avg_i64: if has_i64 && count > 0 {
                Some(sum_i64 / count as i64)
            } else {
                None
            },
            min_f64,
            max_f64,
            sum_f64: if has_f64 { Some(sum_f64) } else { None },
            avg_f64: if has_f64 && count > 0 {
                Some(sum_f64 / count as f64)
            } else {
                None
            },
        })
    }

    /// Get unique values count (for cardinality aggregation)
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name
    ///
    /// # Returns
    /// Number of unique values (approximate for large datasets)
    pub fn cardinality(&self, index_name: &str, field_name: &str) -> usize {
        if !self.enabled {
            return 0;
        }

        let key = Self::cache_key(index_name, field_name);
        if let Some(field_cache) = self.cache.get(&key) {
            // For now, use a simple HashSet approach
            // In the future, this could use HyperLogLog for better memory efficiency
            use std::collections::HashSet;
            let mut unique_values = HashSet::new();
            for entry in field_cache.iter() {
                unique_values.insert(entry.value().to_string());
            }
            unique_values.len()
        } else {
            0
        }
    }

    /// Get term frequencies (for terms aggregation)
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name
    ///
    /// # Returns
    /// Vector of (term, count) pairs sorted by count descending
    pub fn term_frequencies(&self, index_name: &str, field_name: &str) -> Vec<(String, usize)> {
        if !self.enabled {
            return Vec::new();
        }

        let key = Self::cache_key(index_name, field_name);
        if let Some(field_cache) = self.cache.get(&key) {
            use std::collections::HashMap;
            let mut frequencies: HashMap<String, usize> = HashMap::new();

            for entry in field_cache.iter() {
                let term = entry.value().to_string();
                *frequencies.entry(term).or_insert(0) += 1;
            }

            let mut result: Vec<(String, usize)> = frequencies.into_iter().collect();
            result.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by count descending
            result
        } else {
            Vec::new()
        }
    }

    /// Preload field values for all documents in an index
    ///
    /// This method allows pre-loading field values for efficient sorting and aggregations.
    /// Useful for fields that are frequently used for sorting or aggregation operations.
    ///
    /// # Arguments
    /// * `index_name` - Index name
    /// * `field_name` - Field name to preload
    /// * `values` - Vector of (doc_id, field_value) pairs to cache
    ///
    /// # Returns
    /// Number of values successfully cached
    pub fn preload_field(
        &self,
        index_name: &str,
        field_name: &str,
        values: Vec<(u64, FieldValue)>,
    ) -> usize {
        if !self.enabled {
            return 0;
        }

        let key = Self::cache_key(index_name, field_name);

        // Evict oldest field cache if we exceed max_fields
        if self.cache.len() >= self.max_fields && !self.cache.contains_key(&key) {
            if let Some(entry) = self.cache.iter().next() {
                self.cache.remove(entry.key());
            }
        }

        let field_cache = self
            .cache
            .entry(key)
            .or_insert_with(|| Arc::new(DashMap::new()))
            .clone();

        let mut cached = 0;
        for (doc_id, value) in values {
            field_cache.insert(doc_id, value);
            cached += 1;
        }

        cached
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

/// Aggregation statistics for a field
#[derive(Debug, Clone)]
pub struct FieldAggregationStats {
    /// Total count of numeric values
    pub count: usize,
    /// Minimum i64 value (if any)
    pub min_i64: Option<i64>,
    /// Maximum i64 value (if any)
    pub max_i64: Option<i64>,
    /// Sum of i64 values (if any)
    pub sum_i64: Option<i64>,
    /// Average of i64 values (if any)
    pub avg_i64: Option<i64>,
    /// Minimum f64 value (if any)
    pub min_f64: Option<f64>,
    /// Maximum f64 value (if any)
    pub max_f64: Option<f64>,
    /// Sum of f64 values (if any)
    pub sum_f64: Option<f64>,
    /// Average of f64 values (if any)
    pub avg_f64: Option<f64>,
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
        cache.put(
            "test-index",
            "name",
            1,
            FieldValue::String("test".to_string()),
        );

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

    #[test]
    fn test_field_cache_get_all_values() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));
        cache.put("test-index", "price", 3, FieldValue::I64(300));

        let values = cache.get_all_values("test-index", "price");
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_field_cache_get_values_for_docs() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));

        let values = cache.get_values_for_docs("test-index", "price", &[1, 2, 3]);
        assert_eq!(values.len(), 3);
        assert_eq!(values[0], Some(FieldValue::I64(100)));
        assert_eq!(values[1], Some(FieldValue::I64(200)));
        assert_eq!(values[2], None);
    }

    #[test]
    fn test_field_cache_compute_stats() {
        let cache = FieldCache::new();
        cache.put("test-index", "price", 1, FieldValue::I64(100));
        cache.put("test-index", "price", 2, FieldValue::I64(200));
        cache.put("test-index", "price", 3, FieldValue::I64(300));

        let stats = cache.compute_stats("test-index", "price");
        assert!(stats.is_some());
        let stats = stats.unwrap();
        assert_eq!(stats.count, 3);
        assert_eq!(stats.min_i64, Some(100));
        assert_eq!(stats.max_i64, Some(300));
        assert_eq!(stats.sum_i64, Some(600));
        assert_eq!(stats.avg_i64, Some(200));
    }

    #[test]
    fn test_field_cache_cardinality() {
        let cache = FieldCache::new();
        cache.put(
            "test-index",
            "status",
            1,
            FieldValue::String("active".to_string()),
        );
        cache.put(
            "test-index",
            "status",
            2,
            FieldValue::String("active".to_string()),
        );
        cache.put(
            "test-index",
            "status",
            3,
            FieldValue::String("inactive".to_string()),
        );

        let cardinality = cache.cardinality("test-index", "status");
        assert_eq!(cardinality, 2); // "active" and "inactive"
    }

    #[test]
    fn test_field_cache_term_frequencies() {
        let cache = FieldCache::new();
        cache.put(
            "test-index",
            "status",
            1,
            FieldValue::String("active".to_string()),
        );
        cache.put(
            "test-index",
            "status",
            2,
            FieldValue::String("active".to_string()),
        );
        cache.put(
            "test-index",
            "status",
            3,
            FieldValue::String("inactive".to_string()),
        );

        let frequencies = cache.term_frequencies("test-index", "status");
        assert_eq!(frequencies.len(), 2);
        assert_eq!(frequencies[0].0, "active");
        assert_eq!(frequencies[0].1, 2);
        assert_eq!(frequencies[1].0, "inactive");
        assert_eq!(frequencies[1].1, 1);
    }

    #[test]
    fn test_field_cache_preload_field() {
        let cache = FieldCache::new();
        let values = vec![
            (1, FieldValue::I64(100)),
            (2, FieldValue::I64(200)),
            (3, FieldValue::I64(300)),
        ];

        let cached = cache.preload_field("test-index", "price", values);
        assert_eq!(cached, 3);

        assert_eq!(
            cache.get("test-index", "price", 1),
            Some(FieldValue::I64(100))
        );
        assert_eq!(
            cache.get("test-index", "price", 2),
            Some(FieldValue::I64(200))
        );
        assert_eq!(
            cache.get("test-index", "price", 3),
            Some(FieldValue::I64(300))
        );
    }

    #[test]
    fn test_field_cache_preload_field_disabled() {
        let cache = FieldCache::disabled();
        let values = vec![(1, FieldValue::I64(100))];

        let cached = cache.preload_field("test-index", "price", values);
        assert_eq!(cached, 0);
        assert_eq!(cache.get("test-index", "price", 1), None);
    }

    #[test]
    fn test_field_cache_preload_field_mixed_types() {
        let cache = FieldCache::new();
        let values = vec![
            (1, FieldValue::I64(100)),
            (2, FieldValue::F64(2.5)),
            (3, FieldValue::String("test".to_string())),
            (4, FieldValue::Missing),
        ];

        let cached = cache.preload_field("test-index", "mixed", values);
        assert_eq!(cached, 4);

        assert_eq!(
            cache.get("test-index", "mixed", 1),
            Some(FieldValue::I64(100))
        );
        assert_eq!(
            cache.get("test-index", "mixed", 2),
            Some(FieldValue::F64(2.5))
        );
        assert_eq!(
            cache.get("test-index", "mixed", 3),
            Some(FieldValue::String("test".to_string()))
        );
        assert_eq!(
            cache.get("test-index", "mixed", 4),
            Some(FieldValue::Missing)
        );
    }
}
