//! Cardinality aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use utoipa::ToSchema;

/// Cardinality aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CardinalityAggregation {
    /// Field to count unique values for
    pub field: String,
    /// Precision threshold (for HyperLogLog, not yet implemented)
    #[serde(default = "default_precision")]
    pub precision_threshold: usize,
}

fn default_precision() -> usize {
    3000
}

impl AggregationTrait for CardinalityAggregation {
    fn name(&self) -> &str {
        "cardinality"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // For now, use HashSet for exact counting
        // Future: Implement HyperLogLog for large datasets
        let mut unique_values = HashSet::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                let key = match field_value {
                    JsonValue::String(s) => s.clone(),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    _ => field_value.to_string(),
                };
                unique_values.insert(key);
            }
        }

        let cardinality = unique_values.len();

        Ok(AggregationResult::Metric(
            MetricAggregationResult::from_usize(cardinality),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // For cardinality, we need to merge unique sets
        // This is simplified - proper implementation would use HyperLogLog
        let mut total_cardinality = 0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Some(card) = metric_result.value.as_u64() {
                    total_cardinality += card as usize;
                }
            }
        }

        // Note: This is an approximation - proper merging would use HyperLogLog union
        Ok(AggregationResult::Metric(
            MetricAggregationResult::from_usize(total_cardinality),
        ))
    }
}

impl CardinalityAggregation {
    /// Create new cardinality aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            precision_threshold: default_precision(),
        }
    }

    /// Set precision threshold
    pub fn with_precision_threshold(mut self, threshold: usize) -> Self {
        self.precision_threshold = threshold;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: &str) -> SearchHit {
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: value }),
        )
    }

    #[test]
    fn test_cardinality_aggregation_basic() {
        let agg = CardinalityAggregation::new("user_id");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "user_id", "user1"),
            create_test_hit("2", "user_id", "user2"),
            create_test_hit("3", "user_id", "user1"), // Duplicate
            create_test_hit("4", "user_id", "user3"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            // Should have 3 unique values (user1, user2, user3)
            if let Some(card) = metric_result.value.as_u64() {
                assert_eq!(card, 3);
            } else {
                panic!("Expected numeric result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_cardinality_aggregation_all_unique() {
        let agg = CardinalityAggregation::new("id");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "id", "a"),
            create_test_hit("2", "id", "b"),
            create_test_hit("3", "id", "c"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(card) = metric_result.value.as_u64() {
                assert_eq!(card, 3);
            } else {
                panic!("Expected numeric result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_cardinality_aggregation_all_duplicates() {
        let agg = CardinalityAggregation::new("status");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "active"),
            create_test_hit("3", "status", "active"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(card) = metric_result.value.as_u64() {
                assert_eq!(card, 1); // Only one unique value
            } else {
                panic!("Expected numeric result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }
}
