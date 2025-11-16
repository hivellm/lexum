//! Global aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, SingleBucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Global aggregation configuration
/// Global aggregation ignores the query and aggregates over all documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GlobalAggregation {
    /// Name of the aggregation
    #[serde(default = "default_name")]
    pub name: String,
}

fn default_name() -> String {
    "global".to_string()
}

impl AggregationTrait for GlobalAggregation {
    fn name(&self) -> &str {
        &self.name
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Global aggregation returns all documents regardless of query
        // In a real implementation, this would need access to the full index
        // For now, we'll return the count of all hits passed to us
        // Note: This is a simplified implementation

        let doc_count = hits.len();

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(doc_count),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut total_count = 0;

        for result in results {
            if let AggregationResult::SingleBucket(bucket_result) = result {
                total_count += bucket_result.doc_count;
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(total_count),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: value }),
        }
    }

    #[test]
    fn test_global_aggregation_basic() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "pending"),
            create_test_hit("3", "status", "active"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 3);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_empty() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
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
    fn test_global_aggregation_merge() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "status", "active")];
        let hits2 = vec![create_test_hit("2", "status", "pending")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 2);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    /// Test with large dataset (1000 documents)
    /// This test is marked as slow because it processes a large number of documents
    #[test]
    #[cfg(feature = "slow-tests")]
    fn test_global_aggregation_large_dataset() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = (1..=1000)
            .map(|i| create_test_hit(&i.to_string(), "status", "active"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 1000);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_custom_name() {
        let agg = GlobalAggregation {
            name: "custom_global".to_string(),
        };
        assert_eq!(agg.name(), "custom_global");
    }

    #[test]
    fn test_global_aggregation_default_name() {
        let agg = GlobalAggregation {
            name: default_name(),
        };
        assert_eq!(agg.name(), "global");
    }

    #[test]
    fn test_global_aggregation_merge_multiple_shards() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "status", "active")];
        let hits2 = vec![create_test_hit("2", "status", "pending")];
        let hits3 = vec![create_test_hit("3", "status", "active")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();
        let result3 = agg.execute(&hits3, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2, result3]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 3);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_with_various_fields() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            SearchHit {
                id: DocumentId::new("1"),
                score: Score::new(1.0),
                source: serde_json::json!({ "name": "item1", "price": 10.0 }),
            },
            SearchHit {
                id: DocumentId::new("2"),
                score: Score::new(2.0),
                source: serde_json::json!({ "name": "item2", "category": "electronics" }),
            },
            SearchHit {
                id: DocumentId::new("3"),
                score: Score::new(3.0),
                source: serde_json::json!({}),
            },
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 3); // All documents counted
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_merge_empty_results() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };

        let results = vec![];

        let merged = agg.merge(&results).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 0);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    /// Test with very large dataset (10000 documents)
    /// This test is marked as slow because it processes a very large number of documents
    #[test]
    #[cfg(feature = "slow-tests")]
    fn test_global_aggregation_very_large_dataset() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        // Create 10000 hits
        let hits: Vec<SearchHit> = (0..10000)
            .map(|i| create_test_hit(&i.to_string(), "status", "active"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 10000);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_ignores_query() {
        // This test demonstrates that Global aggregation counts all documents
        // regardless of what would match a query (conceptual test)
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        // Mix of documents that would/wouldn't match various queries
        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "pending"),
            create_test_hit("3", "status", "deleted"),
            SearchHit {
                id: DocumentId::new("4"),
                score: Score::new(1.0),
                source: serde_json::json!({}), // No status field
            },
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Global counts ALL documents, regardless of fields
            assert_eq!(bucket_result.doc_count, 4);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_merge_with_zero_counts() {
        let agg = GlobalAggregation {
            name: "global".to_string(),
        };
        let field_cache = FieldCache::new();

        // Some shards have documents, some don't
        let hits1 = vec![create_test_hit("1", "status", "active")];
        let hits2 = vec![]; // Empty shard
        let hits3 = vec![
            create_test_hit("2", "status", "pending"),
            create_test_hit("3", "status", "active"),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();
        let result3 = agg.execute(&hits3, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2, result3]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 3); // 1 + 0 + 2
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_global_aggregation_name_consistency() {
        let agg1 = GlobalAggregation {
            name: "global".to_string(),
        };
        let agg2 = GlobalAggregation {
            name: "global".to_string(),
        };
        let agg3 = GlobalAggregation {
            name: "custom_name".to_string(),
        };

        assert_eq!(agg1.name(), "global");
        assert_eq!(agg2.name(), "global");
        assert_eq!(agg3.name(), "custom_name");
    }
}
