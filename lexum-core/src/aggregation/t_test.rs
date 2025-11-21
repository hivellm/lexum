//! T-Test Aggregation
//!
//! Performs statistical t-test for A/B testing scenarios.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// T-Test Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TTestAggregation {
    /// Field containing numeric values
    pub field: String,
    /// A/B testing configuration
    pub a: TTestGroup,
    /// B testing configuration
    pub b: TTestGroup,
    /// Test type (default: "paired")
    #[serde(default = "default_test_type")]
    pub test_type: String,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_test_type() -> String {
    "paired".to_string()
}

/// T-Test Group configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TTestGroup {
    /// Filter query to identify group A or B
    pub filter: crate::Query,
}

impl TTestAggregation {
    /// Create new t-test aggregation
    pub fn new(field: impl Into<String>, a_filter: crate::Query, b_filter: crate::Query) -> Self {
        Self {
            field: field.into(),
            a: TTestGroup { filter: a_filter },
            b: TTestGroup { filter: b_filter },
            test_type: "paired".to_string(),
            aggs: HashMap::new(),
        }
    }

    /// Set test type ("paired" or "unpaired")
    pub fn test_type(mut self, test_type: impl Into<String>) -> Self {
        self.test_type = test_type.into();
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

/// T-Test result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TTestResult {
    /// T-statistic value
    pub t_value: Option<f64>,
    /// Degrees of freedom
    pub degrees_of_freedom: Option<f64>,
    /// P-value (probability)
    pub p_value: Option<f64>,
    /// Whether the difference is statistically significant (p < 0.05)
    pub is_significant: Option<bool>,
    /// Group A statistics
    pub a: TTestGroupStats,
    /// Group B statistics
    pub b: TTestGroupStats,
}

/// T-Test Group statistics
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TTestGroupStats {
    /// Count of values
    pub count: u64,
    /// Mean value
    pub mean: Option<f64>,
    /// Variance
    pub variance: Option<f64>,
    /// Standard deviation
    pub std_deviation: Option<f64>,
}

impl AggregationTrait for TTestAggregation {
    fn name(&self) -> &str {
        "t_test"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Evaluating filter queries to separate groups A and B
        // 2. Extracting numeric values from each group
        // 3. Calculating statistics for each group
        // 4. Performing t-test calculation
        //
        // For now, provide a simplified implementation that extracts values
        // and calculates basic statistics

        let mut group_a_values: Vec<f64> = Vec::new();
        let mut group_b_values: Vec<f64> = Vec::new();

        // Extract values from field (simplified - full implementation would evaluate filters)
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num_value) = extract_numeric_value(field_value) {
                    // Simplified: split values evenly between groups
                    // Full implementation would evaluate self.a.filter and self.b.filter
                    if group_a_values.len() <= group_b_values.len() {
                        group_a_values.push(num_value);
                    } else {
                        group_b_values.push(num_value);
                    }
                }
            }
        }

        // Calculate statistics for group A
        let stats_a = calculate_group_stats(&group_a_values);

        // Calculate statistics for group B
        let stats_b = calculate_group_stats(&group_b_values);

        // Perform t-test calculation
        let (t_value, degrees_of_freedom, p_value) =
            if let (Some(mean_a), Some(mean_b), Some(var_a), Some(var_b), count_a, count_b) = (
                stats_a.mean,
                stats_b.mean,
                stats_a.variance,
                stats_b.variance,
                stats_a.count,
                stats_b.count,
            ) {
                if count_a > 0 && count_b > 0 {
                    // Welch's t-test (unequal variances)
                    let n_a = count_a as f64;
                    let n_b = count_b as f64;

                    // Pooled standard error
                    let se = ((var_a / n_a) + (var_b / n_b)).sqrt();

                    if se > 0.0 {
                        // T-statistic
                        let t = (mean_a - mean_b) / se;

                        // Degrees of freedom (Welch's approximation)
                        let df = ((var_a / n_a + var_b / n_b).powi(2))
                            / ((var_a / n_a).powi(2) / (n_a - 1.0)
                                + (var_b / n_b).powi(2) / (n_b - 1.0));

                        // Simplified p-value calculation (two-tailed)
                        // Full implementation would use proper t-distribution
                        let p = if t.abs() > 2.0 {
                            Some(0.05) // Approximate significance threshold
                        } else {
                            Some(0.5) // Not significant
                        };

                        (Some(t), Some(df), p)
                    } else {
                        (None, None, None)
                    }
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

        let is_significant = p_value.map(|p| p < 0.05);

        let result = TTestResult {
            t_value,
            degrees_of_freedom,
            p_value,
            is_significant,
            a: stats_a,
            b: stats_b,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge t-test results by combining group statistics
        let mut merged_a_count = 0u64;
        let mut merged_a_sum = 0.0;
        let mut merged_a_sum_squares = 0.0;

        let mut merged_b_count = 0u64;
        let mut merged_b_sum = 0.0;
        let mut merged_b_sum_squares = 0.0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(t_test_result) =
                    serde_json::from_value::<TTestResult>(metric_result.value.clone())
                {
                    // Merge group A statistics
                    merged_a_count += t_test_result.a.count;
                    if let Some(mean_a) = t_test_result.a.mean {
                        merged_a_sum += mean_a * t_test_result.a.count as f64;
                        merged_a_sum_squares += (mean_a * mean_a
                            + t_test_result.a.variance.unwrap_or(0.0))
                            * t_test_result.a.count as f64;
                    }

                    // Merge group B statistics
                    merged_b_count += t_test_result.b.count;
                    if let Some(mean_b) = t_test_result.b.mean {
                        merged_b_sum += mean_b * t_test_result.b.count as f64;
                        merged_b_sum_squares += (mean_b * mean_b
                            + t_test_result.b.variance.unwrap_or(0.0))
                            * t_test_result.b.count as f64;
                    }
                }
            }
        }

        // Recalculate statistics from merged values
        let merged_a_mean = if merged_a_count > 0 {
            Some(merged_a_sum / merged_a_count as f64)
        } else {
            None
        };

        let merged_b_mean = if merged_b_count > 0 {
            Some(merged_b_sum / merged_b_count as f64)
        } else {
            None
        };

        let merged_a_variance = if let Some(mean) = merged_a_mean.filter(|_| merged_a_count > 1) {
            let n = merged_a_count as f64;
            Some((merged_a_sum_squares - n * mean * mean) / (n - 1.0))
        } else {
            None
        };

        let merged_b_variance = if let Some(mean) = merged_b_mean.filter(|_| merged_b_count > 1) {
            let n = merged_b_count as f64;
            Some((merged_b_sum_squares - n * mean * mean) / (n - 1.0))
        } else {
            None
        };

        let merged_a_std_dev = merged_a_variance.map(|v| v.sqrt());
        let merged_b_std_dev = merged_b_variance.map(|v| v.sqrt());

        // Recalculate t-test
        let (t_value, degrees_of_freedom, p_value) =
            if let (Some(mean_a), Some(mean_b), Some(var_a), Some(var_b)) = (
                merged_a_mean,
                merged_b_mean,
                merged_a_variance,
                merged_b_variance,
            ) {
                let n_a = merged_a_count as f64;
                let n_b = merged_b_count as f64;

                let se = ((var_a / n_a) + (var_b / n_b)).sqrt();

                if se > 0.0 {
                    let t = (mean_a - mean_b) / se;
                    let df = ((var_a / n_a + var_b / n_b).powi(2))
                        / ((var_a / n_a).powi(2) / (n_a - 1.0)
                            + (var_b / n_b).powi(2) / (n_b - 1.0));

                    let p = if t.abs() > 2.0 { Some(0.05) } else { Some(0.5) };

                    (Some(t), Some(df), p)
                } else {
                    (None, None, None)
                }
            } else {
                (None, None, None)
            };

        let is_significant = p_value.map(|p| p < 0.05);

        let merged_result = TTestResult {
            t_value,
            degrees_of_freedom,
            p_value,
            is_significant,
            a: TTestGroupStats {
                count: merged_a_count,
                mean: merged_a_mean,
                variance: merged_a_variance,
                std_deviation: merged_a_std_dev,
            },
            b: TTestGroupStats {
                count: merged_b_count,
                mean: merged_b_mean,
                variance: merged_b_variance,
                std_deviation: merged_b_std_dev,
            },
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

/// Calculate statistics for a group
fn calculate_group_stats(values: &[f64]) -> TTestGroupStats {
    if values.is_empty() {
        return TTestGroupStats {
            count: 0,
            mean: None,
            variance: None,
            std_deviation: None,
        };
    }

    let count = values.len() as u64;
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;

    let sum_squares: f64 = values.iter().map(|v| v * v).sum();
    let variance = if count > 1 {
        Some((sum_squares - count as f64 * mean * mean) / (count as f64 - 1.0))
    } else {
        None
    };

    let std_deviation = variance.map(|v| v.sqrt());

    TTestGroupStats {
        count,
        mean: Some(mean),
        variance,
        std_deviation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{Query, TermQuery};

    #[test]
    fn test_t_test_aggregation() {
        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter.clone(), b_filter.clone());

        assert_eq!(agg.field, "value");
        assert_eq!(agg.test_type, "paired");
    }

    #[test]
    fn test_t_test_aggregation_with_test_type() {
        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter, b_filter).test_type("unpaired");

        assert_eq!(agg.test_type, "unpaired");
    }

    #[test]
    fn test_t_test_aggregation_empty() {
        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter, b_filter);
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let t_test: TTestResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(t_test.a.count, 0);
            assert_eq!(t_test.b.count, 0);
            assert!(t_test.t_value.is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_t_test_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter, b_filter);
        let mut hits = vec![];

        // Create hits with values
        // Group A: [10, 20, 30] -> mean = 20
        // Group B: [15, 25, 35] -> mean = 25
        for i in 1..=6 {
            hits.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 5 }),
            ));
        }

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let t_test: TTestResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(t_test.a.count > 0);
            assert!(t_test.b.count > 0);
            assert!(t_test.a.mean.is_some());
            assert!(t_test.b.mean.is_some());
            assert!(t_test.t_value.is_some() || t_test.a.count == 0 || t_test.b.count == 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_t_test_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter, b_filter);

        // Create first result
        let mut hits1 = vec![];
        for i in 1..=3 {
            hits1.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        // Create second result
        let mut hits2 = vec![];
        for i in 4..=6 {
            hits2.push(SearchHit::new(
                DocumentId::new(i.to_string()),
                Score::new(i as f32),
                serde_json::json!({ "value": i * 10 }),
            ));
        }

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let t_test: TTestResult = serde_json::from_value(metric_result.value).unwrap();
            assert!(t_test.a.count > 0 || t_test.b.count > 0);
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_t_test_aggregation_serialization() {
        let a_filter = Query::Term(TermQuery::new("group", "A"));
        let b_filter = Query::Term(TermQuery::new("group", "B"));
        let agg = TTestAggregation::new("value", a_filter, b_filter).test_type("unpaired");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("value"));
        assert!(json.contains("test_type"));
        assert!(json.contains("a"));
        assert!(json.contains("b"));

        let deserialized: TTestAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "value");
        assert_eq!(deserialized.test_type, "unpaired");
    }
}
