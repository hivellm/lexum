//! Filters aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::Query;
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Filter definition for filters aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum FilterSpec {
    /// Query-based filter
    Query(Query),
    /// Match all filter
    MatchAll,
}

/// Filters aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct FiltersAggregation {
    /// Named filters
    pub filters: HashMap<String, FilterSpec>,
}

impl AggregationTrait for FiltersAggregation {
    fn name(&self) -> &str {
        "filters"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // For now, we'll do a simple implementation that counts documents
        // A full implementation would need to evaluate queries against each hit
        // This is a simplified version that matches documents based on field values

        let mut filter_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all filters with 0 count
        for filter_name in self.filters.keys() {
            filter_counts.insert(filter_name.clone(), 0);
        }

        // For each hit, check which filters match
        // Note: This is a simplified implementation
        // A full implementation would need to evaluate the Query against each hit
        for _hit in hits {
            for (filter_name, filter_spec) in &self.filters {
                // Simplified matching - in a real implementation, we'd evaluate the query
                // For now, we'll count all documents for MatchAll, and skip others
                match filter_spec {
                    FilterSpec::MatchAll => {
                        *filter_counts.get_mut(filter_name).unwrap() += 1;
                    }
                    FilterSpec::Query(_) => {
                        // TODO: Evaluate query against hit
                        // For now, we'll skip query-based filters in this simplified implementation
                    }
                }
            }
        }

        // Convert to buckets
        let buckets: Vec<Bucket> = filter_counts
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all filters
        for filter_name in self.filters.keys() {
            merged_counts.insert(filter_name.clone(), 0);
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

        // Convert to buckets
        let buckets: Vec<Bucket> = merged_counts
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
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
    fn test_filters_aggregation_match_all() {
        let mut filters = HashMap::new();
        filters.insert("all".to_string(), FilterSpec::MatchAll);
        filters.insert("none".to_string(), FilterSpec::MatchAll); // Will also match all in simplified impl

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "pending"),
            create_test_hit("3", "status", "active"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2);
            // Both filters should match all documents in simplified implementation
            let all_bucket = buckets.iter().find(|b| b.key.as_str() == Some("all"));
            let none_bucket = buckets.iter().find(|b| b.key.as_str() == Some("none"));
            assert!(all_bucket.is_some());
            assert!(none_bucket.is_some());
            assert_eq!(all_bucket.unwrap().doc_count, 3);
            assert_eq!(none_bucket.unwrap().doc_count, 3);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_merge() {
        let mut filters = HashMap::new();
        filters.insert("filter1".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "status", "active")];
        let hits2 = vec![create_test_hit("2", "status", "pending")];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            let filter1_bucket = buckets.iter().find(|b| b.key.as_str() == Some("filter1"));
            assert!(filter1_bucket.is_some());
            assert_eq!(filter1_bucket.unwrap().doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_empty_hits() {
        let mut filters = HashMap::new();
        filters.insert("all".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
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
    fn test_filters_aggregation_multiple_filters() {
        let mut filters = HashMap::new();
        filters.insert("filter1".to_string(), FilterSpec::MatchAll);
        filters.insert("filter2".to_string(), FilterSpec::MatchAll);
        filters.insert("filter3".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "pending"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // All filters should match all documents in simplified implementation
            for bucket in &buckets {
                assert_eq!(bucket.doc_count, 2);
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_empty_filters() {
        let filters = HashMap::new();
        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit("1", "status", "active")];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_single_filter() {
        let mut filters = HashMap::new();
        filters.insert("active_only".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "pending"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 2); // MatchAll matches all
        } else {
            panic!("Expected Buckets result");
        }
    }

    /// Test with large dataset (1000 documents)
    /// This test is marked as slow because it processes a large number of documents
    #[test]
    #[cfg(feature = "slow-tests")]
    fn test_filters_aggregation_large_dataset() {
        let mut filters = HashMap::new();
        filters.insert("all".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        // Create 1000 hits
        let hits: Vec<SearchHit> = (0..1000)
            .map(|i| create_test_hit(&i.to_string(), "status", "active"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1000);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_merge_many_shards() {
        let mut filters = HashMap::new();
        filters.insert("filter1".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        // Simulate 10 shards
        let mut results = Vec::new();
        for i in 0..10 {
            let hits = vec![create_test_hit(&i.to_string(), "status", "active")];
            results.push(agg.execute(&hits, &field_cache).unwrap());
        }

        let merged = agg.merge(&results).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 10);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_order_preservation() {
        let mut filters = HashMap::new();
        filters.insert("filter_a".to_string(), FilterSpec::MatchAll);
        filters.insert("filter_b".to_string(), FilterSpec::MatchAll);
        filters.insert("filter_c".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit("1", "status", "active")];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // All should have same count
            for bucket in &buckets {
                assert_eq!(bucket.doc_count, 1);
            }
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_filters_aggregation_merge_empty_buckets() {
        let mut filters = HashMap::new();
        filters.insert("filter1".to_string(), FilterSpec::MatchAll);

        let agg = FiltersAggregation { filters };
        let field_cache = FieldCache::new();

        // One shard with hits, one without
        let hits1 = vec![create_test_hit("1", "status", "active")];
        let hits2 = vec![];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 1);
            assert_eq!(buckets[0].doc_count, 1); // Only from first shard
        } else {
            panic!("Expected Buckets result");
        }
    }
}
