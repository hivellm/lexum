//! Median Absolute Deviation Aggregation
//!
//! Computes the median absolute deviation (MAD) of numeric values.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Median Absolute Deviation Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MedianAbsoluteDeviationAggregation {
    /// Field to compute MAD on
    pub field: String,
    /// Compression parameter (default: 1000.0)
    /// Higher values = more accurate but slower
    #[serde(default = "default_compression")]
    pub compression: f64,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_compression() -> f64 {
    1000.0
}

impl MedianAbsoluteDeviationAggregation {
    /// Create new median absolute deviation aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            compression: 1000.0,
            aggs: HashMap::new(),
        }
    }

    /// Set compression parameter
    pub fn compression(mut self, compression: f64) -> Self {
        self.compression = compression;
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

/// Median absolute deviation result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MedianAbsoluteDeviationResult {
    /// Median absolute deviation value
    pub value: Option<f64>,
}

impl AggregationTrait for MedianAbsoluteDeviationAggregation {
    fn name(&self) -> &str {
        "median_absolute_deviation"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Extract numeric values from field
        let mut values: Vec<f64> = Vec::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num_value) = extract_numeric_value(field_value) {
                    values.push(num_value);
                }
            }
        }

        if values.is_empty() {
            return Ok(AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::json!({
                    "value": null
                }),
            )));
        }

        // Calculate median
        let median = calculate_median(&values);

        // Calculate absolute deviations from median
        let mut deviations: Vec<f64> = values.iter().map(|v| (v - median).abs()).collect();

        // Calculate median of absolute deviations (MAD)
        let mad = calculate_median(&deviations);

        let result = MedianAbsoluteDeviationResult { value: Some(mad) };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // For MAD, we need to merge the underlying values, not the MAD values themselves
        // This is a simplified merge - a full implementation would collect all values
        // and recalculate MAD from the merged dataset
        let mut all_mads: Vec<f64> = Vec::new();

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(mad_result) = serde_json::from_value::<MedianAbsoluteDeviationResult>(
                    metric_result.value.clone(),
                ) {
                    if let Some(mad_value) = mad_result.value {
                        all_mads.push(mad_value);
                    }
                }
            }
        }

        // For now, return the median of MADs as an approximation
        // A full implementation would merge all underlying values and recalculate
        let merged_mad = if all_mads.is_empty() {
            None
        } else {
            Some(calculate_median(&all_mads))
        };

        let merged_result = MedianAbsoluteDeviationResult { value: merged_mad };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(merged_result)?,
        )))
    }
}

/// Extract numeric value from JSON value
fn extract_numeric_value(value: &JsonValue) -> Option<f64> {
    match value {
        JsonValue::Number(n) => n.as_f64(),
        JsonValue::String(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Calculate median of a sorted vector
fn calculate_median(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let len = sorted.len();
    if len % 2 == 0 {
        // Even number of elements: average of two middle elements
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        // Odd number of elements: middle element
        sorted[len / 2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_median_absolute_deviation_aggregation() {
        let agg = MedianAbsoluteDeviationAggregation::new("price");

        assert_eq!(agg.field, "price");
        assert_eq!(agg.compression, 1000.0);
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_with_compression() {
        let agg = MedianAbsoluteDeviationAggregation::new("price").compression(500.0);

        assert_eq!(agg.compression, 500.0);
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_empty() {
        let agg = MedianAbsoluteDeviationAggregation::new("price");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let mad: MedianAbsoluteDeviationResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(mad.value.is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = MedianAbsoluteDeviationAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [1, 2, 3, 4, 5]
        // Median = 3
        // Deviations from median: [2, 1, 0, 1, 2]
        // MAD = median of deviations = 1
        for i in 1..=5 {
            hits.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32),
                source: serde_json::json!({ "value": i }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let mad: MedianAbsoluteDeviationResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(mad.value.is_some());
            // MAD should be approximately 1.0
            assert!((mad.value.unwrap() - 1.0).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_odd_count() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = MedianAbsoluteDeviationAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [10, 20, 30, 40, 50, 60, 70]
        // Median = 40
        // Deviations: [30, 20, 10, 0, 10, 20, 30]
        // MAD = median of deviations = 20
        for i in 1..=7 {
            hits.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let mad: MedianAbsoluteDeviationResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(mad.value.is_some());
            // MAD should be approximately 20.0
            assert!((mad.value.unwrap() - 20.0).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_even_count() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = MedianAbsoluteDeviationAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [10, 20, 30, 40]
        // Median = (20 + 30) / 2 = 25
        // Deviations: [15, 5, 5, 15]
        // Sorted deviations: [5, 5, 15, 15]
        // MAD = (5 + 15) / 2 = 10
        for i in 1..=4 {
            hits.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let mad: MedianAbsoluteDeviationResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(mad.value.is_some());
            // MAD should be approximately 10.0
            assert!((mad.value.unwrap() - 10.0).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = MedianAbsoluteDeviationAggregation::new("value");

        // Create first result: [1, 2, 3] -> MAD ≈ 1
        let mut hits1 = vec![];
        for i in 1..=3 {
            hits1.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32),
                source: serde_json::json!({ "value": i }),
            });
        }

        // Create second result: [4, 5, 6] -> MAD ≈ 1
        let mut hits2 = vec![];
        for i in 4..=6 {
            hits2.push(SearchHit {
                id: DocumentId::new(&i.to_string()),
                score: Score::new(i as f32),
                source: serde_json::json!({ "value": i }),
            });
        }

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let mad: MedianAbsoluteDeviationResult =
                serde_json::from_value(metric_result.value).unwrap();
            assert!(mad.value.is_some());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_median_absolute_deviation_aggregation_serialization() {
        let agg = MedianAbsoluteDeviationAggregation::new("price").compression(500.0);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("price"));
        assert!(json.contains("compression"));

        let deserialized: MedianAbsoluteDeviationAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "price");
        assert_eq!(deserialized.compression, 500.0);
    }
}
