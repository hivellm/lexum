//! Range aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Range definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum Range {
    /// Range with from and to
    FromTo {
        /// Lower bound (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<f64>,
        /// Upper bound (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<f64>,
        /// Custom key for this range
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// Simple range with just a key
    KeyOnly {
        /// Custom key
        key: String,
        /// Lower bound (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<f64>,
        /// Upper bound (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<f64>,
    },
}

impl Range {
    /// Get the from value
    fn from(&self) -> Option<f64> {
        match self {
            Range::FromTo { from, .. } | Range::KeyOnly { from, .. } => *from,
        }
    }

    /// Get the to value
    fn to(&self) -> Option<f64> {
        match self {
            Range::FromTo { to, .. } | Range::KeyOnly { to, .. } => *to,
        }
    }

    /// Get the key, or generate one
    fn key(&self) -> String {
        match self {
            Range::FromTo { key, from, to } => {
                if let Some(k) = key {
                    k.clone()
                } else {
                    format!(
                        "{}-{}",
                        from.map(|f| f.to_string())
                            .unwrap_or_else(|| "*".to_string()),
                        to.map(|t| t.to_string()).unwrap_or_else(|| "*".to_string())
                    )
                }
            }
            Range::KeyOnly { key, .. } => key.clone(),
        }
    }

    /// Check if a value matches this range
    fn matches(&self, value: f64) -> bool {
        let from = self.from().unwrap_or(f64::NEG_INFINITY);
        let to = self.to().unwrap_or(f64::INFINITY);
        value >= from && value < to
    }
}

/// Range aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RangeAggregation {
    /// Field to aggregate on
    pub field: String,
    /// Ranges to create buckets for
    pub ranges: Vec<Range>,
    /// Return keyed response (key: bucket) instead of array
    #[serde(default)]
    pub keyed: bool,
}

