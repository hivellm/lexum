//! Sampler aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, SingleBucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Sampler aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SamplerAggregation {
    /// Shard size - number of top-scoring documents to sample from each shard
    #[serde(default = "default_shard_size")]
    pub shard_size: usize,
}

fn default_shard_size() -> usize {
    100
}

impl AggregationTrait for SamplerAggregation {
    fn name(&self) -> &str {
        "sampler"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Sample documents - take top-scoring documents up to shard_size
        // In a real implementation, this would sample randomly from top documents
        // For now, we'll take the top-scoring documents
        let sampled_hits: Vec<&SearchHit> = hits.iter().take(self.shard_size).collect();

        // Return single bucket with sampled documents
        // In a full implementation, sub-aggregations would run on sampled documents
        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(sampled_hits.len()),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge sampler results from multiple shards
        let mut total_sampled = 0;

        for result in results {
            if let AggregationResult::SingleBucket(bucket_result) = result {
                total_sampled += bucket_result.doc_count;
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(total_sampled),
        ))
    }
}

impl SamplerAggregation {
    /// Create new sampler aggregation
    pub fn new() -> Self {
        Self {
            shard_size: default_shard_size(),
        }
    }

    /// Set shard size
    pub fn with_shard_size(mut self, shard_size: usize) -> Self {
        self.shard_size = shard_size;
        self
    }
}

impl Default for SamplerAggregation {
    fn default() -> Self {
        Self::new()
    }
}

/// Diversified sampler aggregation configuration
/// Samples documents while ensuring diversity based on a field
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiversifiedSamplerAggregation {
    /// Shard size - number of top-scoring documents to sample from each shard
    #[serde(default = "default_shard_size")]
    pub shard_size: usize,
    /// Field to use for diversification
    pub field: String,
    /// Maximum number of documents per value of the field
    #[serde(default = "default_max_docs_per_value")]
    pub max_docs_per_value: usize,
}

fn default_max_docs_per_value() -> usize {
    1
}

impl AggregationTrait for DiversifiedSamplerAggregation {
    fn name(&self) -> &str {
        "diversified_sampler"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Sample documents with diversification
        // Track how many documents we've sampled per field value
        let mut value_counts: HashMap<String, usize> = HashMap::new();
        let mut sampled_hits: Vec<&SearchHit> = Vec::new();

        for hit in hits.iter().take(self.shard_size) {
            // Extract field value for diversification
            let field_value = hit
                .source
                .get(&self.field)
                .map(|v| v.to_string())
                .unwrap_or_else(|| "null".to_string());

            let count = value_counts.get(&field_value).copied().unwrap_or(0);

            // Only include if we haven't exceeded max_docs_per_value for this value
            if count < self.max_docs_per_value {
                sampled_hits.push(hit);
                value_counts.insert(field_value, count + 1);
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(sampled_hits.len()),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge diversified sampler results from multiple shards
        let mut total_sampled = 0;

        for result in results {
            if let AggregationResult::SingleBucket(bucket_result) = result {
                total_sampled += bucket_result.doc_count;
            }
        }

        Ok(AggregationResult::SingleBucket(
            SingleBucketAggregationResult::new(total_sampled),
        ))
    }
}

impl DiversifiedSamplerAggregation {
    /// Create new diversified sampler aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            shard_size: default_shard_size(),
            field: field.into(),
            max_docs_per_value: default_max_docs_per_value(),
        }
    }

    /// Set shard size
    pub fn with_shard_size(mut self, shard_size: usize) -> Self {
        self.shard_size = shard_size;
        self
    }

