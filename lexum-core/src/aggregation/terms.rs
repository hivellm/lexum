//! Terms aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Terms aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TermsAggregation {
    /// Field to aggregate on
    pub field: String,
    /// Maximum number of buckets to return
    #[serde(default = "default_size")]
    pub size: usize,
    /// Sort order (by count or term)
    #[serde(default = "default_sort_order")]
    pub order: TermsSortOrder,
    /// Include missing values in a separate bucket
    #[serde(default)]
    pub missing: Option<JsonValue>,
}

fn default_size() -> usize {
    10
}

fn default_sort_order() -> TermsSortOrder {
    TermsSortOrder::CountDesc
}

/// Terms aggregation sort order
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TermsSortOrder {
    /// Sort by count descending
    CountDesc,
    /// Sort by count ascending
    CountAsc,
    /// Sort by term ascending
    TermAsc,
    /// Sort by term descending
    TermDesc,
}

impl AggregationTrait for TermsAggregation {
    fn name(&self) -> &str {
        "terms"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut term_counts: HashMap<String, usize> = HashMap::new();
        let mut missing_count = 0;

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                let key = match field_value {
                    JsonValue::String(s) => s.clone(),
                    JsonValue::Number(n) => n.to_string(),
                    JsonValue::Bool(b) => b.to_string(),
                    _ => field_value.to_string(),
                };
                *term_counts.entry(key).or_insert(0) += 1;
            } else if self.missing.is_some() {
                missing_count += 1;
            }
        }

        // Convert to buckets
        let mut buckets: Vec<Bucket> = term_counts
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        // Sort buckets
        match self.order {
            TermsSortOrder::CountDesc => {
                buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count));
            }
            TermsSortOrder::CountAsc => {
                buckets.sort_by(|a, b| a.doc_count.cmp(&b.doc_count));
            }
            TermsSortOrder::TermAsc => {
                buckets.sort_by(|a, b| a.key.to_string().cmp(&b.key.to_string()));
            }
            TermsSortOrder::TermDesc => {
                buckets.sort_by(|a, b| b.key.to_string().cmp(&a.key.to_string()));
            }
        }

        // Limit to size
        buckets.truncate(self.size);

        // Add missing bucket if configured
        if missing_count > 0 && self.missing.is_some() {
            let missing_key = self.missing.as_ref().unwrap().clone();
            buckets.push(Bucket::new(missing_key, missing_count));
        }

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    // Extract string from JsonValue properly
                    let key = match &bucket.key {
                        JsonValue::String(s) => s.clone(),
                        JsonValue::Number(n) => n.to_string(),
                        JsonValue::Bool(b) => b.to_string(),
                        _ => bucket.key.to_string(),
                    };
                    *merged_counts.entry(key).or_insert(0) += bucket.doc_count;
                }
            }
        }

        // Convert to buckets
        let mut buckets: Vec<Bucket> = merged_counts
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        // Sort buckets
        match self.order {
            TermsSortOrder::CountDesc => {
                buckets.sort_by(|a, b| b.doc_count.cmp(&a.doc_count));
            }
            TermsSortOrder::CountAsc => {
                buckets.sort_by(|a, b| a.doc_count.cmp(&b.doc_count));
            }
            TermsSortOrder::TermAsc => {
                buckets.sort_by(|a, b| a.key.to_string().cmp(&b.key.to_string()));
            }
            TermsSortOrder::TermDesc => {
                buckets.sort_by(|a, b| b.key.to_string().cmp(&a.key.to_string()));
            }
        }

        // Limit to size
        buckets.truncate(self.size);

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }
}

impl TermsAggregation {
    /// Create new terms aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            size: default_size(),
            order: default_sort_order(),
            missing: None,
        }
    }

    /// Set maximum number of buckets
    pub fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    /// Set sort order
    pub fn with_order(mut self, order: TermsSortOrder) -> Self {
        self.order = order;
        self
    }

    /// Set missing value bucket
    pub fn with_missing(mut self, missing: JsonValue) -> Self {
        self.missing = Some(missing);
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

    fn create_test_hit_numeric(id: &str, field: &str, value: i64) -> SearchHit {
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: value }),
        )
    }

    #[test]
    fn test_terms_aggregation_basic() {
        let agg = TermsAggregation::new("status");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "active"),
            create_test_hit("3", "status", "pending"),
            create_test_hit("4", "status", "active"),
            create_test_hit("5", "status", "pending"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 2);
            // Should be sorted by count desc by default
            assert_eq!(buckets[0].doc_count, 3); // active
            assert_eq!(buckets[1].doc_count, 2); // pending
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_terms_aggregation_size_limit() {
        let agg = TermsAggregation::new("category").with_size(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "A"),
            create_test_hit("2", "category", "A"),
            create_test_hit("3", "category", "B"),
            create_test_hit("4", "category", "B"),
            create_test_hit("5", "category", "C"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2); // Limited to 2
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_terms_aggregation_sort_by_term() {
        let agg = TermsAggregation::new("status").with_order(TermsSortOrder::TermAsc);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "zebra"),
            create_test_hit("2", "status", "alpha"),
            create_test_hit("3", "status", "beta"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets_vec();
            assert_eq!(buckets.len(), 3);
            // Should be sorted alphabetically
            let keys: Vec<String> = buckets
                .iter()
                .map(|b| b.key.as_str().unwrap().to_string())
                .collect();
            assert_eq!(keys, vec!["alpha", "beta", "zebra"]);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_terms_aggregation_missing() {
        let agg =
            TermsAggregation::new("status").with_missing(JsonValue::String("missing".to_string()));
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "other", "value"), // Missing status
            create_test_hit("3", "status", "pending"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            // Should have 2 regular buckets + 1 missing bucket
            assert!(bucket_result.buckets().len() >= 2);
            // Check if missing bucket exists
            let has_missing = bucket_result
                .buckets()
                .iter()
                .any(|b| b.key.as_str() == Some("missing"));
            assert!(has_missing);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_terms_aggregation_merge() {
        let agg = TermsAggregation::new("status");
        let field_cache = FieldCache::new();

        // Create two separate results
        let hits1 = vec![
            create_test_hit("1", "status", "active"),
            create_test_hit("2", "status", "active"),
        ];

        let hits2 = vec![
            create_test_hit("3", "status", "pending"),
            create_test_hit("4", "status", "active"),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            // Should have merged counts: active=3, pending=1
            // With default CountDesc order, active (3) should come before pending (1)
            let buckets = bucket_result.buckets();
            assert_eq!(buckets.len(), 2);

            let active_bucket = buckets.iter().find(|b| b.key.as_str() == Some("active"));
            let pending_bucket = buckets.iter().find(|b| b.key.as_str() == Some("pending"));

            assert!(active_bucket.is_some(), "active bucket should exist");
            assert!(pending_bucket.is_some(), "pending bucket should exist");
            assert_eq!(active_bucket.unwrap().doc_count, 3);
            assert_eq!(pending_bucket.unwrap().doc_count, 1);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_terms_aggregation_numeric_values() {
        let agg = TermsAggregation::new("count");
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "count", 10),
            create_test_hit_numeric("2", "count", 20),
            create_test_hit_numeric("3", "count", 10),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            let buckets = bucket_result.buckets();
            assert_eq!(buckets.len(), 2);
            // Count 10 should appear twice
            let count_10 = buckets.iter().find(|b| b.key.as_str() == Some("10"));
            assert!(count_10.is_some());
            assert_eq!(count_10.unwrap().doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