impl AggregationTrait for RangeAggregation {
    fn name(&self) -> &str {
        "range"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut range_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges with 0 count
        for range in &self.ranges {
            range_counts.insert(range.key(), 0);
        }

        // Count documents in each range
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    for range in &self.ranges {
                        if range.matches(num) {
                            *range_counts.get_mut(&range.key()).unwrap() += 1;
                            break; // Document can only match one range
                        }
                    }
                } else if let Some(num) = field_value.as_i64() {
                    let num_f64 = num as f64;
                    for range in &self.ranges {
                        if range.matches(num_f64) {
                            *range_counts.get_mut(&range.key()).unwrap() += 1;
                            break;
                        }
                    }
                }
            }
        }

        // Convert to buckets in order of ranges
        let buckets: Vec<Bucket> = self
            .ranges
            .iter()
            .map(|range| {
                let count = range_counts.get(&range.key()).copied().unwrap_or(0);
                Bucket::new(JsonValue::String(range.key()), count)
            })
            .collect();

        if self.keyed {
            // Return keyed format
            let mut keyed_buckets = HashMap::new();
            for bucket in buckets {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_buckets.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_buckets),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                buckets,
            )))
        }
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges
        for range in &self.ranges {
            merged_counts.insert(range.key(), 0);
        }

        // Merge counts from all results
        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let JsonValue::String(key) = &bucket.key {
                        *merged_counts.entry(key.clone()).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets in order of ranges
        let buckets: Vec<Bucket> = self
            .ranges
            .iter()
            .map(|range| {
                let count = merged_counts.get(&range.key()).copied().unwrap_or(0);
                Bucket::new(JsonValue::String(range.key()), count)
            })
            .collect();

        if self.keyed {
            let mut keyed_buckets = HashMap::new();
            for bucket in buckets {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_buckets.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_buckets),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                buckets,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_range_aggregation_basic() {
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: None,
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(20.0),
                key: None,
            },
            Range::FromTo {
                from: Some(20.0),
                to: None,
                key: None,
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
            create_test_hit_numeric("3", "price", 25.0),
            create_test_hit_numeric("4", "price", 8.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // Check counts
            let bucket_0_10 = buckets.iter().find(|b| b.key.as_str() == Some("0-10"));
            let bucket_10_20 = buckets.iter().find(|b| b.key.as_str() == Some("10-20"));
            let bucket_20_star = buckets.iter().find(|b| b.key.as_str() == Some("20-*"));

            assert!(bucket_0_10.is_some());
            assert_eq!(bucket_0_10.unwrap().doc_count, 2); // 5.0 and 8.0
            assert!(bucket_10_20.is_some());
            assert_eq!(bucket_10_20.unwrap().doc_count, 1); // 15.0
            assert!(bucket_20_star.is_some());
            assert_eq!(bucket_20_star.unwrap().doc_count, 1); // 25.0
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_keyed() {
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: Some("low".to_string()),
            },
            Range::FromTo {
                from: Some(10.0),
                to: None,
                key: Some("high".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    assert_eq!(buckets.len(), 2);
                    assert!(buckets.contains_key("low"));
                    assert!(buckets.contains_key("high"));
                    assert_eq!(buckets["low"].doc_count, 1);
                    assert_eq!(buckets["high"].doc_count, 1);
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_merge() {
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: None,
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(20.0),
                key: None,
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit_numeric("1", "price", 5.0)];
        let hits2 = vec![create_test_hit_numeric("2", "price", 15.0)];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2);
            let bucket_0_10 = buckets.iter().find(|b| b.key.as_str() == Some("0-10"));
            let bucket_10_20 = buckets.iter().find(|b| b.key.as_str() == Some("10-20"));
            assert!(bucket_0_10.is_some());
            assert_eq!(bucket_0_10.unwrap().doc_count, 1);
            assert!(bucket_10_20.is_some());
            assert_eq!(bucket_10_20.unwrap().doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_boundary_values() {
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: None,
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(20.0),
                key: None,
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        // Test boundary values: 0.0 (inclusive), 10.0 (exclusive), 20.0 (exclusive)
        let hits = vec![
            create_test_hit_numeric("1", "price", 0.0), // Should be in first range
            create_test_hit_numeric("2", "price", 9.99), // Should be in first range
            create_test_hit_numeric("3", "price", 10.0), // Should be in second range (to is exclusive)
            create_test_hit_numeric("4", "price", 19.99), // Should be in second range
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2);
            let bucket_0_10 = buckets.iter().find(|b| b.key.as_str() == Some("0-10"));
            let bucket_10_20 = buckets.iter().find(|b| b.key.as_str() == Some("10-20"));
            assert!(bucket_0_10.is_some());
            assert_eq!(bucket_0_10.unwrap().doc_count, 2); // 0.0 and 9.99
            assert!(bucket_10_20.is_some());
            assert_eq!(bucket_10_20.unwrap().doc_count, 2); // 10.0 and 19.99
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_open_ended_ranges() {
        let ranges = vec![
            Range::FromTo {
                from: None,
                to: Some(10.0),
                key: Some("negative".to_string()),
            },
            Range::FromTo {
                from: Some(10.0),
                to: None,
                key: Some("positive".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", -5.0),
            create_test_hit_numeric("2", "price", 5.0),
            create_test_hit_numeric("3", "price", 15.0),
            create_test_hit_numeric("4", "price", 100.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    assert_eq!(buckets.len(), 2);
                    assert_eq!(buckets["negative"].doc_count, 2); // -5.0 and 5.0
                    assert_eq!(buckets["positive"].doc_count, 2); // 15.0 and 100.0
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_integer_values() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(10.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "count".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        // Test with integer values (should be converted to f64)
        let mut hit1 = create_test_hit_numeric("1", "count", 5.0);
        hit1.source["count"] = serde_json::json!(5); // Integer value
        let mut hit2 = create_test_hit_numeric("2", "count", 15.0);
        hit2.source["count"] = serde_json::json!(15); // Integer value

        let hits = vec![hit1, hit2];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1); // Only 5 is in range 0-10
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_empty_hits() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(10.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_missing_field() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(10.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        // Hits without the price field
        let hits = vec![
            SearchHit::new(
                DocumentId::new("1"),
                Score::new(1.0),
                serde_json::json!({ "name": "item1" }),
            ),
            SearchHit::new(
                DocumentId::new("2"),
                Score::new(1.0),
                serde_json::json!({ "name": "item2" }),
            ),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 0); // No documents match (field missing)
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_negative_values() {
        let ranges = vec![
            Range::FromTo {
                from: Some(-100.0),
                to: Some(0.0),
                key: Some("negative".to_string()),
            },
            Range::FromTo {
                from: Some(0.0),
                to: Some(100.0),
                key: Some("positive".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "value", -50.0),
            create_test_hit_numeric("2", "value", 50.0),
            create_test_hit_numeric("3", "value", 0.0), // Boundary: should be in positive range
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    assert_eq!(buckets["negative"].doc_count, 1); // Only -50.0
                    assert_eq!(buckets["positive"].doc_count, 2); // 50.0 and 0.0
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_large_numbers() {
        let ranges = vec![Range::FromTo {
            from: Some(1000000.0),
            to: Some(2000000.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "value", 1500000.0),
            create_test_hit_numeric("2", "value", 2500000.0), // Outside range
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1); // Only 1500000.0
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_overlapping_ranges() {
        // When ranges overlap, document should match first range
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(20.0),
                key: Some("first".to_string()),
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(30.0),
                key: Some("second".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_numeric("1", "value", 15.0)]; // Matches both ranges

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    // Should match first range only (breaks after first match)
                    assert_eq!(buckets["first"].doc_count, 1);
                    assert_eq!(buckets["second"].doc_count, 0);
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_decimal_precision() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(1.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "value", 0.0000001),
            create_test_hit_numeric("2", "value", 0.9999999),
            create_test_hit_numeric("3", "value", 1.0), // Boundary: should NOT be in range (to is exclusive)
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 2); // Only 0.0000001 and 0.9999999
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_all_documents_outside_ranges() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(10.0),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "value", 20.0),
            create_test_hit_numeric("2", "value", 30.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 0); // No documents match
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_zero_boundary() {
        let ranges = vec![
            Range::FromTo {
                from: Some(-10.0),
                to: Some(0.0),
                key: Some("negative".to_string()),
            },
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: Some("positive".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_numeric("1", "value", 0.0)];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    // 0.0 should be in positive range (to is exclusive, but 0.0 >= 0.0)
                    assert_eq!(buckets["negative"].doc_count, 0);
                    assert_eq!(buckets["positive"].doc_count, 1);
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_very_small_interval() {
        let ranges = vec![Range::FromTo {
            from: Some(0.0),
            to: Some(0.0001),
            key: None,
        }];
        let agg = RangeAggregation {
            field: "value".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_numeric("1", "value", 0.00005)];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_merge_with_different_ranges() {
        // Test that merge works even if ranges are slightly different
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: None,
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(20.0),
                key: None,
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: false,
        };
        let field_cache = FieldCache::new();

        // Shard 1: only values in first range
        let hits1 = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 8.0),
        ];
        // Shard 2: values in both ranges
        let hits2 = vec![
            create_test_hit_numeric("3", "price", 5.0),
            create_test_hit_numeric("4", "price", 15.0),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2);
            let bucket_0_10 = buckets.iter().find(|b| b.key.as_str() == Some("0-10"));
            let bucket_10_20 = buckets.iter().find(|b| b.key.as_str() == Some("10-20"));
            assert!(bucket_0_10.is_some());
            assert_eq!(bucket_0_10.unwrap().doc_count, 3); // 5.0, 8.0, 5.0
            assert!(bucket_10_20.is_some());
            assert_eq!(bucket_10_20.unwrap().doc_count, 1); // 15.0
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_range_aggregation_custom_key_names() {
        let ranges = vec![
            Range::FromTo {
                from: Some(0.0),
                to: Some(10.0),
                key: Some("low_price".to_string()),
            },
            Range::FromTo {
                from: Some(10.0),
                to: Some(20.0),
                key: Some("medium_price".to_string()),
            },
            Range::FromTo {
                from: Some(20.0),
                to: None,
                key: Some("high_price".to_string()),
            },
        ];
        let agg = RangeAggregation {
            field: "price".to_string(),
            ranges,
            keyed: true,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
            create_test_hit_numeric("3", "price", 25.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            match bucket_result {
                BucketAggregationResult::Keyed { buckets } => {
                    assert_eq!(buckets.len(), 3);
                    assert_eq!(buckets["low_price"].doc_count, 1);
                    assert_eq!(buckets["medium_price"].doc_count, 1);
                    assert_eq!(buckets["high_price"].doc_count, 1);
                }
                BucketAggregationResult::Array { .. } => {
                    panic!("Expected Keyed format")
                }
            }
        } else {
            panic!("Expected Buckets result");
        }
    }
}
