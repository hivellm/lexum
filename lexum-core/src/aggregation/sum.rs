//! Sum aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Sum aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SumAggregation {
    /// Field to compute sum on
    pub field: String,
}

impl AggregationTrait for SumAggregation {
    fn name(&self) -> &str {
        "sum"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut sum = 0.0;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    sum += num;
                } else if let Some(num) = field_value.as_i64() {
                    sum += num as f64;
                }
            }
        }

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": sum }),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_sum = 0.0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(obj) = metric_result.value.as_object() {
                    if let Some(value) = obj.get("value").and_then(|v| v.as_f64()) {
                        total_sum += value;
                    }
                }
            }
        }

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::json!({ "value": total_sum }),
        )))
    }
}

impl SumAggregation {
    /// Create new sum aggregation
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
    fn test_sum_aggregation_basic() {
        let hits = vec![
            create_test_hit("1", "field", serde_json::json!(10)),
            create_test_hit("2", "field", serde_json::json!(20)),
            create_test_hit("3", "field", serde_json::json!(30)),
        ];
        let field_cache = FieldCache::new();

        let agg = SumAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 60.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_sum_aggregation_empty() {
        let hits: Vec<SearchHit> = vec![];
        let field_cache = FieldCache::new();

        let agg = SumAggregation::new("field");
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            assert_eq!(metric_result.value["value"], 0.0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_sum_aggregation_merge() {
        let field_cache = FieldCache::new();
        let agg = SumAggregation::new("field");

        let hits1 = vec![create_test_hit("1", "field", serde_json::json!(10))];
        let hits2 = vec![create_test_hit("2", "field", serde_json::json!(20))];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            assert_eq!(metric_result.value["value"], 30.0);
        } else {
            panic!("Expected Metric result");
        }
    }
}
