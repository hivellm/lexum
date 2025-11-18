//! Serial Differencing Aggregation
//!
//! Calculates the difference between values at a specified lag.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Serial Differencing Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SerialDifferencingAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Lag (number of buckets to shift, default: 1)
    #[serde(default = "default_lag")]
    pub lag: usize,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Gap policy (default: "skip")
    #[serde(default = "default_gap_policy")]
    pub gap_policy: String,
}

fn default_lag() -> usize {
    1
}

fn default_gap_policy() -> String {
    "skip".to_string()
}

impl SerialDifferencingAggregation {
    /// Create new serial differencing aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            buckets_path: buckets_path.into(),
            lag: 1,
            format: None,
            gap_policy: "skip".to_string(),
        }
    }

    /// Set lag (number of buckets to shift)
    pub fn lag(mut self, lag: usize) -> Self {
        self.lag = lag;
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

impl AggregationTrait for SerialDifferencingAggregation {
    fn name(&self) -> &str {
        "serial_differencing"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Serial Differencing Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Serial Differencing Aggregation must be executed as a pipeline aggregation on parent results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge serial differencing results by calculating differences across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. Calculate difference: bucket[i].difference = bucket[i].value - bucket[i-lag].value
        // 4. Apply gap policy
        // 5. Return buckets with difference values

        // For now, return the first result as placeholder
        // Full implementation would calculate differences
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Calculate serial differencing for buckets
/// This is a helper function that would be called during pipeline aggregation processing
pub fn calculate_serial_differencing(
    buckets: &[Bucket],
    _buckets_path: &str,
    _lag: usize,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values (e.g., "my_histogram>_count")
    // 2. Calculate difference: value[i] - value[i-lag]
    // 3. Handle gap policy (skip or insert zeros for missing buckets)
    // 4. Add difference to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serial_differencing_aggregation() {
        let agg = SerialDifferencingAggregation::new("my_histogram>_count");

        assert_eq!(agg.buckets_path, "my_histogram>_count");
        assert_eq!(agg.lag, 1);
        assert_eq!(agg.gap_policy, "skip");
    }

    #[test]
    fn test_serial_differencing_aggregation_with_lag() {
        let agg = SerialDifferencingAggregation::new("my_histogram>_count").lag(3);

        assert_eq!(agg.lag, 3);
    }

    #[test]
    fn test_serial_differencing_aggregation_with_format() {
        let agg = SerialDifferencingAggregation::new("my_histogram>_count").format("0.00");

        assert_eq!(agg.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_serial_differencing_aggregation_with_gap_policy() {
        let agg =
            SerialDifferencingAggregation::new("my_histogram>_count").gap_policy("insert_zeros");

        assert_eq!(agg.gap_policy, "insert_zeros");
    }

    #[test]
    fn test_serial_differencing_aggregation_execute_error() {
        let agg = SerialDifferencingAggregation::new("my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since serial differencing operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_serial_differencing_aggregation_serialization() {
        let agg = SerialDifferencingAggregation::new("my_histogram>_count")
            .lag(3)
            .format("0.00")
            .gap_policy("insert_zeros");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("lag"));
        assert!(json.contains("format"));
        assert!(json.contains("gap_policy"));

        let deserialized: SerialDifferencingAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.lag, 3);
        assert_eq!(deserialized.format, Some("0.00".to_string()));
        assert_eq!(deserialized.gap_policy, "insert_zeros");
    }
}
