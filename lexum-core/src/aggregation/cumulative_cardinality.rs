//! Cumulative Cardinality Aggregation
//!
//! Calculates cumulative cardinality (unique count) across buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Cumulative Cardinality Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CumulativeCardinalityAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>unique_users")
    pub buckets_path: String,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl CumulativeCardinalityAggregation {
    /// Create new cumulative cardinality aggregation
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

impl AggregationTrait for CumulativeCardinalityAggregation {
    fn name(&self) -> &str {
        "cumulative_cardinality"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Cumulative Cardinality Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Cumulative Cardinality Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge cumulative cardinality results by calculating cumulative cardinality across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract cardinality values from buckets (based on buckets_path)
        // 3. Calculate cumulative cardinality (union of all unique values up to current bucket)
        // 4. Return buckets with cumulative cardinality values

        // For now, return the first result as placeholder
        // Full implementation would calculate cumulative cardinality
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Calculate cumulative cardinality for buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn calculate_cumulative_cardinality(
    buckets: &[Bucket],
    _buckets_path: &str,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract cardinality values (e.g., "my_histogram>unique_users")
    // 2. Track unique values across buckets
    // 3. Calculate cumulative cardinality: bucket[i].cumulative_cardinality = |union(bucket[0..i].unique_values)|
    // 4. Add cumulative_cardinality to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cumulative_cardinality_aggregation() {
        let agg = CumulativeCardinalityAggregation::new("my_histogram>unique_users");

        assert_eq!(agg.buckets_path, "my_histogram>unique_users");
    }

    #[test]
    fn test_cumulative_cardinality_aggregation_with_format() {
        let agg = CumulativeCardinalityAggregation::new("my_histogram>unique_users").format("0");

        assert_eq!(agg.format, Some("0".to_string()));
    }

    #[test]
    fn test_cumulative_cardinality_aggregation_execute_error() {
        let agg = CumulativeCardinalityAggregation::new("my_histogram>unique_users");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since cumulative cardinality operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_cumulative_cardinality_aggregation_serialization() {
        let agg = CumulativeCardinalityAggregation::new("my_histogram>unique_users").format("0");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("format"));

        let deserialized: CumulativeCardinalityAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>unique_users");
        assert_eq!(deserialized.format, Some("0".to_string()));
    }
}
