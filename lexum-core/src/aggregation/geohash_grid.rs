//! Geohash Grid Aggregation
//!
//! Groups documents into geohash grid cells based on geo_point field values.

use crate::query::GeoPoint;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Geohash Grid Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeohashGridAggregation {
    /// Field containing geo_point values
    pub field: String,
    /// Geohash precision (1-12, default: 5)
    /// Higher precision = smaller grid cells
    #[serde(default = "default_precision")]
    pub precision: u8,
    /// Maximum number of buckets to return
    #[serde(default = "default_size")]
    pub size: u32,
    /// Minimum number of documents per bucket
    #[serde(default)]
    pub min_doc_count: Option<u64>,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_precision() -> u8 {
    5
}

fn default_size() -> u32 {
    10000
}

impl GeohashGridAggregation {
    /// Create new geohash grid aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            precision: 5,
            size: 10000,
            min_doc_count: None,
            aggs: HashMap::new(),
        }
    }

    /// Set geohash precision (1-12)
    pub fn precision(mut self, precision: u8) -> Self {
        self.precision = precision.min(12).max(1);
        self
    }

    /// Set maximum number of buckets
    pub fn size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    /// Set minimum document count per bucket
    pub fn min_doc_count(mut self, count: u64) -> Self {
        self.min_doc_count = Some(count);
        self
    }

    /// Add sub-aggregation
    pub fn agg(
        mut self,
        name: impl Into<String>,
        agg: crate::aggregation::AggregationSpec,
    ) -> Self {
        self.aggs.insert(name.into(), agg);
        self
    }
}

impl AggregationTrait for GeohashGridAggregation {
    fn name(&self) -> &str {
        "geohash_grid"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Geohash encoding library (e.g., geohash-rust)
        // 2. Extracting geo_point values from documents
        // 3. Encoding each point to geohash with specified precision
        // 4. Grouping documents by geohash cell
        // 5. Filtering by min_doc_count if specified
        // 6. Limiting to size buckets
        // 7. Executing sub-aggregations for each bucket
        //
        // For now, return empty buckets as placeholder
        let buckets: Vec<Bucket> = Vec::new();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> crate::error::Result<AggregationResult> {
        // Merge geohash grid results by combining buckets from all shards
        let mut merged_buckets: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    let key = bucket.key.to_string();
                    *merged_buckets.entry(key).or_insert(0) += bucket.doc_count;
                }
            }
        }

        let buckets: Vec<Bucket> = merged_buckets
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }
}

/// Geohash grid bucket result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeohashGridBucket {
    /// Geohash key (cell identifier)
    pub key: String,
    /// Number of documents in this cell
    pub doc_count: u64,
    /// Sub-aggregation results
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::result::AggregationResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geohash_grid_aggregation() {
        let agg = GeohashGridAggregation::new("location");

        assert_eq!(agg.field, "location");
        assert_eq!(agg.precision, 5);
        assert_eq!(agg.size, 10000);
    }

    #[test]
    fn test_geohash_grid_aggregation_with_precision() {
        let agg = GeohashGridAggregation::new("location").precision(7);

        assert_eq!(agg.precision, 7);
    }

    #[test]
    fn test_geohash_grid_aggregation_with_size() {
        let agg = GeohashGridAggregation::new("location").size(100);

        assert_eq!(agg.size, 100);
    }

    #[test]
    fn test_geohash_grid_aggregation_with_min_doc_count() {
        let agg = GeohashGridAggregation::new("location").min_doc_count(5);

        assert_eq!(agg.min_doc_count, Some(5));
    }

    #[test]
    fn test_geohash_grid_aggregation_precision_clamp() {
        let agg = GeohashGridAggregation::new("location").precision(20);

        assert_eq!(agg.precision, 12); // Clamped to max 12
    }

    #[test]
    fn test_geohash_grid_aggregation_serialization() {
        let agg = GeohashGridAggregation::new("location")
            .precision(6)
            .size(500)
            .min_doc_count(10);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("location"));
        assert!(json.contains("precision"));
        assert!(json.contains("size"));
        assert!(json.contains("min_doc_count"));

        let deserialized: GeohashGridAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert_eq!(deserialized.precision, 6);
        assert_eq!(deserialized.size, 500);
        assert_eq!(deserialized.min_doc_count, Some(10));
    }
}
