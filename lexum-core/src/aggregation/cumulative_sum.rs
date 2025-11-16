//! Cumulative Sum Aggregation
//!
//! Calculates cumulative sum of metric values across buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Cumulative Sum Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CumulativeSumAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl CumulativeSumAggregation {
    /// Create new cumulative sum aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            buckets_path: buckets_path.into(),
            format: None,
        }
    }

    /// Set format for output value
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

impl AggregationTrait for CumulativeSumAggregation {
    fn name(&self) -> &str {
        "cumulative_sum"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Cumulative Sum Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Cumulative Sum Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge cumulative sum results by calculating cumulative sum across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. Calculate cumulative sum for each bucket
        // 4. Return buckets with cumulative sum values

        // For now, return the first result as placeholder
        // Full implementation would calculate cumulative sums
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Calculate cumulative sum for buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn calculate_cumulative_sum(buckets: &[Bucket], buckets_path: &str) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values (e.g., "my_histogram>_count")
    // 2. Calculate cumulative sum: bucket[i].cumulative_sum = sum(bucket[0..i].value)
    // 3. Add cumulative_sum to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulative_sum_aggregation() {
        let agg = CumulativeSumAggregation::new("my_histogram>_count");

        assert_eq!(agg.buckets_path, "my_histogram>_count");
    }

    #[test]
    fn test_cumulative_sum_aggregation_with_format() {
        let agg = CumulativeSumAggregation::new("my_histogram>_count").format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_cumulative_sum_aggregation_execute_error() {
        let agg = CumulativeSumAggregation::new("my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since cumulative sum operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_cumulative_sum_aggregation_serialization() {
        let agg = CumulativeSumAggregation::new("my_histogram>_count").format("0.00");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("format"));

        let deserialized: CumulativeSumAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.format, Some("0.00".to_string()));
    }
}
