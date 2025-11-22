//! Value count aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Value count aggregation configuration
/// Counts the number of values for a field
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ValueCountAggregation {
    /// Field to count values for
    pub field: String,
}

impl AggregationTrait for ValueCountAggregation {
    fn name(&self) -> &str {
        "value_count"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut count = 0;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                // Count non-null values
                if !field_value.is_null() {
                    // For arrays, count each element
                    if let Some(array) = field_value.as_array() {
                        count += array.len();
                    } else {
                        count += 1;
                    }
                }
            }
        }

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": count }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_count = 0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(value) = obj.get("value").and_then(|v| v.as_u64()) {
                        total_count += value as usize;
                    }
                }
            }
        }

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": total_count }),
        )))
    }
}

impl ValueCountAggregation {
    /// Create new value count aggregation
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
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: value }),
        )
    }

    #[test]
    fn test_value_count_aggregation_basic() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(20)),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 3);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_with_arrays() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!([1, 2, 3])),
            create_test_hit("2", "field", serde_json::json!([4, 5])),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 5); // 3 + 2
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_with_null() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::Value::Null),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 2); // null values are not counted
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_missing_field() {
        let hits = vec![
            create_test_hit("1", "other_field", serde_json::json!(10)),
            create_test_hit("2", "other_field", serde_json::json!(20)),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_empty_hits() {
        let hits: Vec<SearchHit> = vec![];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_merge() {
        let field_cache = FieldCache::new();
        let agg = ValueCountAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(20))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 2);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_empty_array() {
        let hits = vec![create_test_hit("1", "field", serde_json::json!([]))];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_mixed_types() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!("string")),
            create_test_hit("3", "field", serde_json::json!(true)),
            create_test_hit("4", "field", serde_json::json!({ "nested": "object" })),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 4); // All non-null values are counted
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_array_with_null() {
        let hits = vec![create_test_hit(
            "1",
            "field",
            serde_json::json!([1, null, 3, null]),
        )];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 4); // Array length, nulls are counted as elements
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_merge_multiple_shards() {
        let field_cache = FieldCache::new();
        let agg = ValueCountAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!([1, 2]))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!([3, 4, 5]))];
        let hits3 = vec![create_test_hit("3", "field", serde_json::json!(10))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();
        let result3 = agg.execute(&hits3, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2, result3]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 6); // 2 + 3 + 1
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_merge_empty_results() {
        let agg = ValueCountAggregation::new("field");

        let empty_result = AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": 0 }),
        ));

        let merged = agg.merge(&[empty_result]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_string_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!("value1")),
            create_test_hit("2", "field", serde_json::json!("value2")),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 2);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_value_count_aggregation_boolean_values() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(true)),
            create_test_hit("2", "field", serde_json::json!(false)),
        ];
        let field_cache = FieldCache::new();

        let agg = ValueCountAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 2);
        } else {
            panic!("Expected Metric result");
        }
    }
}
