//! Max aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Max aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MaxAggregation {
    /// Field to compute maximum on
    pub field: String,
}

impl AggregationTrait for MaxAggregation {
    fn name(&self) -> &str {
        "max"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut max_value = f64::NEG_INFINITY;
        let mut has_values = false;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    max_value = max_value.max(num);
                    has_values = true;
                } else if let Some(num) = field_value.as_i64() {
                    max_value = max_value.max(num as f64);
                    has_values = true;
                }
            }
        }

        let result_value = if has_values {
            serde_json::json!(max_value)
        } else {
            serde_json::Value::Null
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": result_value }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut max_value = f64::NEG_INFINITY;
        let mut has_values = false;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(value) = obj.get("value") {
                        if let Some(num) = value.as_f64() {
                            max_value = max_value.max(num);
                            has_values = true;
                        }
                    }
                }
            }
        }

        let result_value = if has_values {
            serde_json::json!(max_value)
        } else {
            serde_json::Value::Null
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": result_value }),
        )))
    }
}

impl MaxAggregation {
    /// Create new max aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: serde_json::Value) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: value }),
        }
    }

    #[test]
    fn test_max_aggregation_basic() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(30)),
            create_test_hit("3", "field", serde_json::json!(20)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_empty() {
        let hits: Vec<SearchHit> = vec![];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert!(metric_result.value["value"].is_null());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_merge() {
        let field_cache = FieldCache::new();
        let agg = MaxAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(30))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_negative_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(-10)),
            create_test_hit("2", "field", serde_json::json!(-20)),
            create_test_hit("3", "field", serde_json::json!(-5)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], -5.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_decimal_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10.5)),
            create_test_hit("2", "field", serde_json::json!(20.25)),
            create_test_hit("3", "field", serde_json::json!(30.75)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 30.75);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_zero_value() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(-10)),
            create_test_hit("2", "field", serde_json::json!(0)),
            create_test_hit("3", "field", serde_json::json!(-20)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 0.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_mixed_int_float() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(25.5)),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_single_value() {
        let hits = vec![create_test_hit("1", "field", serde_json::json!(42))];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 42.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_merge_multiple_shards() {
        let field_cache = FieldCache::new();
        let agg = MaxAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(30))];
        let hits3 = vec![create_test_hit("3", "field", serde_json::json!(20))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();
        let result3 = agg.execute(&hits3, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2, result3]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_missing_field() {
        let hits = vec![create_test_hit("1", "other_field", serde_json::json!(10))];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert!(metric_result.value["value"].is_null());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_max_aggregation_with_null_field() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::Value::Null),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = MaxAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 30.0); // Null values are ignored
        } else {
            panic!("Expected Metric result");
        }
    }
}
