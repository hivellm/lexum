//! Normalize Aggregation
//!
//! Normalizes metric values using various methods (rescale, percent, etc.).

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Normalization method
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum NormalizeMethod {
    /// Rescale to [0, 1] range
    Rescale,
    /// Percent of sum
    Percent,
    /// Percent of max
    PercentOfSum,
    /// Z-score normalization (mean=0, std=1)
    ZScore,
    /// Softmax normalization
    Softmax,
}

impl Default for NormalizeMethod {
    fn default() -> Self {
        NormalizeMethod::Rescale
    }
}

/// Normalize Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NormalizeAggregation {
    /// Buckets path (parent aggregation path, e.g., "my_histogram>_count")
    pub buckets_path: String,
    /// Normalization method (default: "rescale")
    #[serde(default)]
    pub method: NormalizeMethod,
    /// Format for the output value (optional)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

impl NormalizeAggregation {
    /// Create new normalize aggregation
    pub fn new(buckets_path: impl Into<String>) -> Self {
        Self {
            buckets_path: buckets_path.into(),
            method: NormalizeMethod::Rescale,
            format: None,
        }
    }

    /// Set normalization method
    pub fn method(mut self, method: NormalizeMethod) -> Self {
        self.method = method;
        self
    }

    /// Set format for output value
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }
}

impl AggregationTrait for NormalizeAggregation {
    fn name(&self) -> &str {
        "normalize"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Normalize Aggregation operates on parent aggregation results
        // It should be executed as a pipeline aggregation after the parent aggregation
        // This execute method is a placeholder - actual execution happens in pipeline processing
        use crate::error::Error;
        Err(Error::Config(
            "Normalize Aggregation must be executed as a pipeline aggregation on parent results"
                .to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge normalize results by normalizing values across merged buckets
        // In a full implementation, this would:
        // 1. Extract buckets from parent aggregation results
        // 2. Extract metric values from buckets (based on buckets_path)
        // 3. Apply normalization method:
        //    - Rescale: (value - min) / (max - min)
        //    - Percent: value / sum * 100
        //    - PercentOfSum: value / sum * 100
        //    - ZScore: (value - mean) / std_deviation
        //    - Softmax: exp(value) / sum(exp(values))
        // 4. Return buckets with normalized values

        // For now, return the first result as placeholder
        // Full implementation would normalize values
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            use crate::error::Error;
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

/// Normalize bucket values
/// This is a helper function that would be called during pipeline aggregation processing
pub fn normalize_buckets(
    buckets: &[Bucket],
    buckets_path: &str,
    method: NormalizeMethod,
) -> Result<Vec<Bucket>> {
    // Note: Full implementation would:
    // 1. Parse buckets_path to extract metric values
    // 2. Calculate normalization parameters based on method:
    //    - Rescale: find min and max
    //    - Percent/PercentOfSum: calculate sum
    //    - ZScore: calculate mean and std deviation
    //    - Softmax: calculate sum of exponentials
    // 3. Apply normalization to each bucket value
    // 4. Add normalized value to each bucket
    //
    // For now, return buckets as-is (placeholder)
    Ok(buckets.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_aggregation() {
        let agg = NormalizeAggregation::new("my_histogram>_count");

        assert_eq!(agg.buckets_path, "my_histogram>_count");
        assert_eq!(agg.method, NormalizeMethod::Rescale);
    }

    #[test]
    fn test_normalize_aggregation_with_method() {
        let agg = NormalizeAggregation::new("my_histogram>_count").method(NormalizeMethod::Percent);

        assert_eq!(agg.method, NormalizeMethod::Percent);
    }

    #[test]
    fn test_normalize_aggregation_with_format() {
        let agg = NormalizeAggregation::new("my_histogram>_count").format("0.00%");

        assert_eq!(agg.format, Some("0.00%".to_string()));
    }

    #[test]
    fn test_normalize_aggregation_execute_error() {
        let agg = NormalizeAggregation::new("my_histogram>_count");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache);

        // Should return error since normalize operates on parent results
        assert!(result.is_err());
    }

    #[test]
    fn test_normalize_aggregation_serialization() {
        let agg = NormalizeAggregation::new("my_histogram>_count")
            .method(NormalizeMethod::ZScore)
            .format("0.00");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("buckets_path"));
        assert!(json.contains("method"));
        assert!(json.contains("format"));

        let deserialized: NormalizeAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.buckets_path, "my_histogram>_count");
        assert_eq!(deserialized.method, NormalizeMethod::ZScore);
        assert_eq!(deserialized.format, Some("0.00".to_string()));
    }

    #[test]
    fn test_normalize_method_types() {
        assert_eq!(NormalizeMethod::Rescale, NormalizeMethod::Rescale);
        assert_eq!(NormalizeMethod::Percent, NormalizeMethod::Percent);
        assert_eq!(NormalizeMethod::PercentOfSum, NormalizeMethod::PercentOfSum);
        assert_eq!(NormalizeMethod::ZScore, NormalizeMethod::ZScore);
        assert_eq!(NormalizeMethod::Softmax, NormalizeMethod::Softmax);
    }
}
