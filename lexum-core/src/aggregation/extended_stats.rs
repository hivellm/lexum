//! Extended Stats Aggregation
//!
//! Computes extended statistics including variance, standard deviation, and sum of squares.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Extended Stats Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtendedStatsAggregation {
    /// Field to compute extended stats on
    pub field: String,
    /// Number of standard deviations for bounds (default: 2)
    #[serde(default = "default_sigma")]
    pub sigma: f64,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_sigma() -> f64 {
    2.0
}

impl ExtendedStatsAggregation {
    /// Create new extended stats aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            sigma: 2.0,
            aggs: HashMap::new(),
        }
    }

    /// Set sigma (number of standard deviations for bounds)
    pub fn sigma(mut self, sigma: f64) -> Self {
        self.sigma = sigma;
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

/// Extended stats result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ExtendedStatsResult {
    /// Count of values
    pub count: u64,
    /// Minimum value
    pub min: Option<f64>,
    /// Maximum value
    pub max: Option<f64>,
    /// Average (mean) value
    pub avg: Option<f64>,
    /// Sum of all values
    pub sum: f64,
    /// Sum of squares of all values
    pub sum_of_squares: f64,
    /// Variance
    pub variance: Option<f64>,
    /// Standard deviation
    pub std_deviation: Option<f64>,
    /// Upper bound (avg + sigma * std_deviation)
    pub std_deviation_bounds_upper: Option<f64>,
    /// Lower bound (avg - sigma * std_deviation)
    pub std_deviation_bounds_lower: Option<f64>,
}

