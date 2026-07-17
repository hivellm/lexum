//! Bucket Sort Aggregation
//!
//! Sorts buckets based on metric values.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::{SearchHit, SortOption};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Bucket Sort Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BucketSortAggregation {
    /// Sort options (default: sort by _count descending)
    #[serde(default)]
    pub sort: Vec<SortOption>,
    /// Maximum number of buckets to return (default: 10)
    #[serde(default = "default_size")]
    pub size: usize,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub from: usize,
    /// Buckets path (parent aggregation path)
    pub buckets_path: String,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_size() -> usize {
    10
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl BucketSortAggregation {
    /// Create new bucket sort aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            sort: Vec::new(),
            size: 10,
            from: 0,
            buckets_path: buckets_path.into(),
            gap_policy: "skip".to_string(),
        }
    }

    /// Add sort option
    pub fn sort(mut self, sort: SortOption) -> Self {
        self.sort.push(sort);
        self
    }

    /// Set maximum number of buckets to return
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set offset for pagination
    pub fn from(mut self, from: usize) -> Self {
        self.from = from;
        self
    }

    /// Set gap policy ("skip" or "insert_zeros")
    pub fn gap_policy(mut self, gap_policy: impl Into<String>) -> Self {
        self.gap_policy = gap_policy.into();
        self
    }
}

impl AggregationTrait for BucketSortAggregation {
    fn name(&self) -> &str {
        "bucket_sort"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Bucket Sort Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Bucket Sort Aggregation must be executed as a pipeline aggregation on parent results"
                .to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge bucket sort results by sorting and paginating buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Sort buckets based on sort options (by metric values)
        // 3. Apply pagination (from and size)
        // 4. Return sorted and paginated buckets

        // For now, return the first result as placeholder
        // Full implementation would sort and paginate buckets
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Sort and paginate buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn sort_and_paginate_buckets(
    buckets: &[Bucket],
    _sort_options: &[SortOption],
    _size: usize,
    _from: usize,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Sort buckets based on sort options
    //    - By metric value (e.g., "my_metric.value")
    //    - By bucket key
    //    - By bucket doc_count
    // 2. Apply pagination (skip _from, take _size)
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SortOrder;

    #[test]
    fn test_bucket_sort_aggregation() {
        let agg = BucketSortAggregation::new("my_histogram");

        assert_eq!(agg.buckets_path, "my_histogram");
        assert_eq!(agg.size, 10);
        assert_eq!(agg.from, 0);
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_bucket_sort_aggregation_with_sort() {
        let agg = BucketSortAggregation::new("my_histogram").sort(SortOption::desc("_count"));

        assert_eq!(agg.sort.len(), 1);
        assert_eq!(agg.sort[0].field, "_count");
        assert_eq!(agg.sort[0].order, SortOrder::Desc);
    }

    #[test]
    fn test_bucket_sort_aggregation_with_size() {
        let agg = BucketSortAggregation::new("my_histogram").size(5);

        assert_eq!(agg.size, 5);
    }

    #[test]
    fn test_bucket_sort_aggregation_with_from() {
        let agg = BucketSortAggregation::new("my_histogram").from(10);

        assert_eq!(agg.from, 10);
    }

    #[test]
    fn test_bucket_sort_aggregation_with_gap_policy() {
        let agg = BucketSortAggregation::new("my_histogram").gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_bucket_sort_aggregation_execute_error() {
        let agg = BucketSortAggregation::new("my_histogram");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since bucket sort operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_bucket_sort_aggregation_serialization() {
        let agg = BucketSortAggregation::new("my_histogram")
            .sort(SortOption::desc("my_metric.value"))
            .size(5)
            .from(10)
            .gap_policy("insert_zeros");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("sort"));
        assert!(json.contains("size"));
        assert!(json.contains("from"));
        assert!(json.contains("buckets_path"));
        assert!(json.contains("gap_policy"));

        let deserialized: BucketSortAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram");
        assert_eq!(deserialized.size, 5);
        assert_eq!(deserialized.from, 10);
        assert_eq!(deserialized.gap_policy, "insert_zeros");
        assert_eq!(deserialized.sort.len(), 1);
    }
}
