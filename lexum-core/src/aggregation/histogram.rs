//! Histogram aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Histogram aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistogramAggregation {
    /// Field to create histogram on
    pub field: String,
    /// Interval between buckets
    pub interval: f64,
    /// Minimum document count per bucket (buckets with fewer docs are omitted)
    #[serde(default)]
    pub min_doc_count: usize,
}

impl AggregationTrait for HistogramAggregation {
    fn name(&self) -> &str {
        "histogram"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut buckets: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(num) = field_value.as_f64() {
                    let bucket_key = (num / self.interval).floor() as i64;
                    *buckets.entry(bucket_key).or_insert(0) += 1;
                } else if let Some(num) = field_value.as_i64() {
                    let bucket_key = (num as f64 / self.interval).floor() as i64;
                    *buckets.entry(bucket_key).or_insert(0) += 1;
                }
            }
        }

        // Convert to buckets and filter by min_doc_count
        let mut bucket_vec: Vec<Bucket> = buckets
            .into_iter()
            .filter(|(_, count)| *count >= self.min_doc_count)
            .map(|(key, count)| {
                let bucket_value = key as f64 * self.interval;
                Bucket::new(
                    JsonValue::Number(
                        serde_json::Number::from_f64(bucket_value)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ),
                    count,
                )
            })
            .collect();

        // Sort by bucket key
        bucket_vec.sort_by(|a, b| {
            let a_val = a.key.as_f64().unwrap_or(0.0);
            let b_val = b.key.as_f64().unwrap_or(0.0);
            a_val
                .partial_cmp(&b_val)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            bucket_vec,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_buckets: std::collections::HashMap<i64, usize> =
            std::collections::HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let Some(bucket_value) = bucket.key.as_f64() {
                        let bucket_key = (bucket_value / self.interval).floor() as i64;
                        *merged_buckets.entry(bucket_key).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets and filter by min_doc_count
        let mut bucket_vec: Vec<Bucket> = merged_buckets
            .into_iter()
            .filter(|(_, count)| *count >= self.min_doc_count)
            .map(|(key, count)| {
                let bucket_value = key as f64 * self.interval;
                Bucket::new(
                    JsonValue::Number(
                        serde_json::Number::from_f64(bucket_value)
                            .unwrap_or_else(|| serde_json::Number::from(0)),
                    ),
                    count,
                )
            })
            .collect();

        // Sort by bucket key
        bucket_vec.sort_by(|a, b| {
            let a_val = a.key.as_f64().unwrap_or(0.0);
            let b_val = b.key.as_f64().unwrap_or(0.0);
            a_val
                .partial_cmp(&b_val)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            bucket_vec,
        )))
    }
}

impl HistogramAggregation {
    /// Create new histogram aggregation
    pub fn new(field: impl Into<String>, interval: f64) -> Self {
        Self {
            field: field.into(),
            interval,
            min_doc_count: 0,
        }
    }

    /// Set minimum document count
    pub fn with_min_doc_count(mut self, min_doc_count: usize) -> Self {
        self.min_doc_count = min_doc_count;
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
    fn test_histogram_aggregation_basic() {
        let agg = HistogramAggregation::new("price", 10.0);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
            create_test_hit_numeric("3", "price", 25.0),
            create_test_hit_numeric("4", "price", 35.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 4);
            // Each value should be in its own bucket (0-10, 10-20, 20-30, 30-40)
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_histogram_aggregation_min_doc_count() {
        let agg = HistogramAggregation::new("price", 10.0).with_min_doc_count(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
            create_test_hit_numeric("3", "price", 15.0), // Two in same bucket
            create_test_hit_numeric("4", "price", 25.0),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            // Only bucket with 15.0 should have 2 docs, others should be filtered
            let buckets = bucket_result.buckets();
            let bucket_15 = buckets.iter().find(|b| {
                b.key
                    .as_f64()
                    .map(|v| (v - 10.0).abs() < 0.1)
                    .unwrap_or(false)
            });
            assert!(bucket_15.is_some());
            assert_eq!(bucket_15.unwrap().doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_histogram_aggregation_merge() {
        let agg = HistogramAggregation::new("price", 10.0);
        let field_cache = FieldCache::new();

        let hits1 = vec![
            create_test_hit_numeric("1", "price", 5.0),
            create_test_hit_numeric("2", "price", 15.0),
        ];

        let hits2 = vec![
            create_test_hit_numeric("3", "price", 15.0),
            create_test_hit_numeric("4", "price", 25.0),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            // Bucket 10-20 should have 2 docs (one from each result)
            let buckets = bucket_result.buckets();
            let bucket_15 = buckets.iter().find(|b| {
                b.key
                    .as_f64()
                    .map(|v| (v - 10.0).abs() < 0.1)
                    .unwrap_or(false)
            });
            assert!(bucket_15.is_some());
            assert_eq!(bucket_15.unwrap().doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
