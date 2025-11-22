//! Stats aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Stats aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatsAggregation {
    /// Field to compute stats on
    pub field: String,
}

impl AggregationTrait for StatsAggregation {
    fn name(&self) -> &str {
        "stats"
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
                    "count": 0,
                    "min": null,
                    "max": null,
                    "avg": null,
                    "sum": 0.0
                }),
            )));
        }

        let count = values.len();
        let min = values.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let sum: f64 = values.iter().sum();
        let avg = sum / count as f64;

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({
                "count": count,
                "min": min,
                "max": max,
                "avg": avg,
                "sum": sum
            }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_count = 0;
        let mut total_sum = 0.0;
        let mut all_mins: Vec<f64> = Vec::new();
        let mut all_maxs: Vec<f64> = Vec::new();

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(count) = obj.get("count").and_then(|v| v.as_u64()) {
                        total_count += count as usize;
                    }
                    if let Some(sum) = obj.get("sum").and_then(|v| v.as_f64()) {
                        total_sum += sum;
                    }
                    if let Some(min) = obj.get("min").and_then(|v| v.as_f64()) {
                        all_mins.push(min);
                    }
                    if let Some(max) = obj.get("max").and_then(|v| v.as_f64()) {
                        all_maxs.push(max);
                    }
                }
            }
        }

        let min = all_mins.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max = all_maxs.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        let avg = if total_count > 0 {
            total_sum / total_count as f64
        } else {
            0.0
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({
                "count": total_count,
                "min": if min == f64::INFINITY { JsonValue::Null } else { JsonValue::Number(serde_json::Number::from_f64(min).unwrap()) },
                "max": if max == f64::NEG_INFINITY { JsonValue::Null } else { JsonValue::Number(serde_json::Number::from_f64(max).unwrap()) },
                "avg": avg,
                "sum": total_sum
            }),
        )))
    }
}

impl StatsAggregation {
    /// Create new stats aggregation
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

    fn create_test_hit_numeric(id: &str, field: &str, value: f64) -> SearchHit {
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: value }),
        )
    }

    fn create_test_hit_integer(id: &str, field: &str, value: i64) -> SearchHit {
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: value }),
        )
    }

    #[test]
    fn test_stats_aggregation_basic() {
        let agg = StatsAggregation::new("price");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 10.0),
            create_test_hit_numeric("2", "price", 20.0),
            create_test_hit_numeric("3", "price", 30.0),
            create_test_hit_numeric("4", "price", 40.0),
            create_test_hit_numeric("5", "price", 50.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                assert_eq!(obj.get("count").and_then(|v| v.as_u64()), Some(5));
                assert_eq!(obj.get("min").and_then(|v| v.as_f64()), Some(10.0));
                assert_eq!(obj.get("max").and_then(|v| v.as_f64()), Some(50.0));
                assert_eq!(obj.get("sum").and_then(|v| v.as_f64()), Some(150.0));
                assert_eq!(obj.get("avg").and_then(|v| v.as_f64()), Some(30.0));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_stats_aggregation_empty() {
        let agg = StatsAggregation::new("price");
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                assert_eq!(obj.get("count").and_then(|v| v.as_u64()), Some(0));
                assert_eq!(obj.get("min"), Some(&JsonValue::Null));
                assert_eq!(obj.get("max"), Some(&JsonValue::Null));
                assert_eq!(obj.get("sum").and_then(|v| v.as_f64()), Some(0.0));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_stats_aggregation_integer_values() {
        let agg = StatsAggregation::new("count");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_integer("1", "count", 5),
            create_test_hit_integer("2", "count", 10),
            create_test_hit_integer("3", "count", 15),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                assert_eq!(obj.get("count").and_then(|v| v.as_u64()), Some(3));
                assert_eq!(obj.get("min").and_then(|v| v.as_f64()), Some(5.0));
                assert_eq!(obj.get("max").and_then(|v| v.as_f64()), Some(15.0));
                assert_eq!(obj.get("sum").and_then(|v| v.as_f64()), Some(30.0));
                assert_eq!(obj.get("avg").and_then(|v| v.as_f64()), Some(10.0));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_stats_aggregation_merge() {
        let agg = StatsAggregation::new("price");
        let field_cache = FieldCache::new();

        // Create two separate results
        let hits1 = vec![
            create_test_hit_numeric("1", "price", 10.0),
            create_test_hit_numeric("2", "price", 20.0),
        ];

        let hits2 = vec![
            create_test_hit_numeric("3", "price", 30.0),
            create_test_hit_numeric("4", "price", 40.0),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            if let Some(obj) = metric_result.value.as_object() {
                assert_eq!(obj.get("count").and_then(|v| v.as_u64()), Some(4));
                assert_eq!(obj.get("min").and_then(|v| v.as_f64()), Some(10.0));
                assert_eq!(obj.get("max").and_then(|v| v.as_f64()), Some(40.0));
                assert_eq!(obj.get("sum").and_then(|v| v.as_f64()), Some(100.0));
                assert_eq!(obj.get("avg").and_then(|v| v.as_f64()), Some(25.0));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_stats_aggregation_with_missing_values() {
        let agg = StatsAggregation::new("price");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 10.0),
            create_test_hit_numeric("2", "other", 20.0), // Missing price
            create_test_hit_numeric("3", "price", 30.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                // Should only count documents with price field
                assert_eq!(obj.get("count").and_then(|v| v.as_u64()), Some(2));
                assert_eq!(obj.get("sum").and_then(|v| v.as_f64()), Some(40.0));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }
}
