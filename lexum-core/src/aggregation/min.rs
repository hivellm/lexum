//! Min aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Min aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinAggregation {
    /// Field to compute minimum on
    pub field: String,
}

impl AggregationTrait for MinAggregation {
    fn name(&self) -> &str {
        "min"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut min_value = f64::INFINITY;
        let mut has_values = false;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    min_value = min_value.min(num);
                    has_values = true;
                } else if let Some(num) = field_value.as_i64() {
                    min_value = min_value.min(num as f64);
                    has_values = true;
                }
            }
        }

        let result_value = if has_values {
            serde_json::json!(min_value)
        } else {
            serde_json::Value::Null
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": result_value }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut min_value = f64::INFINITY;
        let mut has_values = false;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(value) = obj.get("value") {
                        if let Some(num) = value.as_f64() {
                            min_value = min_value.min(num);
                            has_values = true;
                        }
                    }
                }
            }
        }

        let result_value = if has_values {
            serde_json::json!(min_value)
        } else {
            serde_json::Value::Null
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": result_value }),
        )))
    }
}

impl MinAggregation {
    /// Create new min aggregation
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
    fn test_min_aggregation_basic() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(30)),
            create_test_hit("2", "field", serde_json::json!(10)),
            create_test_hit("3", "field", serde_json::json!(20)),
        ];
        let field_cache = FieldCache::new();

        let agg = MinAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 10.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_min_aggregation_empty() {
        let hits: Vec<SearchHit> = vec![];
        let field_cache = FieldCache::new();

        let agg = MinAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert!(metric_result.value["value"].is_null());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_min_aggregation_merge() {
        let field_cache = FieldCache::new();
        let agg = MinAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(20))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(10))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 10.0);
        } else {
            panic!("Expected Metric result");
        }
    }
}
