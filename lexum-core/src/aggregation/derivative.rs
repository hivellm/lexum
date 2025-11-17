//! Derivative Aggregation
//!
//! Calculates the rate of change (derivative) of metric values across buckets.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Derivative Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DerivativeAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Unit for the derivative (optional, e.g., "1s", "1m", "1h", "1d")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl DerivativeAggregation {
    /// Create new derivative aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            buckets_path: buckets_path.into(),
            unit: None,
            format: None,
            gap_policy: "skip".to_string(),
        }
    }

    /// Set unit for derivative calculation
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Set format for output value
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set gap policy ("skip" or "insert_zeros")
    pub fn gap_policy(mut self, gap_policy: impl Into<String>) -> Self {
        self.gap_policy = gap_policy.into();
        self
    }
}

impl AggregationTrait for DerivativeAggregation {
    fn name(&self) -> &str {
        "derivative"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Derivative Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Derivative Aggregation must be executed as a pipeline aggregation on parent results"
                .to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge derivative results by calculating derivative across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. Calculate derivative: bucket[i].derivative = (bucket[i].value - bucket[i-1].value) / (bucket[i].key - bucket[i-1].key)
        // 4. Apply unit conversion if specified
        // 5. Return buckets with derivative values

        // For now, return the first result as placeholder
        // Full implementation would calculate derivatives
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Calculate derivative for buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn calculate_derivative(
    buckets: &[Bucket],
    buckets_path: &str,
    unit: Option<&str>,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values (e.g., "my_histogram>_count")
    // 2. Extract bucket keys (for time-based histograms, these would be timestamps)
    // 3. Calculate derivative: (value[i] - value[i-1]) / (key[i] - key[i-1])
    // 4. Apply unit conversion if specified (e.g., convert to per second)
    // 5. Handle gap policy (skip or insert zeros for missing buckets)
    // 6. Add derivative to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derivative_aggregation() {
        let agg = DerivativeAggregation::new("my_histogram>_count");

        assert_eq!(agg.buckets_path, "my_histogram>_count");
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_derivative_aggregation_with_unit() {
        let agg = DerivativeAggregation::new("my_histogram>_count").unit("1s");

        assert_eq!(agg.unit, Some("1s".to_string()));
    }

    #[test]
    fn test_derivative_aggregation_with_format() {
        let agg = DerivativeAggregation::new("my_histogram>_count").format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_derivative_aggregation_with_gap_policy() {
        let agg = DerivativeAggregation::new("my_histogram>_count").gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_derivative_aggregation_execute_error() {
        let agg = DerivativeAggregation::new("my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since derivative operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_derivative_aggregation_serialization() {
        let agg = DerivativeAggregation::new("my_histogram>_count")
            .unit("1s")
            .format("0.00")
            .gap_policy("insert_zeros");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("unit"));
        assert!(json.contains("format"));
        assert!(json.contains("gap_policy"));

        let deserialized: DerivativeAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.unit, Some("1s".to_string()));
        assert_eq!(deserialized.format, Some("0.00".to_string()));
        assert_eq!(deserialized.gap_policy, "insert_zeros");
    }
}
