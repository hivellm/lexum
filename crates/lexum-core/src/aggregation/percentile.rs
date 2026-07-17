//! Percentile aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Percentile aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PercentileAggregation {
    /// Field to compute percentiles on
    pub field: String,
    /// Percentiles to compute (e.g., [50.0, 95.0, 99.0])
    #[serde(default = "default_percentiles")]
    pub percentiles: Vec<f64>,
}

fn default_percentiles() -> Vec<f64> {
    vec![50.0, 95.0, 99.0]
}

impl AggregationTrait for PercentileAggregation {
    fn name(&self) -> &str {
        "percentile"
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
            let mut result_obj = serde_json::Map::new();
            for percentile in &self.percentiles {
                result_obj.insert(percentile.to_string(), JsonValue::Null);
            }
            return Ok(AggregationResult::Metric(MetricAggregationResult::new(
                JsonValue::Object(result_obj),
            )));
        }

        // Sort values
        values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate percentiles
        let mut result_obj = serde_json::Map::new();
        for percentile in &self.percentiles {
            let index = (percentile / 100.0) * (values.len() - 1) as f64;
            let value = if index.fract() == 0.0 {
                values[index as usize]
            } else {
                let lower = values[index.floor() as usize];
                let upper = values[index.ceil() as usize];
                lower + (upper - lower) * index.fract()
            };
            result_obj.insert(
                percentile.to_string(),
                JsonValue::Number(
                    serde_json::Number::from_f64(value)
                        .unwrap_or_else(|| serde_json::Number::from(0)),
                ),
            );
        }

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            JsonValue::Object(result_obj),
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // For percentile merging, we need to merge the raw values
        // This is a simplified version - a full implementation would use T-Digest
        // Collect all values from all results
        // Note: This is simplified - in practice, we'd need to store raw values
        // or use a more sophisticated merging algorithm like T-Digest
        for result in results {
            if let AggregationResult::Metric(_metric_result) = result {
                // In a real implementation, we'd need access to raw values
                // For now, we'll use a weighted average approach
            }
        }

        // For now, return the first result (proper merging requires T-Digest)
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            let mut result_obj = serde_json::Map::new();
            for percentile in &self.percentiles {
                result_obj.insert(percentile.to_string(), JsonValue::Null);
            }
            Ok(AggregationResult::Metric(MetricAggregationResult::new(
                JsonValue::Object(result_obj),
            )))
        }
    }
}

impl PercentileAggregation {
    /// Create new percentile aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            percentiles: default_percentiles(),
        }
    }

    /// Set percentiles to compute
    pub fn with_percentiles(mut self, percentiles: Vec<f64>) -> Self {
        self.percentiles = percentiles;
        self
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

    #[test]
    fn test_percentile_aggregation_basic() {
        let agg = PercentileAggregation::new("latency");
        let field_cache = FieldCache::new();

        // Create 100 values from 1 to 100
        let hits: Vec<SearchHit> = (1..=100)
            .map(|i| create_test_hit_numeric(&format!("{i}"), "latency", f64::from(i)))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                // Check 50th percentile (median) should be around 50
                if let Some(p50) = obj.get("50").and_then(|v| v.as_f64()) {
                    assert!((p50 - 50.0).abs() < 1.0); // Allow small tolerance
                } else {
                    panic!("50th percentile not found");
                }
                // Check 95th percentile should be around 95
                if let Some(p95) = obj.get("95").and_then(|v| v.as_f64()) {
                    assert!((p95 - 95.0).abs() < 1.0);
                } else {
                    panic!("95th percentile not found");
                }
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_percentile_aggregation_empty() {
        let agg = PercentileAggregation::new("latency");
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                // All percentiles should be null
                assert_eq!(obj.get("50"), Some(&JsonValue::Null));
                assert_eq!(obj.get("95"), Some(&JsonValue::Null));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_percentile_aggregation_custom_percentiles() {
        let agg = PercentileAggregation::new("score").with_percentiles(vec![25.0, 75.0]);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "score", 10.0),
            create_test_hit_numeric("2", "score", 20.0),
            create_test_hit_numeric("3", "score", 30.0),
            create_test_hit_numeric("4", "score", 40.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            if let Some(obj) = metric_result.value.as_object() {
                // Should have 25th and 75th percentiles
                assert!(obj.contains_key("25"));
                assert!(obj.contains_key("75"));
                // Should not have default percentiles
                assert!(!obj.contains_key("50"));
                assert!(!obj.contains_key("99"));
            } else {
                panic!("Expected object result");
            }
        } else {
            panic!("Expected Metric result");
        }
    }
}
