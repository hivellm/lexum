//! Missing aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, SingleBucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Missing aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MissingAggregation {
    /// Field to check for missing values
    pub field: String,
}

impl AggregationTrait for MissingAggregation {
    fn name(&self) -> &str {
        "missing"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut missing_count = 0;

        for hit in hits {
            // Check if field is missing or null
            if let Some(field_value) = hit.source.get(&self.field) {
                if field_value.is_null() {
                    missing_count += 1;
                }
            } else {
                missing_count += 1;
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(missing_count),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_missing = 0;

        for result in results {
            if let AggregationResult::SingleBucket(bucket_result) = result {
                total_missing += bucket_result.doc_count;
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(total_missing),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: Option<&str>) -> SearchHit {
        let mut source = serde_json::json!({});
        if let Some(v) = value {
            source[field] = serde_json::json!(v);
        }
        SearchHit::new(DocumentId::new(id), Score::new(1.0), source)
    }

    #[test]
    fn test_missing_aggregation_basic() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", Some("active")),
            create_test_hit("2", "status", None), // Missing
            create_test_hit("3", "status", Some("pending")),
            create_test_hit("4", "status", None), // Missing
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 2); // Two missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_null_values() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit_with_null = create_test_hit("1", "status", Some("active"));
        hit_with_null.source["status"] = serde_json::Value::Null;

        let hits = vec![
            hit_with_null,
            create_test_hit("2", "status", Some("pending")),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 1); // One null value
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_merge() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "status", None)];
        let hits2 = vec![create_test_hit("2", "status", None)];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 2);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_no_missing() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", Some("active")),
            create_test_hit("2", "status", Some("pending")),
            create_test_hit("3", "status", Some("active")),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // No missing values
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_all_missing() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", None),
            create_test_hit("2", "status", None),
            create_test_hit("3", "status", None),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 3); // All missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_empty_hits() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_mixed_null_and_missing() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit_with_null = create_test_hit("1", "status", Some("active"));
        hit_with_null.source["status"] = serde_json::Value::Null;
        let hit_missing = create_test_hit("2", "status", None);
        let hit_present = create_test_hit("3", "status", Some("active"));

        let hits = vec![hit_with_null, hit_missing, hit_present];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 2); // One null, one missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_empty_string() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "status", Some(""));
        hit.source["status"] = serde_json::json!(""); // Empty string is not missing

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // Empty string is not missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_zero_value() {
        let agg = MissingAggregation {
            field: "count".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "count", Some("0"));
        hit.source["count"] = serde_json::json!(0); // Zero is not missing

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // Zero is not missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_false_value() {
        let agg = MissingAggregation {
            field: "active".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "active", Some("false"));
        hit.source["active"] = serde_json::json!(false); // False is not missing

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // False is not missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_empty_array() {
        let agg = MissingAggregation {
            field: "tags".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "tags", Some("[]"));
        hit.source["tags"] = serde_json::json!([]); // Empty array is not missing

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // Empty array is not missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    /// Test with large dataset (1000 documents)
    /// This test is marked as slow because it processes a large number of documents
    #[test]
    #[cfg(feature = "slow-tests")]
    fn test_missing_aggregation_large_dataset() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        // Create 1000 hits, half missing status
        let mut hits = Vec::new();
        for i in 0..1000 {
            if i % 2 == 0 {
                hits.push(create_test_hit(&i.to_string(), "status", Some("active")));
            } else {
                hits.push(create_test_hit(&i.to_string(), "status", None));
            }
        }

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 500); // Half are missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_empty_object() {
        let agg = MissingAggregation {
            field: "metadata".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "metadata", Some("{}"));
        hit.source["metadata"] = serde_json::json!({}); // Empty object is not missing

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0); // Empty object is not missing
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_nested_null() {
        let agg = MissingAggregation {
            field: "nested".to_string(), // Check top-level field
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "nested", Some("{}"));
        hit.source["nested"] = serde_json::json!({
            "field": serde_json::Value::Null
        });

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        // Note: Current implementation only checks if top-level field exists
        // If field exists (even as object), it's NOT missing
        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Field exists (as object), so NOT counted as missing
            assert_eq!(bucket_result.doc_count, 0);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_field_is_null() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        let mut hit = create_test_hit("1", "status", Some("null"));
        hit.source["status"] = serde_json::Value::Null; // Field exists but is null

        let hits = vec![hit];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Field exists but is null, so counted as missing
            assert_eq!(bucket_result.doc_count, 1);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_missing_aggregation_merge_partial_missing() {
        let agg = MissingAggregation {
            field: "status".to_string(),
        };
        let field_cache = FieldCache::new();

        // Shard 1: 2 missing, 1 present
        let hits1 = vec![
            create_test_hit("1", "status", None),
            create_test_hit("2", "status", None),
            create_test_hit("3", "status", Some("active")),
        ];
        // Shard 2: 1 missing, 2 present
        let hits2 = vec![
            create_test_hit("4", "status", None),
            create_test_hit("5", "status", Some("pending")),
            create_test_hit("6", "status", Some("active")),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 3); // 2 + 1 = 3 missing total
        } else {
            panic!("Expected SingleBucket result");
        }
    }
}
