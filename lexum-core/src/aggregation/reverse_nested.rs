//! Reverse nested aggregation

use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use super::{AggregationSpec, AggregationTrait};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Reverse nested aggregation configuration
/// Aggregates parent documents from nested documents
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ReverseNestedAggregation {
    /// Path to nested field (optional, defaults to root)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Sub-aggregations to run on parent documents
    pub aggregations: HashMap<String, AggregationSpec>,
}

impl AggregationTrait for ReverseNestedAggregation {
    fn name(&self) -> &str {
        "reverse_nested"
    }

    fn execute(&self, hits: &[SearchHit], field_cache: &FieldCache) -> Result<AggregationResult> {
        // If path is specified, we need to extract parent documents from nested context
        // For now, we'll treat all hits as parent documents (simplified implementation)
        // In a full implementation, this would track parent document IDs from nested context

        // Get unique parent documents
        // In a real implementation, this would use parent document IDs from the nested context
        let parent_hits: Vec<SearchHit> = if let Some(path) = &self.path {
            // Filter to get parent documents that contain the nested path
            hits.iter()
                .filter(|hit| {
                    // Check if this hit is a parent of nested documents at the specified path
                    hit.source
                        .pointer(&format!("/{}", path.replace('.', "/")))
                        .is_some()
                })
                .cloned()
                .collect()
        } else {
            // No path specified, use all hits as parent documents
            hits.to_vec()
        };

        // Execute sub-aggregations on parent documents
        let mut sub_results = HashMap::new();
        for (name, agg) in &self.aggregations {
            let result = match agg {
                AggregationSpec::Terms(terms_agg) => {
                    terms_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::SignificantTerms(sig_terms_agg) => {
                    sig_terms_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Stats(stats_agg) => {
                    stats_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Histogram(hist_agg) => {
                    hist_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::DateHistogram(date_hist_agg) => {
                    date_hist_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::DateRange(date_range_agg) => {
                    date_range_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::IpRange(ip_range_agg) => {
                    ip_range_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Percentile(percentile_agg) => {
                    percentile_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Cardinality(cardinality_agg) => {
                    cardinality_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Range(range_agg) => {
                    range_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Filters(filters_agg) => {
                    filters_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Missing(missing_agg) => {
                    missing_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Global(global_agg) => {
                    global_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Composite(composite_agg) => {
                    composite_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Sampler(sampler_agg) => {
                    sampler_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::DiversifiedSampler(diversified_sampler_agg) => {
                    diversified_sampler_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Nested(nested_agg) => {
                    nested_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::ReverseNested(reverse_nested_agg) => {
                    reverse_nested_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::ValueCount(value_count_agg) => {
                    value_count_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::Avg(avg_agg) => avg_agg.execute(&parent_hits, field_cache)?,
                AggregationSpec::Sum(sum_agg) => sum_agg.execute(&parent_hits, field_cache)?,
                AggregationSpec::Min(min_agg) => min_agg.execute(&parent_hits, field_cache)?,
                AggregationSpec::Max(max_agg) => max_agg.execute(&parent_hits, field_cache)?,
                AggregationSpec::Pipeline(pipeline_agg) => {
                    pipeline_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                    geohash_grid_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::GeoBounds(geo_bounds_agg) => {
                    geo_bounds_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::GeoDistance(geo_distance_agg) => {
                    geo_distance_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::ExtendedStats(extended_stats_agg) => {
                    extended_stats_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::MedianAbsoluteDeviation(mad_agg) => {
                    mad_agg.execute(&parent_hits, field_cache)?
                }
                AggregationSpec::WeightedAverage(weighted_avg_agg) => {
                    weighted_avg_agg.execute(&parent_hits, field_cache)?
                }
            };
            sub_results.insert(name.clone(), result);
        }

        // Create single bucket with sub-aggregations
        let mut bucket = Bucket::new(serde_json::json!("reverse_nested"), parent_hits.len());
        for (name, result) in sub_results {
            bucket = bucket.with_aggregation(name, result);
        }

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            vec![bucket],
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge reverse nested aggregation results from multiple shards
        let mut merged_buckets: Vec<Bucket> = Vec::new();
        let mut merged_sub_results: HashMap<String, AggregationResult> = HashMap::new();
        let mut total_doc_count = 0;

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    total_doc_count += bucket.doc_count;

                    // Merge sub-aggregations
                    if let Some(ref aggs) = bucket.aggregations {
                        for (name, agg_result) in aggs {
                            merged_sub_results
                                .entry(name.clone())
                                .and_modify(|existing| {
                                    // Merge aggregation results
                                    if let Some(agg_spec) = self.aggregations.get(name) {
                                        if let Ok(merged) = match agg_spec {
                                            AggregationSpec::Terms(terms_agg) => terms_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::SignificantTerms(sig_terms_agg) => {
                                                sig_terms_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::Stats(stats_agg) => stats_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Histogram(hist_agg) => hist_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::DateHistogram(date_hist_agg) => {
                                                date_hist_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::DateRange(date_range_agg) => {
                                                date_range_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::IpRange(ip_range_agg) => ip_range_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Percentile(percentile_agg) => {
                                                percentile_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::Cardinality(cardinality_agg) => {
                                                cardinality_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::Range(range_agg) => range_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Filters(filters_agg) => filters_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Missing(missing_agg) => missing_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Global(global_agg) => global_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Composite(composite_agg) => {
                                                composite_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::Sampler(sampler_agg) => sampler_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::DiversifiedSampler(
                                                diversified_sampler_agg,
                                            ) => diversified_sampler_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Nested(nested_agg) => nested_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::ReverseNested(reverse_nested_agg) => {
                                                reverse_nested_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::ValueCount(value_count_agg) => {
                                                value_count_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::Avg(avg_agg) => avg_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Sum(sum_agg) => sum_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Min(min_agg) => min_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Max(max_agg) => max_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::Pipeline(pipeline_agg) => pipeline_agg
                                                .merge(&[existing.clone(), agg_result.clone()]),
                                            AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                                                geohash_grid_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::GeoBounds(geo_bounds_agg) => {
                                                geo_bounds_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::GeoDistance(geo_distance_agg) => {
                                                geo_distance_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::ExtendedStats(extended_stats_agg) => {
                                                extended_stats_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                            AggregationSpec::MedianAbsoluteDeviation(mad_agg) => {
                                                mad_agg
                                                    .merge(&[existing.clone(), agg_result.clone()])
                                            }
                                        } {
                                            *existing = merged;
                                        }
                                    }
                                })
                                .or_insert_with(|| agg_result.clone());
                        }
                    }
                }
            }
        }

        // Create merged bucket
        let mut bucket = Bucket::new(serde_json::json!("reverse_nested"), total_doc_count);
        for (name, result) in merged_sub_results {
            bucket = bucket.with_aggregation(name, result);
        }

        merged_buckets.push(bucket);

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            merged_buckets,
        )))
    }
}

impl ReverseNestedAggregation {
    /// Create new reverse nested aggregation
    pub fn new() -> Self {
        Self {
            path: None,
            aggregations: HashMap::new(),
        }
    }

    /// Set nested path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    /// Add sub-aggregation
    pub fn with_aggregation(mut self, name: impl Into<String>, agg: AggregationSpec) -> Self {
        self.aggregations.insert(name.into(), agg);
        self
    }
}

impl Default for ReverseNestedAggregation {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregation::{StatsAggregation, TermsAggregation};
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit(id: &str, field: &str, value: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: value }),
        }
    }

    fn create_nested_hit(
        id: &str,
        nested_path: &str,
        nested_value: &str,
        parent_field: &str,
        parent_value: &str,
    ) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({
                parent_field: parent_value,
                nested_path: {
                    "value": nested_value
                }
            }),
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_basic() {
        let mut aggregations = HashMap::new();
        aggregations.insert(
            "parent_stats".to_string(),
            AggregationSpec::Stats(StatsAggregation::new("parent_field")),
        );

        let agg = ReverseNestedAggregation {
            path: Some("nested".to_string()),
            aggregations,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_nested_hit("1", "nested", "value1", "parent_field", "parent1"),
            create_nested_hit("2", "nested", "value2", "parent_field", "parent2"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            let bucket = &bucket_result.buckets()[0];
            assert_eq!(bucket.doc_count, 2);
            assert!(bucket.aggregations.is_some());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_with_sub_aggregations() {
        let mut aggregations = HashMap::new();
        aggregations.insert(
            "parent_terms".to_string(),
            AggregationSpec::Terms(TermsAggregation::new("parent_field")),
        );

        let agg = ReverseNestedAggregation {
            path: Some("nested".to_string()),
            aggregations,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_nested_hit("1", "nested", "value1", "parent_field", "parent1"),
            create_nested_hit("2", "nested", "value2", "parent_field", "parent1"),
            create_nested_hit("3", "nested", "value3", "parent_field", "parent2"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            let bucket = &bucket_result.buckets()[0];
            assert_eq!(bucket.doc_count, 3);
            assert!(bucket.aggregations.is_some());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_no_path() {
        let mut aggregations = HashMap::new();
        aggregations.insert(
            "parent_stats".to_string(),
            AggregationSpec::Stats(StatsAggregation::new("field")),
        );

        let agg = ReverseNestedAggregation {
            path: None,
            aggregations,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "field", "value1"),
            create_test_hit("2", "field", "value2"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_empty_hits() {
        let agg = ReverseNestedAggregation::new();
        let field_cache = FieldCache::new();
        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_merge() {
        let mut aggregations = HashMap::new();
        aggregations.insert(
            "parent_stats".to_string(),
            AggregationSpec::Stats(StatsAggregation::new("parent_field")),
        );

        let agg = ReverseNestedAggregation {
            path: Some("nested".to_string()),
            aggregations,
        };
        let field_cache = FieldCache::new();

        let hits1 = vec![create_nested_hit(
            "1",
            "nested",
            "value1",
            "parent_field",
            "parent1",
        )];
        let hits2 = vec![create_nested_hit(
            "2",
            "nested",
            "value2",
            "parent_field",
            "parent2",
        )];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_reverse_nested_aggregation_no_matching_path() {
        let mut aggregations = HashMap::new();
        aggregations.insert(
            "parent_stats".to_string(),
            AggregationSpec::Stats(StatsAggregation::new("parent_field")),
        );

        let agg = ReverseNestedAggregation {
            path: Some("nonexistent".to_string()),
            aggregations,
        };
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit("1", "field", "value1"),
            create_test_hit("2", "field", "value2"),
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            // No hits match the nested path, so doc_count should be 0
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
