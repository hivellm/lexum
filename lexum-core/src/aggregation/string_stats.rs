//! String Stats Aggregation
//!
//! Computes statistics on string field values including character count, min/max/average length.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// String Stats Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StringStatsAggregation {
    /// Field containing string values
    pub field: String,
    /// Show distribution of character counts (default: false)
    #[serde(default)]
    pub show_distribution: bool,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

impl StringStatsAggregation {
    /// Create new string stats aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            show_distribution: false,
            aggs: HashMap::new(),
        }
    }

    /// Set show distribution flag
    pub fn show_distribution(mut self, show: bool) -> Self {
        self.show_distribution = show;
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

/// String stats result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StringStatsResult {
    /// Total count of string values
    pub count: u64,
    /// Minimum length
    pub min_length: Option<u64>,
    /// Maximum length
    pub max_length: Option<u64>,
    /// Average length
    pub avg_length: Option<f64>,
    /// Total character count (sum of all lengths)
    pub sum_of_lengths: u64,
    /// Character distribution (if show_distribution is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub distribution: Option<HashMap<char, u64>>,
}

impl AggregationTrait for StringStatsAggregation {
    fn name(&self) -> &str {
        "string_stats"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut count = 0u64;
        let mut sum_of_lengths = 0u64;
        let mut min_length: Option<u64> = None;
        let mut max_length: Option<u64> = None;
        let mut distribution: Option<HashMap<char, u64>> = if self.show_distribution {
            Some(HashMap::new())
        } else {
            None
        };

        // Extract string values from field
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(string_value) = extract_string_value(field_value) {
                    count += 1;
                    let length = string_value.chars().count() as u64;
                    sum_of_lengths += length;

                    match min_length {
                        None => {
                            min_length = Some(length);
                            max_length = Some(length);
                        }
                        Some(current_min) => {
                            if length < current_min {
                                min_length = Some(length);
                            }
                        }
                    }

                    match max_length {
                        None => max_length = Some(length),
                        Some(current_max) => {
                            if length > current_max {
                                max_length = Some(length);
                            }
                        }
                    }

                    // Update character distribution if enabled
                    if let Some(ref mut dist) = distribution {
                        for ch in string_value.chars() {
                            *dist.entry(ch).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        let avg_length = if count > 0 {
            Some(sum_of_lengths as f64 / count as f64)
        } else {
            None
        };

        let result = StringStatsResult {
            count,
            min_length,
            max_length,
            avg_length,
            sum_of_lengths,
            distribution,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge string stats from multiple shards
        let mut total_count = 0u64;
        let mut total_sum_of_lengths = 0u64;
        let mut global_min_length: Option<u64> = None;
        let mut global_max_length: Option<u64> = None;
        let mut merged_distribution: Option<HashMap<char, u64>> = if self.show_distribution {
            Some(HashMap::new())
        } else {
            None
        };

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(stats) =
                    serde_json::from_value::<StringStatsResult>(metric_result.value.clone())
                {
                    total_count += stats.count;
                    total_sum_of_lengths += stats.sum_of_lengths;

                    if let Some(min_len) = stats.min_length {
                        global_min_length =
                            Some(global_min_length.map(|m| m.min(min_len)).unwrap_or(min_len));
                    }

                    if let Some(max_len) = stats.max_length {
                        global_max_length =
                            Some(global_max_length.map(|m| m.max(max_len)).unwrap_or(max_len));
                    }

                    // Merge character distributions
                    if let (Some(ref mut merged_dist), Some(stats_dist)) =
                        (merged_distribution.as_mut(), stats.distribution.as_ref())
                    {
                        for (ch, count) in stats_dist.iter() {
                            *merged_dist.entry(*ch).or_insert(0) += count;
                        }
                    }
                }
            }
        }

        let merged_avg_length = if total_count > 0 {
            Some(total_sum_of_lengths as f64 / total_count as f64)
        } else {
            None
        };

        let merged_result = StringStatsResult {
            count: total_count,
            min_length: global_min_length,
            max_length: global_max_length,
            avg_length: merged_avg_length,
            sum_of_lengths: total_sum_of_lengths,
            distribution: merged_distribution,
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(merged_result)?,
        )))
    }
}