    /// Set max documents per value
    pub fn with_max_docs_per_value(mut self, max_docs_per_value: usize) -> Self {
        self.max_docs_per_value = max_docs_per_value;
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

    #[test]
    fn test_sampler_aggregation_basic() {
        let agg = SamplerAggregation::new().with_shard_size(5);
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = (0..10)
            .map(|i| create_test_hit(&i.to_string(), "category", "electronics"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 5); // Should sample 5 documents
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_sampler_aggregation_shard_size_limit() {
        let agg = SamplerAggregation::new().with_shard_size(3);
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = (0..10)
            .map(|i| create_test_hit(&i.to_string(), "category", "electronics"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 3); // Should sample only 3 documents
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_sampler_aggregation_empty_hits() {
        let agg = SamplerAggregation::new();
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
    fn test_sampler_aggregation_merge() {
        let agg = SamplerAggregation::new().with_shard_size(5);
        let field_cache = FieldCache::new();

        let hits1: Vec<SearchHit> = (0..5)
            .map(|i| create_test_hit(&i.to_string(), "category", "electronics"))
            .collect();
        let hits2: Vec<SearchHit> = (5..10)
            .map(|i| create_test_hit(&i.to_string(), "category", "electronics"))
            .collect();

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = merged {
            assert_eq!(bucket_result.doc_count, 10); // 5 + 5
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_diversified_sampler_aggregation_basic() {
        let agg = DiversifiedSamplerAggregation::new("category").with_max_docs_per_value(2);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "electronics"),
            create_test_hit("2", "category", "electronics"),
            create_test_hit("3", "category", "electronics"), // Should be excluded (max 2 per value)
            create_test_hit("4", "category", "clothing"),
            create_test_hit("5", "category", "clothing"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Should have 4 documents: 2 electronics + 2 clothing
            assert_eq!(bucket_result.doc_count, 4);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_diversified_sampler_aggregation_max_docs_per_value() {
        let agg = DiversifiedSamplerAggregation::new("category")
            .with_max_docs_per_value(1)
            .with_shard_size(10);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "category", "electronics"),
            create_test_hit("2", "category", "electronics"), // Should be excluded
            create_test_hit("3", "category", "clothing"),
            create_test_hit("4", "category", "clothing"), // Should be excluded
            create_test_hit("5", "category", "books"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Should have 3 documents: 1 per category
            assert_eq!(bucket_result.doc_count, 3);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_diversified_sampler_aggregation_shard_size_limit() {
        let agg = DiversifiedSamplerAggregation::new("category")
            .with_shard_size(3)
            .with_max_docs_per_value(10);
        let field_cache = FieldCache::new();

        let hits: Vec<SearchHit> = (0..10)
            .map(|i| create_test_hit(&i.to_string(), "category", "electronics"))
            .collect();

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Should sample only 3 documents (shard_size limit)
            assert_eq!(bucket_result.doc_count, 3);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_diversified_sampler_aggregation_missing_field() {
        let agg = DiversifiedSamplerAggregation::new("category")
            .with_max_docs_per_value(1)
            .with_shard_size(10);
        let field_cache = FieldCache::new();

        let hits = vec![
            SearchHit::new(
                DocumentId::new("1"),
                Score::new(1.0),
                serde_json::json!({ "other_field": "value" }),
            ),
            SearchHit::new(
                DocumentId::new("2"),
                Score::new(1.0),
                serde_json::json!({ "other_field": "value" }),
            ),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            // Both should be included (missing field = "null" value, max 1 per value)
            assert_eq!(bucket_result.doc_count, 1);
        } else {
            panic!("Expected SingleBucket result");
        }
    }

    #[test]
    fn test_diversified_sampler_aggregation_merge() {
        let agg = DiversifiedSamplerAggregation::new("category").with_max_docs_per_value(2);
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit("1", "category", "electronics")];
        let hits2 = vec![create_test_hit("2", "category", "electronics")];

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
    fn test_diversified_sampler_aggregation_empty_hits() {
        let agg = DiversifiedSamplerAggregation::new("category");
        let field_cache = FieldCache::new();
        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::SingleBucket(bucket_result) = result {
            assert_eq!(bucket_result.doc_count, 0);
        } else {
            panic!("Expected SingleBucket result");
        }
    }
}
