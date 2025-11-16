//! Average aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Average aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AverageAggregation {
    /// Field to compute average on
    pub field: String,
}

impl AggregationTrait for AverageAggregation {
    fn name(&self) -> &str {
        "avg"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut values: Vec<f64> = Vec::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    values.push(num);
                } else if let Some(num) = field_value.as_i64() {
                    values.push(num as f64);
                }
            }
        }

        if values.is_empty() {
            return Ok(AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::json!({
                    "value": serde_json::Value::Null,
                    "sum": 0.0,
                    "count": 0
                }),
            )));
        }

        let sum: f64 = values.iter().sum();
        let count = values.len();
        let avg = sum / count as f64;

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({
                "value": avg,
                "sum": sum,
                "count": count
            }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_sum = 0.0;
        let mut total_count = 0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(sum) = obj.get("sum").and_then(|v| v.as_f64()) {
                        total_sum += sum;
                    }
                    if let Some(count) = obj.get("count").and_then(|v| v.as_u64()) {
                        total_count += count as usize;
                    }
                }
            }
        }

        if total_count == 0 {
            return Ok(AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::json!({
                    "value": serde_json::Value::Null,
                    "sum": 0.0,
                    "count": 0
                }),
            )));
        }

        let avg = total_sum / total_count as f64;

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({
                "value": avg,
                "sum": total_sum,
                "count": total_count
            }),
        )))
    }
}

impl AverageAggregation {
    /// Create new average aggregation
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
    fn test_average_aggregation_basic() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(20)),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - 20.0).abs() < 0.001);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_empty() {
        let hits: Vec<SearchHit> = vec![];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert!(metric_result.value["value"].is_null());
            assert_eq!(metric_result.value["sum"], 0.0);
            assert_eq!(metric_result.value["count"], 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_merge() {
        let field_cache = FieldCache::new();
        let agg = AverageAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(20))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - 15.0).abs() < 0.001);
            assert_eq!(metric_result.value["sum"], 30.0);
            assert_eq!(metric_result.value["count"], 2);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_missing_field() {
        let hits = vec![create_test_hit("1", "other_field", serde_json::json!(10))];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert!(metric_result.value["value"].is_null());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_negative_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(-10)),
            create_test_hit("2", "field", serde_json::json!(-20)),
            create_test_hit("3", "field", serde_json::json!(10)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - (-6.666666666666667)).abs() < 0.001);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_decimal_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10.5)),
            create_test_hit("2", "field", serde_json::json!(20.25)),
            create_test_hit("3", "field", serde_json::json!(30.75)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - 20.5).abs() < 0.001);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_zero_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(0)),
            create_test_hit("2", "field", serde_json::json!(0)),
            create_test_hit("3", "field", serde_json::json!(0)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert_eq!(avg, 0.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_mixed_int_float() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(20.5)),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - 20.166666666666668).abs() < 0.001);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_single_value() {
        let hits = vec![create_test_hit("1", "field", serde_json::json!(42))];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert_eq!(avg, 42.0);
            assert_eq!(metric_result.value["sum"], 42.0);
            assert_eq!(metric_result.value["count"], 1);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_merge_multiple_shards() {
        let field_cache = FieldCache::new();
        let agg = AverageAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(20))];
        let hits3 = vec![create_test_hit("3", "field", serde_json::json!(30))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();
        let result3 = agg.execute(&hits3, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2, result3]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert!((avg - 20.0).abs() < 0.001);
            assert_eq!(metric_result.value["sum"], 60.0);
            assert_eq!(metric_result.value["count"], 3);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_merge_with_empty() {
        let field_cache = FieldCache::new();
        let agg = AverageAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2: Vec<SearchHit> = vec![];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert_eq!(avg, 10.0);
            assert_eq!(metric_result.value["sum"], 10.0);
            assert_eq!(metric_result.value["count"], 1);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_average_aggregation_with_null_field() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::Value::Null),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = AverageAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let avg = metric_result.value["value"].as_f64().unwrap();
            assert_eq!(avg, 20.0); // Null values are ignored
            assert_eq!(metric_result.value["sum"], 40.0);
            assert_eq!(metric_result.value["count"], 2);
        } else {
            panic!("Expected Metric result");
        }
    }
}
