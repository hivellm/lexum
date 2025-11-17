//! Aggregation executor

use super::AggregationTrait;
use super::result::AggregationResult;
use crate::error::Result;
use crate::index::Index;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use std::sync::Arc;

/// Aggregation executor
pub struct AggregationExecutor {
    #[allow(dead_code)]
    index: Arc<Index>,
    field_cache: Arc<FieldCache>,
}

impl AggregationExecutor {
    /// Create new aggregation executor
    pub fn new(index: Arc<Index>, field_cache: Arc<FieldCache>) -> Self {
        Self { index, field_cache }
    }

    /// Execute aggregations on search hits
    pub fn execute(
        &self,
        aggregations: &[super::AggregationSpec],
        hits: &[SearchHit],
    ) -> Result<std::collections::HashMap<String, AggregationResult>> {
        let mut results = std::collections::HashMap::new();

        for agg in aggregations {
            let result = match agg {
                super::AggregationSpec::Terms(terms_agg) => {
                    terms_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::SignificantTerms(sig_terms_agg) => {
                    sig_terms_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Stats(stats_agg) => {
                    stats_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Histogram(hist_agg) => {
                    hist_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::DateHistogram(date_hist_agg) => {
                    date_hist_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::DateRange(date_range_agg) => {
                    date_range_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::IpRange(ip_range_agg) => {
                    ip_range_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Percentile(percentile_agg) => {
                    percentile_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Cardinality(cardinality_agg) => {
                    cardinality_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Nested(nested_agg) => {
                    nested_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::ReverseNested(reverse_nested_agg) => {
                    reverse_nested_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Pipeline(pipeline_agg) => {
                    pipeline_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Range(range_agg) => {
                    range_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Filters(filters_agg) => {
                    filters_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Missing(missing_agg) => {
                    missing_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Global(global_agg) => {
                    global_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Composite(composite_agg) => {
                    composite_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Sampler(sampler_agg) => {
                    sampler_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::DiversifiedSampler(diversified_sampler_agg) => {
                    diversified_sampler_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::ValueCount(value_count_agg) => {
                    value_count_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Avg(avg_agg) => avg_agg.execute(hits, &self.field_cache)?,
                super::AggregationSpec::Sum(sum_agg) => sum_agg.execute(hits, &self.field_cache)?,
                super::AggregationSpec::Min(min_agg) => min_agg.execute(hits, &self.field_cache)?,
                super::AggregationSpec::Max(max_agg) => max_agg.execute(hits, &self.field_cache)?,
                super::AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                    geohash_grid_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::GeoBounds(geo_bounds_agg) => {
                    geo_bounds_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::GeoDistance(geo_distance_agg) => {
                    geo_distance_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::ExtendedStats(extended_stats_agg) => {
                    extended_stats_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::MedianAbsoluteDeviation(mad_agg) => {
                    mad_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::WeightedAverage(weighted_avg_agg) => {
                    weighted_avg_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::StringStats(string_stats_agg) => {
                    string_stats_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Boxplot(boxplot_agg) => {
                    boxplot_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Rate(rate_agg) => {
                    rate_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::TTest(t_test_agg) => {
                    t_test_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::TopHits(top_hits_agg) => {
                    top_hits_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::ScriptedMetric(scripted_metric_agg) => {
                    scripted_metric_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::BucketScript(bucket_script_agg) => {
                    bucket_script_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::BucketSelector(bucket_selector_agg) => {
                    bucket_selector_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::BucketSort(bucket_sort_agg) => {
                    bucket_sort_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::CumulativeSum(cumulative_sum_agg) => {
                    cumulative_sum_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::CumulativeCardinality(cumulative_cardinality_agg) => {
                    cumulative_cardinality_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Derivative(derivative_agg) => {
                    derivative_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::MovingAverage(moving_avg_agg) => {
                    moving_avg_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::MovingFunction(moving_function_agg) => {
                    moving_function_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::SerialDifferencing(serial_diff_agg) => {
                    serial_diff_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Normalize(normalize_agg) => {
                    normalize_agg.execute(hits, &self.field_cache)?
                }
            };

            results.insert(agg.name().to_string(), result);
        }

        Ok(results)
    }

    /// Merge aggregation results from multiple shards
    pub fn merge(
        &self,
        aggregation: &super::AggregationSpec,
        results: &[AggregationResult],
    ) -> Result<AggregationResult> {
        match aggregation {
            super::AggregationSpec::Terms(terms_agg) => terms_agg.merge(results),
            super::AggregationSpec::SignificantTerms(sig_terms_agg) => sig_terms_agg.merge(results),
            super::AggregationSpec::Stats(stats_agg) => stats_agg.merge(results),
            super::AggregationSpec::Histogram(hist_agg) => hist_agg.merge(results),
            super::AggregationSpec::DateHistogram(date_hist_agg) => date_hist_agg.merge(results),
            super::AggregationSpec::DateRange(date_range_agg) => date_range_agg.merge(results),
            super::AggregationSpec::IpRange(ip_range_agg) => ip_range_agg.merge(results),
            super::AggregationSpec::Percentile(percentile_agg) => percentile_agg.merge(results),
            super::AggregationSpec::Cardinality(cardinality_agg) => cardinality_agg.merge(results),
            super::AggregationSpec::Nested(nested_agg) => nested_agg.merge(results),
            super::AggregationSpec::ReverseNested(reverse_nested_agg) => {
                reverse_nested_agg.merge(results)
            }
            super::AggregationSpec::Pipeline(pipeline_agg) => pipeline_agg.merge(results),
            super::AggregationSpec::Range(range_agg) => range_agg.merge(results),
            super::AggregationSpec::Filters(filters_agg) => filters_agg.merge(results),
            super::AggregationSpec::Missing(missing_agg) => missing_agg.merge(results),
            super::AggregationSpec::Global(global_agg) => global_agg.merge(results),
            super::AggregationSpec::Composite(composite_agg) => composite_agg.merge(results),
            super::AggregationSpec::Sampler(sampler_agg) => sampler_agg.merge(results),
            super::AggregationSpec::DiversifiedSampler(diversified_sampler_agg) => {
                diversified_sampler_agg.merge(results)
            }
            super::AggregationSpec::ValueCount(value_count_agg) => value_count_agg.merge(results),
            super::AggregationSpec::Avg(avg_agg) => avg_agg.merge(results),
            super::AggregationSpec::Sum(sum_agg) => sum_agg.merge(results),
            super::AggregationSpec::Min(min_agg) => min_agg.merge(results),
            super::AggregationSpec::Max(max_agg) => max_agg.merge(results),
            super::AggregationSpec::GeohashGrid(geohash_grid_agg) => {
                geohash_grid_agg.merge(results)
            }
            super::AggregationSpec::GeoBounds(geo_bounds_agg) => geo_bounds_agg.merge(results),
            super::AggregationSpec::GeoDistance(geo_distance_agg) => {
                geo_distance_agg.merge(results)
            }
            super::AggregationSpec::ExtendedStats(extended_stats_agg) => {
                extended_stats_agg.merge(results)
            }
            super::AggregationSpec::MedianAbsoluteDeviation(mad_agg) => mad_agg.merge(results),
            super::AggregationSpec::WeightedAverage(weighted_avg_agg) => {
                weighted_avg_agg.merge(results)
            }
            super::AggregationSpec::StringStats(string_stats_agg) => {
                string_stats_agg.merge(results)
            }
            super::AggregationSpec::Boxplot(boxplot_agg) => boxplot_agg.merge(results),
            super::AggregationSpec::Rate(rate_agg) => rate_agg.merge(results),
            super::AggregationSpec::TTest(t_test_agg) => t_test_agg.merge(results),
            super::AggregationSpec::TopHits(top_hits_agg) => top_hits_agg.merge(results),
            super::AggregationSpec::ScriptedMetric(scripted_metric_agg) => {
                scripted_metric_agg.merge(results)
            }
            super::AggregationSpec::BucketScript(bucket_script_agg) => {
                bucket_script_agg.merge(results)
            }
            super::AggregationSpec::BucketSelector(bucket_selector_agg) => {
                bucket_selector_agg.merge(results)
            }
            super::AggregationSpec::BucketSort(bucket_sort_agg) => bucket_sort_agg.merge(results),
            super::AggregationSpec::CumulativeSum(cumulative_sum_agg) => {
                cumulative_sum_agg.merge(results)
            }
            super::AggregationSpec::CumulativeCardinality(cumulative_cardinality_agg) => {
                cumulative_cardinality_agg.merge(results)
            }
            super::AggregationSpec::Derivative(derivative_agg) => derivative_agg.merge(results),
            super::AggregationSpec::MovingAverage(moving_avg_agg) => moving_avg_agg.merge(results),
            super::AggregationSpec::MovingFunction(moving_function_agg) => {
                moving_function_agg.merge(results)
            }
            super::AggregationSpec::SerialDifferencing(serial_diff_agg) => {
                serial_diff_agg.merge(results)
            }
            super::AggregationSpec::Normalize(normalize_agg) => normalize_agg.merge(results),
        }
    }
}
