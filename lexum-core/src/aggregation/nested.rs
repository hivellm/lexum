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
        // Extract nested documents from hits
        // Support both single nested objects and arrays of nested objects
        let mut nested_hits: Vec<SearchHit> = Vec::new();
        let path_pointer = format!("/{}", self.path.replace('.', "/"));

        for hit in hits {
            if let Some(nested_value) = hit.source.pointer(&path_pointer) {
                match nested_value {
                    serde_json::Value::Array(arr) => {
                        // Array of nested objects - create a hit for each nested object
                        for (idx, nested_obj) in arr.iter().enumerate() {
                            if let serde_json::Value::Object(_) = nested_obj {
                                // Create a new hit with the nested object as source
                                let mut nested_source = serde_json::Map::new();
                                nested_source.insert("_nested".to_string(), nested_obj.clone());
                                nested_hits.push(SearchHit {
                                    id: hit.id.clone(),
                                    score: hit.score.clone(),
                                    source: serde_json::Value::Object(nested_source),
                                });
                            }
                        }
                    }
                    serde_json::Value::Object(_) => {
                        // Single nested object
                        let mut nested_source = serde_json::Map::new();
                        nested_source.insert("_nested".to_string(), nested_value.clone());
                        nested_hits.push(SearchHit {
                            id: hit.id.clone(),
                            score: hit.score.clone(),
                            source: serde_json::Value::Object(nested_source),
                        });
                    }
                    _ => {
                        // Other types - skip
                    }
                }
            }
        }

        // Execute sub-aggregations
        let mut sub_results = HashMap::new();
        for (name, agg) in &self.aggregations {
            let result = match agg {
                AggregationSpec::Terms(terms_agg) => {
                    terms_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::SignificantTerms(sig_terms_agg) => {
                    sig_terms_agg.execute(&nested_hits, field_cache)?
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
                AggregationSpec::IpRange(ip_range_agg) => {
                    ip_range_agg.execute(&nested_hits, field_cache)?
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
                AggregationSpec::ReverseNested(reverse_nested_agg) => {
                    reverse_nested_agg.execute(&nested_hits, field_cache)?
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
                AggregationSpec::Sampler(sampler_agg) => {
                    sampler_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::DiversifiedSampler(diversified_sampler_agg) => {
                    diversified_sampler_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::ValueCount(value_count_agg) => {
                    value_count_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Avg(avg_agg) => avg_agg.execute(&nested_hits, field_cache)?,
                AggregationSpec::Sum(sum_agg) => sum_agg.execute(&nested_hits, field_cache)?,
                AggregationSpec::Min(min_agg) => min_agg.execute(&nested_hits, field_cache)?,
                AggregationSpec::Max(max_agg) => max_agg.execute(&nested_hits, field_cache)?,
                AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                    geohash_grid_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::GeoBounds(geo_bounds_agg) => {
                    geo_bounds_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::GeoDistance(geo_distance_agg) => {
                    geo_distance_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::ExtendedStats(extended_stats_agg) => {
                    extended_stats_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::MedianAbsoluteDeviation(mad_agg) => {
                    mad_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::WeightedAverage(weighted_avg_agg) => {
                    weighted_avg_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::StringStats(string_stats_agg) => {
                    string_stats_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Boxplot(boxplot_agg) => {
                    boxplot_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Rate(rate_agg) => rate_agg.execute(&nested_hits, field_cache)?,
                AggregationSpec::TTest(t_test_agg) => {
                    t_test_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::TopHits(top_hits_agg) => {
                    top_hits_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::ScriptedMetric(scripted_metric_agg) => {
                    scripted_metric_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::BucketScript(bucket_script_agg) => {
                    bucket_script_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::BucketSelector(bucket_selector_agg) => {
                    bucket_selector_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::BucketSort(bucket_sort_agg) => {
                    bucket_sort_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::CumulativeSum(cumulative_sum_agg) => {
                    cumulative_sum_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::CumulativeCardinality(cumulative_cardinality_agg) => {
                    cumulative_cardinality_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Derivative(derivative_agg) => {
                    derivative_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::MovingAverage(moving_avg_agg) => {
                    moving_avg_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::MovingFunction(moving_function_agg) => {
                    moving_function_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::SerialDifferencing(serial_diff_agg) => {
                    serial_diff_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Normalize(normalize_agg) => {
                    normalize_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Children(children_agg) => {
                    children_agg.execute(&nested_hits, field_cache)?
                }
                AggregationSpec::Parent(parent_agg) => {
                    parent_agg.execute(&nested_hits, field_cache)?
                }
            };
            sub_results.insert(name.clone(), result);
        }

        // Create single bucket with sub-aggregations
        // In Elasticsearch, nested aggregation returns a single bucket containing sub-aggregations
        let mut single_bucket =
            super::result::SingleBucketAggregationResult::new(nested_hits.len());

        // Add sub-aggregation results
        for (name, result) in sub_results {
            single_bucket = single_bucket.with_aggregation(name, result);
        }

        Ok(AggregationResult::SingleBucket(single_bucket))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge nested aggregation results
        // Nested aggregation returns a SingleBucket result with sub-aggregations
        if results.is_empty() {
            return Ok(AggregationResult::SingleBucket(
                super::result::SingleBucketAggregationResult::new(0),
            ));
        }

        // Extract all sub-aggregation results from each shard result
        let mut merged_sub_results: HashMap<String, Vec<AggregationResult>> = HashMap::new();

        for result in results {
            if let AggregationResult::SingleBucket(single_bucket) = result {
                for (name, sub_result) in single_bucket.aggregations() {
                    merged_sub_results
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push(sub_result.clone());
                }
            }
        }

        // Merge each sub-aggregation
        let mut merged_aggregations = HashMap::new();
        for (name, agg_spec) in &self.aggregations {
            if let Some(sub_results) = merged_sub_results.get(name) {
                if let Ok(merged_result) = match agg_spec {
                    AggregationSpec::Terms(terms_agg) => terms_agg.merge(sub_results),
                    AggregationSpec::SignificantTerms(sig_terms_agg) => {
                        sig_terms_agg.merge(sub_results)
                    }
                    AggregationSpec::Stats(stats_agg) => stats_agg.merge(sub_results),
                    AggregationSpec::Histogram(hist_agg) => hist_agg.merge(sub_results),
                    AggregationSpec::DateHistogram(date_hist_agg) => {
                        date_hist_agg.merge(sub_results)
                    }
                    AggregationSpec::DateRange(date_range_agg) => date_range_agg.merge(sub_results),
                    AggregationSpec::IpRange(ip_range_agg) => ip_range_agg.merge(sub_results),
                    AggregationSpec::Percentile(percentile_agg) => {
                        percentile_agg.merge(sub_results)
                    }
                    AggregationSpec::Cardinality(cardinality_agg) => {
                        cardinality_agg.merge(sub_results)
                    }
                    AggregationSpec::Nested(nested_agg) => nested_agg.merge(sub_results),
                    AggregationSpec::ReverseNested(reverse_nested_agg) => {
                        reverse_nested_agg.merge(sub_results)
                    }
                    AggregationSpec::Pipeline(pipeline_agg) => pipeline_agg.merge(sub_results),
                    AggregationSpec::Range(range_agg) => range_agg.merge(sub_results),
                    AggregationSpec::Filters(filters_agg) => filters_agg.merge(sub_results),
                    AggregationSpec::Missing(missing_agg) => missing_agg.merge(sub_results),
                    AggregationSpec::Global(global_agg) => global_agg.merge(sub_results),
                    AggregationSpec::Composite(composite_agg) => composite_agg.merge(sub_results),
                    AggregationSpec::Sampler(sampler_agg) => sampler_agg.merge(sub_results),
                    AggregationSpec::DiversifiedSampler(diversified_sampler_agg) => {
                        diversified_sampler_agg.merge(sub_results)
                    }
                    AggregationSpec::ValueCount(value_count_agg) => {
                        value_count_agg.merge(sub_results)
                    }
                    AggregationSpec::Avg(avg_agg) => avg_agg.merge(sub_results),
                    AggregationSpec::Sum(sum_agg) => sum_agg.merge(sub_results),
                    AggregationSpec::Min(min_agg) => min_agg.merge(sub_results),
                    AggregationSpec::Max(max_agg) => max_agg.merge(sub_results),
                    AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                        geohash_grid_agg.merge(sub_results)
                    }
                    AggregationSpec::GeoBounds(geo_bounds_agg) => geo_bounds_agg.merge(sub_results),
                    AggregationSpec::GeoDistance(geo_distance_agg) => {
                        geo_distance_agg.merge(sub_results)
                    }
                    AggregationSpec::ExtendedStats(extended_stats_agg) => {
                        extended_stats_agg.merge(sub_results)
                    }
                    AggregationSpec::MedianAbsoluteDeviation(mad_agg) => mad_agg.merge(sub_results),
                    AggregationSpec::WeightedAverage(weighted_avg_agg) => {
                        weighted_avg_agg.merge(sub_results)
                    }
                    AggregationSpec::StringStats(string_stats_agg) => {
                        string_stats_agg.merge(sub_results)
                    }
                    AggregationSpec::Boxplot(boxplot_agg) => boxplot_agg.merge(sub_results),
                    AggregationSpec::Rate(rate_agg) => rate_agg.merge(sub_results),
                    AggregationSpec::TTest(t_test_agg) => t_test_agg.merge(sub_results),
                    AggregationSpec::TopHits(top_hits_agg) => top_hits_agg.merge(sub_results),
                    AggregationSpec::ScriptedMetric(scripted_metric_agg) => {
                        scripted_metric_agg.merge(sub_results)
                    }
                    AggregationSpec::BucketScript(bucket_script_agg) => {
                        bucket_script_agg.merge(sub_results)
                    }
                    AggregationSpec::BucketSelector(bucket_selector_agg) => {
                        bucket_selector_agg.merge(sub_results)
                    }
                    AggregationSpec::BucketSort(bucket_sort_agg) => {
                        bucket_sort_agg.merge(sub_results)
                    }
                    AggregationSpec::CumulativeSum(cumulative_sum_agg) => {
                        cumulative_sum_agg.merge(sub_results)
                    }
                    AggregationSpec::CumulativeCardinality(cumulative_cardinality_agg) => {
                        cumulative_cardinality_agg.merge(sub_results)
                    }
                    AggregationSpec::Derivative(derivative_agg) => {
                        derivative_agg.merge(sub_results)
                    }
                    AggregationSpec::MovingAverage(moving_avg_agg) => {
                        moving_avg_agg.merge(sub_results)
                    }
                    AggregationSpec::MovingFunction(moving_function_agg) => {
                        moving_function_agg.merge(sub_results)
                    }
                    AggregationSpec::SerialDifferencing(serial_diff_agg) => {
                        serial_diff_agg.merge(sub_results)
                    }
                    AggregationSpec::Normalize(normalize_agg) => normalize_agg.merge(sub_results),
                    AggregationSpec::Children(children_agg) => children_agg.merge(sub_results),
                    AggregationSpec::Parent(parent_agg) => parent_agg.merge(sub_results),
                } {
                    merged_aggregations.insert(name.clone(), merged_result);
                }
            }
        }

        // Calculate total doc count from all shards
        let total_doc_count: usize = results
            .iter()
            .filter_map(|r| {
                if let AggregationResult::SingleBucket(sb) = r {
                    Some(sb.doc_count())
                } else {
                    None
                }
            })
            .sum();

        let mut merged_bucket = super::result::SingleBucketAggregationResult::new(total_doc_count);
        for (name, result) in merged_aggregations {
            merged_bucket = merged_bucket.with_aggregation(name, result);
        }

        Ok(AggregationResult::SingleBucket(merged_bucket))
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