/// Extract string value from JSON value
fn extract_string_value(value: &JsonValue) -> Option<String> {
    match value {
        JsonValue::String(s) => Some(s.clone()),
        JsonValue::Number(n) => Some(n.to_string()),
        JsonValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_string_stats_aggregation() {
        let agg = StringStatsAggregation::new("name");

        assert_eq!(agg.field, "name");
        assert!(!agg.show_distribution);
    }

    #[test]
    fn test_string_stats_aggregation_empty() {
        let agg = StringStatsAggregation::new("name");
        let hits = vec![];
        let field_cache = FieldCache::new();

        let result = agg.execute(&hits, &field_cache).unwrap();
        if let AggregationResult::Metric(metric_result) = result {
            let stats: StringStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 0);
            assert_eq!(stats.sum_of_lengths, 0);
            assert!(stats.min_length.is_none());
            assert!(stats.max_length.is_none());
            assert!(stats.avg_length.is_none());
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_string_stats_aggregation_basic() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = StringStatsAggregation::new("name");
        let mut hits = vec![];

        // Create hits with string values
        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "name": "abc" }), // length: 3
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "name": "hello" }), // length: 5
        ));
        hits.push(SearchHit::new(
            DocumentId::new("3"),
            Score::new(1.0),
            serde_json::json!({ "name": "x" }), // length: 1
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let stats: StringStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 3);
            assert_eq!(stats.sum_of_lengths, 9); // 3 + 5 + 1
            assert_eq!(stats.min_length, Some(1));
            assert_eq!(stats.max_length, Some(5));
            assert_eq!(stats.avg_length, Some(3.0)); // 9 / 3
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_string_stats_aggregation_with_distribution() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = StringStatsAggregation::new("name").show_distribution(true);
        let mut hits = vec![];

        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "name": "abc" }),
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "name": "aab" }),
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let stats: StringStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 2);
            assert!(stats.distribution.is_some());
            let dist = stats.distribution.unwrap();
            // 'a' appears 3 times, 'b' appears 2 times, 'c' appears 1 time
            assert_eq!(dist.get(&'a'), Some(&3));
            assert_eq!(dist.get(&'b'), Some(&2));
            assert_eq!(dist.get(&'c'), Some(&1));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_string_stats_aggregation_numeric_values() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = StringStatsAggregation::new("value");
        let mut hits = vec![];

        // Numeric values should be converted to strings
        hits.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "value": 123 }), // "123" -> length 3
        ));
        hits.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "value": 45 }), // "45" -> length 2
        ));

        let field_cache = FieldCache::new();
        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Metric(metric_result) = result {
            let stats: StringStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 2);
            assert_eq!(stats.min_length, Some(2));
            assert_eq!(stats.max_length, Some(3));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_string_stats_aggregation_merge() {
        use crate::search::result::SearchHit;
        use crate::types::{DocumentId, Score};

        let agg = StringStatsAggregation::new("name");

        // Create first result: ["abc", "hello"]
        let mut hits1 = vec![];
        hits1.push(SearchHit::new(
            DocumentId::new("1"),
            Score::new(1.0),
            serde_json::json!({ "name": "abc" }),
        ));
        hits1.push(SearchHit::new(
            DocumentId::new("2"),
            Score::new(1.0),
            serde_json::json!({ "name": "hello" }),
        ));

        // Create second result: ["x"]
        let mut hits2 = vec![];
        hits2.push(SearchHit::new(
            DocumentId::new("3"),
            Score::new(1.0),
            serde_json::json!({ "name": "x" }),
        ));

        let field_cache = FieldCache::new();
        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Metric(metric_result) = merged {
            let stats: StringStatsResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(stats.count, 3);
            assert_eq!(stats.sum_of_lengths, 9);
            assert_eq!(stats.min_length, Some(1));
            assert_eq!(stats.max_length, Some(5));
            assert_eq!(stats.avg_length, Some(3.0));
        } else {
            panic!("Expected Metric result");
        }
    }

    #[test]
    fn test_string_stats_aggregation_serialization() {
        let agg = StringStatsAggregation::new("name").show_distribution(true);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("name"));
        assert!(json.contains("show_distribution"));

        let deserialized: StringStatsAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "name");
        assert!(deserialized.show_distribution);
    }
}
