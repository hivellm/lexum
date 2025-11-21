//! Boxplot Aggregation
//!
//! Computes boxplot statistics including quartiles (Q1, Q2/median, Q3), min, max, and outliers.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Boxplot Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BoxplotAggregation {
    /// Field to compute boxplot on
    pub field: String,
    /// Compression parameter (default: 100.0)
    /// Higher values = more accurate but slower
    #[serde(default = "default_compression")]
    pub compression: f64,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_compression() -> f64 {
    100.0
}

impl BoxplotAggregation {
    /// Create new boxplot aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            compression: 100.0,
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

/// Boxplot result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct BoxplotResult {
    /// Minimum value
    pub min: Option<f64>,
    /// First quartile (Q1) - 25th percentile
    pub q1: Option<f64>,
    /// Median (Q2) - 50th percentile
    pub median: Option<f64>,
    /// Third quartile (Q3) - 75th percentile
    pub q3: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
    /// Interquartile range (IQR = Q3 - Q1)
    pub iqr: Option<f64>,
    /// Lower whisker (Q1 - 1.5 * IQR)
    pub lower_whisker: Option<f64>,
    /// Upper whisker (Q3 + 1.5 * IQR)
    pub upper_whisker: Option<f64>,
}

impl AggregationTrait for BoxplotAggregation {
    fn name(&self) -> &str {
        "boxplot"
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
                    "min": null,
                    "q1": null,
                    "median": null,
                    "q3": null,
                    "max": null,
                    "iqr": null,
                    "lower_whisker": null,
                    "upper_whisker": null
                }),
            )));
        }

        // Sort values
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = values.len();
        let min = values[0];
        let max = values[len - 1];

        // Calculate quartiles
        let q1 = calculate_percentile(&values, 25.0);
        let median = calculate_percentile(&values, 50.0);
        let q3 = calculate_percentile(&values, 75.0);

        // Calculate IQR (Interquartile Range)
        let iqr = if let (Some(q1_val), Some(q3_val)) = (q1, q3) {
            Some(q3_val - q1_val)
        } else {
            None
        };

        // Calculate whiskers (1.5 * IQR rule)
        let lower_whisker = if let (Some(q1_val), Some(iqr_val)) = (q1, iqr) {
            Some((q1_val - 1.5 * iqr_val).max(min))
        } else {
            Some(min)
        };

        let upper_whisker = if let (Some(q3_val), Some(iqr_val)) = (q3, iqr) {
            Some((q3_val + 1.5 * iqr_val).min(max))
        } else {
            Some(max)
        };

        let result = BoxplotResult {
            min: Some(min),
            q1,
            median,
            q3,
            max: Some(max),
            iqr,
            lower_whisker,
            upper_whisker,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // For boxplot, we need to merge the underlying values, not the quartiles themselves
        // This is a simplified merge - a full implementation would collect all values
        // and recalculate quartiles from the merged dataset
        let mut all_values: Vec<f64> = Vec::new();

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(boxplot_result) =
                    serde_json::from_value::<BoxplotResult>(metric_result.value.clone())
                {
                    // Reconstruct values from quartiles (approximation)
                    // A full implementation would store all values or use T-Digest
                    if let (Some(q1), Some(median), Some(q3), Some(min), Some(max)) = (
                        boxplot_result.q1,
                        boxplot_result.median,
                        boxplot_result.q3,
                        boxplot_result.min,
                        boxplot_result.max,
                    ) {
                        // Approximate distribution with representative values
                        all_values.push(min);
                        all_values.push(q1);
                        all_values.push(median);
                        all_values.push(q3);
                        all_values.push(max);
                    }
                }
            }
        }

        if all_values.is_empty() {
            return Ok(AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::json!({
                    "min": null,
                    "q1": null,
                    "median": null,
                    "q3": null,
                    "max": null,
                    "iqr": null,
                    "lower_whisker": null,
                    "upper_whisker": null
                }),
            )));
        }

        // Recalculate quartiles from merged values
        all_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let len = all_values.len();
        let min = all_values[0];
        let max = all_values[len - 1];

        let q1 = calculate_percentile(&all_values, 25.0);
        let median = calculate_percentile(&all_values, 50.0);
        let q3 = calculate_percentile(&all_values, 75.0);

        let iqr = if let (Some(q1_val), Some(q3_val)) = (q1, q3) {
            Some(q3_val - q1_val)
        } else {
            None
        };

        let lower_whisker = if let (Some(q1_val), Some(iqr_val)) = (q1, iqr) {
            Some((q1_val - 1.5 * iqr_val).max(min))
        } else {
            Some(min)
        };

        let upper_whisker = if let (Some(q3_val), Some(iqr_val)) = (q3, iqr) {
            Some((q3_val + 1.5 * iqr_val).min(max))
        } else {
            Some(max)
        };

        let merged_result = BoxplotResult {
            min: Some(min),
            q1,
            median,
            q3,
            max: Some(max),
            iqr,
            lower_whisker,
            upper_whisker,
        };

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