impl AggregationTrait for ExtendedStatsAggregation {
    fn name(&self) -> &str {
        "extended_stats"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut count = 0u64;
        let mut sum = 0.0;
        let mut sum_of_squares = 0.0;
        let mut min_value: Option<f64> = None;
        let mut max_value: Option<f64> = None;

        // Extract numeric values from field
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num_value) = extract_numeric_value(field_value) {
                    count += 1;
                    sum += num_value;
                    sum_of_squares += num_value * num_value;

                    match min_value {
                        None => {
                            min_value = Some(num_value);
                            max_value = Some(num_value);
                        }
                        Some(current_min) => {
                            if num_value < current_min {
                                min_value = Some(num_value);
                            }
                        }
                    }

                    match max_value {
                        None => max_value = Some(num_value),
                        Some(current_max) => {
                            if num_value > current_max {
                                max_value = Some(num_value);
                            }
                        }
                    }
                }
            }
        }

        // Calculate statistics
        let avg = if count > 0 {
            Some(sum / count as f64)
        } else {
            None
        };
        let variance = if count > 1 {
            // Sample variance: sum((x - mean)^2) / (n - 1)
            // Using sum_of_squares: variance = (sum_of_squares - n * mean^2) / (n - 1)
            if let Some(mean) = avg {
                let n = count as f64;
                Some((sum_of_squares - n * mean * mean) / (n - 1.0))
            } else {
                None
            }
        } else {
            None
        };

        let std_deviation = variance.map(|v| v.sqrt());
        let std_deviation_bounds_upper = if let (Some(mean), Some(std_dev)) = (avg, std_deviation) {
            Some(mean + self.sigma * std_dev)
        } else {
            None
        };
        let std_deviation_bounds_lower = if let (Some(mean), Some(std_dev)) = (avg, std_deviation) {
            Some(mean - self.sigma * std_dev)
        } else {
            None
        };

        let result = ExtendedStatsResult {
            count,
            min: min_value,
            max: max_value,
            avg,
            sum,
            sum_of_squares,
            variance,
            std_deviation,
            std_deviation_bounds_upper,
            std_deviation_bounds_lower,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge extended stats from multiple shards
        let mut total_count = 0u64;
        let mut total_sum = 0.0;
        let mut total_sum_of_squares = 0.0;
        let mut global_min: Option<f64> = None;
        let mut global_max: Option<f64> = None;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(stats) =
                    serde_json::from_value::<ExtendedStatsResult>(metric_result.value.clone())
                {
                    total_count += stats.count;
                    total_sum += stats.sum;
                    total_sum_of_squares += stats.sum_of_squares;

                    if let Some(min_val) = stats.min {
                        global_min = Some(global_min.map(|m| m.min(min_val)).unwrap_or(min_val));
                    }

                    if let Some(max_val) = stats.max {
                        global_max = Some(global_max.map(|m| m.max(max_val)).unwrap_or(max_val));
                    }
                }
            }
        }

        // Recalculate statistics from merged values
        let avg = if total_count > 0 {
            Some(total_sum / total_count as f64)
        } else {
            None
        };

        let variance = if total_count > 1 {
            if let Some(mean) = avg {
                let n = total_count as f64;
                Some((total_sum_of_squares - n * mean * mean) / (n - 1.0))
            } else {
                None
            }
        } else {
            None
        };

        let std_deviation = variance.map(|v| v.sqrt());
        let std_deviation_bounds_upper = if let (Some(mean), Some(std_dev)) = (avg, std_deviation) {
            Some(mean + self.sigma * std_dev)
        } else {
            None
        };
        let std_deviation_bounds_lower = if let (Some(mean), Some(std_dev)) = (avg, std_deviation) {
            Some(mean - self.sigma * std_dev)
        } else {
            None
        };

        let merged_result = ExtendedStatsResult {
            count: total_count,
            min: global_min,
            max: global_max,
            avg,
            sum: total_sum,
            sum_of_squares: total_sum_of_squares,
            variance,
            std_deviation,
            std_deviation_bounds_upper,
            std_deviation_bounds_lower,
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
    fn test_extended_stats_aggregation() {
        let agg = ExtendedStatsAggregation::new("price");

        assert_eq!(agg.field, "price");
        assert_eq!(agg.sigma, 2.0);
    }

    #[test]
    fn test_extended_stats_aggregation_with_sigma() {
        let agg = ExtendedStatsAggregation::new("price").sigma(3.0);

        assert_eq!(agg.sigma, 3.0);
    }

    #[test]
    fn test_extended_stats_aggregation_empty() {
        let agg = ExtendedStatsAggregation::new("price");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let stats: ExtendedStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 0);
            assert_eq!(stats.sum, 0.0);
            assert_eq!(stats.sum_of_squares, 0.0);
            assert!(stats.min.is_none());
            assert!(stats.max.is_none());
            assert!(stats.avg.is_none());
            assert!(stats.variance.is_none());
            assert!(stats.std_deviation.is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_extended_stats_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = ExtendedStatsAggregation::new("value");
        let mut hits = vec![];

        // Create hits with numeric values
        for i in 1..=5 {
            hits.push(SearchHit {
                id: DocumentId(i),
                score: Score(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let stats: ExtendedStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 5);
            assert_eq!(stats.sum, 150.0); // 10 + 20 + 30 + 40 + 50
            assert_eq!(stats.min, Some(10.0));
            assert_eq!(stats.max, Some(50.0));
            assert_eq!(stats.avg, Some(30.0));
            assert!(stats.variance.is_some());
            assert!(stats.std_deviation.is_some());
            assert!(stats.std_deviation_bounds_upper.is_some());
            assert!(stats.std_deviation_bounds_lower.is_some());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_extended_stats_aggregation_variance() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = ExtendedStatsAggregation::new("value");
        let mut hits = vec![];

        // Create hits with values: [10, 20, 30, 40, 50]
        // Mean = 30
        // Variance = sum((x - mean)^2) / (n - 1) = (400 + 100 + 0 + 100 + 400) / 4 = 250
        // Std deviation = sqrt(250) ≈ 15.81
        for i in 1..=5 {
            hits.push(SearchHit {
                id: DocumentId(i),
                score: Score(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let stats: ExtendedStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 5);
            assert_eq!(stats.avg, Some(30.0));
            // Variance should be approximately 250
            assert!((stats.variance.unwrap() - 250.0).abs() < 0.01);
            // Std deviation should be approximately 15.81
            assert!((stats.std_deviation.unwrap() - 15.811388300841896).abs() < 0.01);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_extended_stats_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = ExtendedStatsAggregation::new("value");

        // Create first result: [10, 20, 30]
        let mut hits1 = vec![];
        for i in 1..=3 {
            hits1.push(SearchHit {
                id: DocumentId(i),
                score: Score(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        // Create second result: [40, 50]
        let mut hits2 = vec![];
        for i in 4..=5 {
            hits2.push(SearchHit {
                id: DocumentId(i),
                score: Score(i as f32),
                source: serde_json::json!({ "value": i * 10 }),
            });
        }

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let stats: ExtendedStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 5);
            assert_eq!(stats.sum, 150.0);
            assert_eq!(stats.min, Some(10.0));
            assert_eq!(stats.max, Some(50.0));
            assert_eq!(stats.avg, Some(30.0));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_extended_stats_aggregation_serialization() {
        let agg = ExtendedStatsAggregation::new("price").sigma(2.5);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("price"));
        assert!(json.contains("sigma"));

        let deserialized: ExtendedStatsAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "price");
        assert_eq!(deserialized.sigma, 2.5);
    }
}
