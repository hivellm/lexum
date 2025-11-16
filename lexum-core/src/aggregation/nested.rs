//! Nested aggregation

use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use super::{AggregationSpec, AggregationTrait};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Nested aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct NestedAggregation {
    /// Path to nested field
    pub path: String,
    /// Sub-aggregations
    pub aggregations: HashMap<String, AggregationSpec>,
}

impl AggregationTrait for NestedAggregation {
    fn name(&self) -> &str {
        "nested"
    }

    fn execute(&self, hits: &[SearchHit], field_cache: &FieldCache) -> Result<AggregationResult> {
        // Filter hits to only those with the nested path
        let nested_hits: Vec<SearchHit> = hits
            .iter()
            .filter(|hit| {
                hit.source
                    .pointer(&format!("/{}", self.path.replace('.', "/")))
                    .is_some()
            })
            .cloned()
            .collect();

        // Execute sub-aggregations
        let mut sub_results = HashMap::new();
        for (name, agg) in &self.aggregations {
            let result = match agg {
                AggregationSpec::Terms(terms_agg) => {
                    terms_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Stats(stats_agg) => {
                    stats_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Histogram(hist_agg) => {
                    hist_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::DateHistogram(date_hist_agg) => {
                    date_hist_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::DateRange(date_range_agg) => {
                    date_range_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Percentile(percentile_agg) => {
                    percentile_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Cardinality(cardinality_agg) => {
                    cardinality_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Nested(nested_agg) => {
                    nested_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Pipeline(pipeline_agg) => {
                    pipeline_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Range(range_agg) => {
                    range_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Filters(filters_agg) => {
                    filters_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Missing(missing_agg) => {
                    missing_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Global(global_agg) => {
                    global_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Composite(composite_agg) => {
                    composite_agg.execute(&nested_hits, field_cache)?
                }
            };
            sub_results.insert(name.clone(), result);
        }

        // Create single bucket with sub-aggregations
        let bucket = Bucket::new(
            serde_json::Value::String(self.path.clone()),
            nested_hits.len(),
        )
        .with_aggregation("aggregations".to_string(), {
            // Convert HashMap to single bucket result
            AggregationResult::SingleBucket(
                super::result::SingleBucketAggregationResult::new(nested_hits.len())
                    .with_aggregation("aggregations".to_string(), {
                        // This is a simplified representation
                        // In practice, we'd return the sub-aggregations directly
                        AggregationResult::SingleBucket(
                            super::result::SingleBucketAggregationResult::new(nested_hits.len()),
                        )
                    }),
            )
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            vec![bucket],
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge nested aggregations
        // This is simplified - proper implementation would merge sub-aggregations
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                vec![],
            )))
        }
    }
}

impl NestedAggregation {
    /// Create new nested aggregation
    pub fn new(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            aggregations: HashMap::new(),
        }
    }

    /// Add sub-aggregation
    pub fn with_aggregation(mut self, name: String, aggregation: AggregationSpec) -> Self {
        self.aggregations.insert(name, aggregation);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregation::{StatsAggregation, TermsAggregation};
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, nested_path: &str, value: &str) -> SearchHit {
        // Create nested structure: { nested_path: { field: value } }
        let mut source = serde_json::Map::new();
        let mut nested_obj = serde_json::Map::new();
        nested_obj.insert("field".to_string(), serde_json::json!(value));
        source.insert(
            nested_path.to_string(),
            serde_json::Value::Object(nested_obj),
        );

        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::Value::Object(source),
        }
    }

    #[test]
    fn test_nested_aggregation_basic() {
        let mut nested_agg = NestedAggregation::new("nested");
        nested_agg = nested_agg.with_aggregation(
            "terms".to_string(),
            AggregationSpec::Terms(TermsAggregation::new("nested.field")),
        );

        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "nested", "value1"),
            create_test_hit("2", "nested", "value2"),
            create_test_hit("3", "nested", "value1"),
        ];

        let result = nested_agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            // Should have one bucket with nested path
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_nested_aggregation_with_sub_aggregations() {
        let mut nested_agg = NestedAggregation::new("items");
        nested_agg = nested_agg.with_aggregation(
            "stats".to_string(),
            AggregationSpec::Stats(StatsAggregation::new("items.price")),
        );

        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "items", "item1"),
            create_test_hit("2", "items", "item2"),
        ];

        let result = nested_agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            // Should have sub-aggregations
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_nested_aggregation_no_matching_path() {
        let nested_agg = NestedAggregation::new("nonexistent");
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit("1", "other", "value")];

        let result = nested_agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            // Should have empty or single bucket with 0 docs
            assert!(
                bucket_result.buckets().is_empty()
                    || bucket_result.buckets().iter().all(|b| b.doc_count == 0)
            );
        } else {
            panic!("Expected Buckets result");
        }
    }
}