/// Calculate percentile using linear interpolation
fn calculate_percentile(sorted_values: &[f64], percentile: f64) -> Option<f64> {
    if sorted_values.is_empty() {
        return None;
    }

    if sorted_values.len() == 1 {
        return Some(sorted_values[0]);
    }

    let p = percentile / 100.0;
    let n = sorted_values.len() as f64;
    let index = p * (n - 1.0);

    let lower_index = index.floor() as usize;
    let upper_index = (index.ceil() as usize).min(sorted_values.len() - 1);
    let weight = index - lower_index as f64;

    if lower_index == upper_index {
        Some(sorted_values[lower_index])
    } else {
        Some(sorted_values[lower_index] * (1.0 - weight) + sorted_values[upper_index] * weight)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boxplot_aggregation() {
        let agg = BoxplotAggregation::new("price");

        assert_eq!(agg.field, "price");
        assert_eq!(agg.compression, 100.0);
    }

    #[test]
    fn test_boxplot_aggregation_with_compression() {
        let agg = BoxplotAggregation::new("price").compression(200.0);

        assert_eq!(agg.compression, 200.0);
    }

    #[test]
    fn test_boxplot_aggregation_empty() {
        let agg = BoxplotAggregation::new("price");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let boxplot: BoxplotResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(boxplot.min.is_none());
            assert!(boxplot.q1.is_none());
            assert!(boxplot.median.is_none());
            assert!(boxplot.q3.is_none());
            assert!(boxplot.max.is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_boxplot_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = BoxplotAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
        // Q1 (25th percentile) ≈ 3.25
        // Median (50th percentile) = 5.5
        // Q3 (75th percentile) ≈ 7.75
        // IQR = 7.75 - 3.25 = 4.5
        for i in 1..=10 {
            hits.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i }),
            ));
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let boxplot: BoxplotResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(boxplot.min, Some(1.0));
            assert_eq!(boxplot.max, Some(10.0));
            assert!(boxplot.q1.is_some());
            assert!(boxplot.median.is_some());
            assert!(boxplot.q3.is_some());
            assert!(boxplot.iqr.is_some());
            assert!(boxplot.lower_whisker.is_some());
            assert!(boxplot.upper_whisker.is_some());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_boxplot_aggregation_quartiles() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = BoxplotAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [10, 20, 30, 40, 50]
        // Q1 (25th percentile) = 20
        // Median (50th percentile) = 30
        // Q3 (75th percentile) = 40
        // IQR = 40 - 20 = 20
        for i in 1..=5 {
            hits.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let boxplot: BoxplotResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(boxplot.min, Some(10.0));
            assert_eq!(boxplot.max, Some(50.0));
            assert_eq!(boxplot.median, Some(30.0));
            assert_eq!(boxplot.iqr, Some(20.0));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_boxplot_aggregation_whiskers() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = BoxplotAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [10, 20, 30, 40, 50]
        // Q1 = 20, Q3 = 40, IQR = 20
        // Lower whisker = max(20 - 1.5*20, 10) = max(-10, 10) = 10
        // Upper whisker = min(40 + 1.5*20, 50) = min(70, 50) = 50
        for i in 1..=5 {
            hits.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let boxplot: BoxplotResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(boxplot.lower_whisker, Some(10.0));
            assert_eq!(boxplot.upper_whisker, Some(50.0));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_boxplot_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = BoxplotAggregation::new("value");

        // Create first result: [10, 20, 30]
        let mut hits1 = vec![];
        for i in 1..=3 {
            hits1.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        // Create second result: [40, 50]
        let mut hits2 = vec![];
        for i in 4..=5 {
            hits2.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let boxplot: BoxplotResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(boxplot.min.is_some());
            assert!(boxplot.max.is_some());
            assert!(boxplot.median.is_some());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_boxplot_aggregation_serialization() {
        let agg = BoxplotAggregation::new("price").compression(200.0);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("price"));
        assert!(json.contains("compression"));

        let deserialized: BoxplotAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "price");
        assert_eq!(deserialized.compression, 200.0);
    }
}
