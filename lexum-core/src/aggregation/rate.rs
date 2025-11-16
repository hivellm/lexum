//! Rate Aggregation
//!
//! Computes the rate of change over time for numeric values.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Rate Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateAggregation {
    /// Field containing numeric values
    pub field: String,
    /// Unit for the rate (default: "1s" - per second)
    /// Supported units: s (second), m (minute), h (hour), d (day)
    #[serde(default = "default_unit")]
    pub unit: String,
    /// Mode for rate calculation (default: "sum")
    /// Options: "sum", "value_count"
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_unit() -> String {
    "1s".to_string()
}

fn default_mode() -> String {
    "sum".to_string()
}

impl RateAggregation {
    /// Create new rate aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            unit: "1s".to_string(),
            mode: "sum".to_string(),
            aggs: HashMap::new(),
        }
    }

    /// Set rate unit (e.g., "1s", "1m", "1h", "1d")
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Set rate calculation mode ("sum" or "value_count")
    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
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

/// Rate result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RateResult {
    /// Rate value
    pub value: Option<f64>,
    /// Unit used for the rate
    pub unit: String,
}

impl AggregationTrait for RateAggregation {
    fn name(&self) -> &str {
        "rate"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Time field to determine time range
        // 2. Time-based grouping to calculate rate over time periods
        // 3. Unit conversion (s, m, h, d)
        //
        // For now, calculate a simplified rate based on value sum/count
        // A full implementation would need time-based aggregation context

        let mut sum = 0.0;
        let mut count = 0u64;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num_value) = extract_numeric_value(field_value) {
                    sum += num_value;
                    count += 1;
                }
            }
        }

        // Calculate rate based on mode
        let rate_value = match self.mode.as_str() {
            "sum" => {
                if count > 0 {
                    // Simplified: assume unit time period
                    // Full implementation would use actual time range
                    Some(sum)
                } else {
                    None
                }
            }
            "value_count" => {
                if count > 0 {
                    // Simplified: count per unit time period
                    Some(count as f64)
                } else {
                    None
                }
            }
            _ => None,
        };

        // Apply unit conversion (simplified)
        let converted_rate = rate_value.map(|r| convert_rate(r, &self.unit));

        let result = RateResult {
            value: converted_rate,
            unit: self.unit.clone(),
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge rate results by summing values
        let mut total_sum = 0.0;
        let mut has_value = false;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(rate_result) =
                    serde_json::from_value::<RateResult>(metric_result.value.clone())
                {
                    if let Some(value) = rate_result.value {
                        total_sum += value;
                        has_value = true;
                    }
                }
            }
        }

        let merged_rate = if has_value { Some(total_sum) } else { None };

        let merged_result = RateResult {
            value: merged_rate,
            unit: self.unit.clone(),
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

/// Convert rate based on unit
/// Note: This is a simplified conversion. Full implementation would
/// use actual time range from the aggregation context.
fn convert_rate(rate: f64, unit: &str) -> f64 {
    // Parse unit (e.g., "1s", "1m", "1h", "1d")
    // For now, return rate as-is (full implementation would convert)
    // Full implementation would:
    // - Parse unit to get time period (1s = 1 second, 1m = 1 minute, etc.)
    // - Divide rate by actual time range in seconds
    // - Multiply by unit time period
    rate
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_aggregation() {
        let agg = RateAggregation::new("value");

        assert_eq!(agg.field, "value");
        assert_eq!(agg.unit, "1s");
        assert_eq!(agg.mode, "sum");
    }

    #[test]
    fn test_rate_aggregation_with_unit() {
        let agg = RateAggregation::new("value").unit("1m");

        assert_eq!(agg.unit, "1m");
    }

    #[test]
    fn test_rate_aggregation_with_mode() {
        let agg = RateAggregation::new("value").mode("value_count");

        assert_eq!(agg.mode, "value_count");
    }

    #[test]
    fn test_rate_aggregation_empty() {
        let agg = RateAggregation::new("value");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let rate: RateResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(rate.value.is_none());
            assert_eq!(rate.unit, "1s");
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_rate_aggregation_sum_mode() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = RateAggregation::new("value").mode("sum");
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 10 }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 20 }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("3"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 30 }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let rate: RateResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(rate.value.is_some());
            assert_eq!(rate.value.unwrap(), 60.0); // 10 + 20 + 30
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_rate_aggregation_value_count_mode() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = RateAggregation::new("value").mode("value_count");
        let mut hits = vec![];

        hits.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 10 }),
        });
        hits.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 20 }),
        });

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let rate: RateResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(rate.value.is_some());
            assert_eq!(rate.value.unwrap(), 2.0); // Count of values
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_rate_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = RateAggregation::new("value").mode("sum");

        // Create first result: sum = 10
        let mut hits1 = vec![];
        hits1.push(SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 10 }),
        });

        // Create second result: sum = 20
        let mut hits2 = vec![];
        hits2.push(SearchHit {
            id: DocumentId::new("2"),
            score: Score::new(1.0),
            source: serde_json::json!({ "value": 20 }),
        });

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let rate: RateResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(rate.value.is_some());
            assert_eq!(rate.value.unwrap(), 30.0); // 10 + 20
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_rate_aggregation_serialization() {
        let agg = RateAggregation::new("value").unit("1m").mode("value_count");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("value"));
        assert!(json.contains("unit"));
        assert!(json.contains("mode"));

        let deserialized: RateAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "value");
        assert_eq!(deserialized.unit, "1m");
        assert_eq!(deserialized.mode, "value_count");
    }
}
