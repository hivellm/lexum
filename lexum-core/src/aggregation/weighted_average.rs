//! Weighted Average Aggregation
//!
//! Computes the weighted average of numeric values using a weight field.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Weighted Average Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeightedAverageAggregation {
    /// Field containing the values to average
    pub value_field: String,
    /// Field containing the weights
    pub weight_field: String,
    /// Value type hint (optional, for format support)
    #[serde(default)]
    pub value_type: Option<String>,
    /// Format for the result (optional)
    #[serde(default)]
    pub format: Option<String>,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

impl WeightedAverageAggregation {
    /// Create new weighted average aggregation
    pub fn new(value_field: impl Into<String>, weight_field: impl Into<String>) -> Self {
        Self {
            value_field: value_field.into(),
            weight_field: weight_field.into(),
            value_type: None,
            format: None,
            aggs: HashMap::new(),
        }
    }

    /// Set value type hint
    pub fn value_type(mut self, value_type: impl Into<String>) -> Self {
        self.value_type = Some(value_type.into());
        self
    }

    /// Set format for the result
    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
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

/// Weighted average result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WeightedAverageResult {
    /// Weighted average value
    pub value: Option<f64>,
    /// Sum of weights
    pub sum_of_weights: f64,
}

impl AggregationTrait for WeightedAverageAggregation {
    fn name(&self) -> &str {
        "weighted_avg"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut weighted_sum = 0.0;
        let mut sum_of_weights = 0.0;

        // Extract values and weights from fields
        for hit in hits {
            let value = hit.source.get(&self.value_field);
            let weight = hit.source.get(&self.weight_field);

            if let (Some(value_val), Some(weight_val)) = (value, weight) {
                if let (Some(value_num), Some(weight_num)) = (
                    extract_numeric_value(value_val),
                    extract_numeric_value(weight_val),
                ) {
                    // Skip if weight is zero or negative
                    if weight_num > 0.0 {
                        weighted_sum += value_num * weight_num;
                        sum_of_weights += weight_num;
                    }
                }
            }
        }

        let weighted_avg = if sum_of_weights > 0.0 {
            Some(weighted_sum / sum_of_weights)
        } else {
            None
        };

        let result = WeightedAverageResult {
            value: weighted_avg,
            sum_of_weights,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge weighted averages by combining weighted sums and sum of weights
        let mut total_weighted_sum = 0.0;
        let mut total_sum_of_weights = 0.0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(wa_result) =
                    serde_json::from_value::<WeightedAverageResult>(metric_result.value.clone())
                {
                    if let Some(value) = wa_result.value {
                        // Reconstruct weighted sum from average and sum of weights
                        total_weighted_sum += value * wa_result.sum_of_weights;
                    }
                    total_sum_of_weights += wa_result.sum_of_weights;
                }
            }
        }

        let merged_weighted_avg = if total_sum_of_weights > 0.0 {
            Some(total_weighted_sum / total_sum_of_weights)
        } else {
            None
        };

        let merged_result = WeightedAverageResult {
            value: merged_weighted_avg,
            sum_of_weights: total_sum_of_weights,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weighted_average_aggregation() {
        let agg = WeightedAverageAggregation::new("price", "quantity");

        assert_eq!(agg.value_field, "price");
        assert_eq!(agg.weight_field, "quantity");
    }

    #[test]
    fn test_weighted_average_aggregation_with_format() {
        let agg = WeightedAverageAggregation::new("price", "quantity")
            .value_type("double")
            .format("#.##");

        assert_eq!(agg.value_type, Some("double".to_string()));
        assert_eq!(agg.format, Some("#.##".to_string()));
    }

    #[test]
    fn test_weighted_average_aggregation_empty() {
        let agg = WeightedAverageAggregation::new("price", "quantity");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let wa: WeightedAverageResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(wa.value.is_none());
            assert_eq!(wa.sum_of_weights, 0.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_weighted_average_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = WeightedAverageAggregation::new("price", "quantity");
        let mut hits = vec![];

        // Create hits with price and quantity
        // Hit 1: price=10, quantity=2 -> weighted: 10*2 = 20
        // Hit 2: price=20, quantity=3 -> weighted: 20*3 = 60
        // Hit 3: price=30, quantity=1 -> weighted: 30*1 = 30
        // Total weighted: 110, Total weights: 6
        // Weighted average: 110/6 ≈ 18.33
        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "price": 10, "quantity": 2 }),
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "price": 20, "quantity": 3 }),
        ));
        hits.push(SearchHit::new(
            DocumentId::new("3"),
            Score::new(1.0),
            serde_json::json!({ "price": 30, "quantity": 1 }),
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let wa: WeightedAverageResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(wa.value.is_some());
            assert_eq!(wa.sum_of_weights, 6.0);
            // Weighted average should be approximately 18.33
            assert!((wa.value.unwrap() - 18.333333333333332).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_weighted_average_aggregation_zero_weight() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = WeightedAverageAggregation::new("price", "quantity");
        let mut hits = vec![];

        // Create hits with one zero weight
        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "price": 10, "quantity": 2 }),
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "price": 20, "quantity": 0 }), // Zero weight should be skipped
        ));
        hits.push(SearchHit::new(
            DocumentId::new("3"),
            Score::new(1.0),
            serde_json::json!({ "price": 30, "quantity": 1 }),
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let wa: WeightedAverageResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(wa.value.is_some());
            assert_eq!(wa.sum_of_weights, 3.0); // Only weights 2 and 1
            // Weighted average: (10*2 + 30*1) / 3 = 50/3 ≈ 16.67
            assert!((wa.value.unwrap() - 16.666666666666668).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_weighted_average_aggregation_missing_fields() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = WeightedAverageAggregation::new("price", "quantity");
        let mut hits = vec![];

        // Create hits with missing fields
        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "price": 10 }), // Missing quantity
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "quantity": 2 }), // Missing price
        ));
        hits.push(SearchHit::new(
            DocumentId::new("3"),
            Score::new(1.0),
            serde_json::json!({ "price": 30, "quantity": 1 }), // Both present
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let wa: WeightedAverageResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(wa.value.is_some());
            assert_eq!(wa.sum_of_weights, 1.0); // Only one valid hit
            assert_eq!(wa.value.unwrap(), 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_weighted_average_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = WeightedAverageAggregation::new("price", "quantity");

        // Create first result: price=10, quantity=2 -> weighted avg = 10
        let mut hits1 = vec![];
        hits1.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "price": 10, "quantity": 2 }),
        ));

        // Create second result: price=20, quantity=3 -> weighted avg = 20
        let mut hits2 = vec![];
        hits2.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "price": 20, "quantity": 3 }),
        ));

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let wa: WeightedAverageResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(wa.value.is_some());
            assert_eq!(wa.sum_of_weights, 5.0); // 2 + 3
            // Merged weighted average: (10*2 + 20*3) / 5 = 80/5 = 16
            assert!((wa.value.unwrap() - 16.0).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_weighted_average_aggregation_serialization() {
        let agg = WeightedAverageAggregation::new("price", "quantity")
            .value_type("double")
            .format("#.##");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("price"));
        assert!(json.contains("quantity"));
        assert!(json.contains("value_type"));
        assert!(json.contains("format"));

        let deserialized: WeightedAverageAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.value_field, "price");
        assert_eq!(deserialized.weight_field, "quantity");
        assert_eq!(deserialized.value_type, Some("double".to_string()));
        assert_eq!(deserialized.format, Some("#.##".to_string()));
    }
}
