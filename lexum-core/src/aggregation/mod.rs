//! Aggregation framework
//!
//! Provides functionality for computing aggregations on search results including:
//! - Terms aggregation (top values)
//! - Stats aggregation (min, max, avg, sum, count)
//! - Histogram aggregation (numeric buckets)
//! - Date histogram (time-based buckets)
//! - Percentile aggregation
//! - Cardinality aggregation (unique count)
//! - Nested aggregations
//! - Pipeline aggregations

pub mod average;
pub mod boxplot;
pub mod bucket_script;
pub mod bucket_selector;
pub mod bucket_sort;
pub mod cardinality;
pub mod children;
pub mod composite;
pub mod cumulative_cardinality;
pub mod cumulative_sum;
pub mod date_histogram;
pub mod date_range;
pub mod derivative;
pub mod executor;
pub mod extended_stats;
pub mod filters;
pub mod geo_bounds;
pub mod geo_distance;
pub mod geohash_grid;
pub mod global;
pub mod histogram;
pub mod ip_range;
pub mod max;
pub mod median_absolute_deviation;
pub mod min;
pub mod missing;
pub mod moving_average;
pub mod moving_function;
pub mod nested;
pub mod normalize;
pub mod parent;
pub mod percentile;
pub mod pipeline;
pub mod range;
pub mod rate;
pub mod result;
pub mod reverse_nested;
pub mod sampler;
pub mod scripted_metric;
pub mod serial_differencing;
pub mod significant_terms;
pub mod stats;
pub mod string_stats;
pub mod sum;
pub mod t_test;
pub mod terms;
pub mod top_hits;
pub mod value_count;
pub mod weighted_average;

pub use average::AverageAggregation;
pub use boxplot::{BoxplotAggregation, BoxplotResult};
pub use bucket_script::BucketScriptAggregation;
pub use bucket_selector::BucketSelectorAggregation;
pub use bucket_sort::BucketSortAggregation;
pub use cardinality::CardinalityAggregation;
pub use children::ChildrenAggregation;
pub use composite::CompositeAggregation;
pub use cumulative_cardinality::CumulativeCardinalityAggregation;
pub use cumulative_sum::CumulativeSumAggregation;
pub use date_histogram::DateHistogramAggregation;
pub use date_range::DateRangeAggregation;
pub use derivative::DerivativeAggregation;
pub use executor::AggregationExecutor;
pub use extended_stats::{ExtendedStatsAggregation, ExtendedStatsResult};
pub use filters::FiltersAggregation;
pub use geo_bounds::{GeoBoundsAggregation, GeoBoundsResult};
pub use geo_distance::{DistanceRange, GeoDistanceAggregation, GeoDistanceBucket};
pub use geohash_grid::{GeohashGridAggregation, GeohashGridBucket};
pub use global::GlobalAggregation;
pub use histogram::HistogramAggregation;
pub use ip_range::IpRangeAggregation;
pub use max::MaxAggregation;
pub use median_absolute_deviation::{
    MedianAbsoluteDeviationAggregation, MedianAbsoluteDeviationResult,
};
pub use min::MinAggregation;
pub use missing::MissingAggregation;
pub use moving_average::{MovingAverageAggregation, MovingAverageModel};
pub use moving_function::MovingFunctionAggregation;
pub use nested::NestedAggregation;
pub use normalize::{NormalizeAggregation, NormalizeMethod};
pub use parent::ParentAggregation;
pub use percentile::PercentileAggregation;
pub use pipeline::PipelineAggregation;
pub use range::RangeAggregation;
pub use rate::{RateAggregation, RateResult};
pub use result::{
    AggregationResult, Bucket, BucketAggregationResult, MetricAggregationResult,
    SingleBucketAggregationResult,
};
pub use reverse_nested::ReverseNestedAggregation;
pub use sampler::{DiversifiedSamplerAggregation, SamplerAggregation};
pub use scripted_metric::{ScriptedMetricAggregation, ScriptedMetricResult};
pub use significant_terms::SignificantTermsAggregation;
pub use stats::StatsAggregation;
pub use string_stats::{StringStatsAggregation, StringStatsResult};
pub use sum::SumAggregation;
pub use t_test::{TTestAggregation, TTestGroup, TTestGroupStats, TTestResult};
pub use terms::TermsAggregation;
pub use top_hits::{TopHitsAggregation, TopHitsHighlight, TopHitsResult};
pub use value_count::ValueCountAggregation;
pub use weighted_average::{WeightedAverageAggregation, WeightedAverageResult};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Aggregation trait that all aggregations must implement
pub trait AggregationTrait: Send + Sync {
    /// Get the name of this aggregation
    fn name(&self) -> &str;

    /// Execute the aggregation on search results
    fn execute(
        &self,
        hits: &[crate::search::result::SearchHit],
        field_cache: &crate::search::field_cache::FieldCache,
    ) -> crate::error::Result<AggregationResult>;

    /// Merge results from multiple shards/executions
    fn merge(&self, results: &[AggregationResult]) -> crate::error::Result<AggregationResult>;
}

/// Main aggregation specification enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AggregationSpec {
    /// Terms aggregation (top values)
    Terms(TermsAggregation),
    /// Significant terms aggregation (statistically significant terms)
    SignificantTerms(SignificantTermsAggregation),
    /// Stats aggregation (min, max, avg, sum, count)
    Stats(StatsAggregation),
    /// Histogram aggregation (numeric buckets)
    Histogram(HistogramAggregation),
    /// Date histogram aggregation (time-based buckets)
    DateHistogram(DateHistogramAggregation),
    /// Date range aggregation (date ranges)
    DateRange(DateRangeAggregation),
    /// IP range aggregation (IP ranges with CIDR support)
    IpRange(IpRangeAggregation),
    /// Percentile aggregation
    Percentile(PercentileAggregation),
    /// Cardinality aggregation (unique count)
    Cardinality(CardinalityAggregation),
    /// Nested aggregation (sub-aggregations)
    Nested(NestedAggregation),
    /// Reverse nested aggregation (parent documents from nested)
    ReverseNested(ReverseNestedAggregation),
    /// Pipeline aggregation (derived from other aggregations)
    Pipeline(PipelineAggregation),
    /// Range aggregation (numeric ranges)
    Range(RangeAggregation),
    /// Filters aggregation (multiple named filters)
    Filters(FiltersAggregation),
    /// Missing aggregation (documents with missing values)
    Missing(MissingAggregation),
    /// Global aggregation (global scope)
    Global(GlobalAggregation),
    /// Composite aggregation (multi-level grouping)
    Composite(CompositeAggregation),
    /// Sampler aggregation (random sampling)
    Sampler(SamplerAggregation),
    /// Diversified sampler aggregation (diversified sampling)
    DiversifiedSampler(DiversifiedSamplerAggregation),
    /// Value count aggregation (counts number of values)
    ValueCount(ValueCountAggregation),
    /// Average aggregation (mean value)
    Avg(AverageAggregation),
    /// Sum aggregation (sum of values)
    Sum(SumAggregation),
    /// Min aggregation (minimum value)
    Min(MinAggregation),
    /// Max aggregation (maximum value)
    Max(MaxAggregation),
    /// Geohash grid aggregation (geo grid cells)
    GeohashGrid(GeohashGridAggregation),
    /// Geo bounds aggregation (bounding box)
    GeoBounds(GeoBoundsAggregation),
    /// Geo distance aggregation (distance-based buckets)
    GeoDistance(GeoDistanceAggregation),
    /// Extended stats aggregation (variance, std deviation, sum of squares)
    ExtendedStats(ExtendedStatsAggregation),
    /// Median absolute deviation aggregation (MAD)
    MedianAbsoluteDeviation(MedianAbsoluteDeviationAggregation),
    /// Weighted average aggregation (value and weight fields)
    WeightedAverage(WeightedAverageAggregation),
    /// String stats aggregation (character count, min/max/avg length)
    StringStats(StringStatsAggregation),
    /// Boxplot aggregation (quartiles, IQR, whiskers)
    Boxplot(BoxplotAggregation),
    /// Rate aggregation (time-based rate calculation)
    Rate(RateAggregation),
    /// T-Test aggregation (statistical t-test for A/B testing)
    TTest(TTestAggregation),
    /// Top Hits aggregation (document retrieval within buckets)
    TopHits(TopHitsAggregation),
    /// Scripted Metric aggregation (custom script execution)
    ScriptedMetric(ScriptedMetricAggregation),
    /// Bucket Script aggregation (script execution per bucket)
    BucketScript(BucketScriptAggregation),
    /// Bucket Selector aggregation (filter buckets by condition)
    BucketSelector(BucketSelectorAggregation),
    /// Bucket Sort aggregation (sort buckets by metric)
    BucketSort(BucketSortAggregation),
    /// Cumulative Sum aggregation (cumulative sum of metric values)
    CumulativeSum(CumulativeSumAggregation),
    /// Cumulative Cardinality aggregation (cumulative unique count)
    CumulativeCardinality(CumulativeCardinalityAggregation),
    /// Derivative aggregation (rate of change)
    Derivative(DerivativeAggregation),
    /// Moving Average aggregation (smoothing)
    MovingAverage(MovingAverageAggregation),
    /// Moving Function aggregation (custom window function)
    MovingFunction(MovingFunctionAggregation),
    /// Serial Differencing aggregation (lag-based differencing)
    SerialDifferencing(SerialDifferencingAggregation),
    /// Normalize aggregation (value normalization)
    Normalize(NormalizeAggregation),
    /// Children aggregation (child documents)
    Children(ChildrenAggregation),
    /// Parent aggregation (parent documents)
    Parent(ParentAggregation),
}

impl AggregationSpec {
    /// Get aggregation name
    pub fn name(&self) -> &str {
        match self {
            AggregationSpec::Terms(agg) => agg.name(),
            AggregationSpec::SignificantTerms(agg) => agg.name(),
            AggregationSpec::Stats(agg) => agg.name(),
            AggregationSpec::Histogram(agg) => agg.name(),
            AggregationSpec::DateHistogram(agg) => agg.name(),
            AggregationSpec::DateRange(agg) => agg.name(),
            AggregationSpec::IpRange(agg) => agg.name(),
            AggregationSpec::Percentile(agg) => agg.name(),
            AggregationSpec::Cardinality(agg) => agg.name(),
            AggregationSpec::Nested(agg) => agg.name(),
            AggregationSpec::ReverseNested(agg) => agg.name(),
            AggregationSpec::Pipeline(agg) => agg.name(),
            AggregationSpec::Range(agg) => agg.name(),
            AggregationSpec::Filters(agg) => agg.name(),
            AggregationSpec::Missing(agg) => agg.name(),
            AggregationSpec::Global(agg) => agg.name(),
            AggregationSpec::Composite(agg) => agg.name(),
            AggregationSpec::Sampler(agg) => agg.name(),
            AggregationSpec::DiversifiedSampler(agg) => agg.name(),
            AggregationSpec::ValueCount(agg) => agg.name(),
            AggregationSpec::Avg(agg) => agg.name(),
            AggregationSpec::Sum(agg) => agg.name(),
            AggregationSpec::Min(agg) => agg.name(),
            AggregationSpec::Max(agg) => agg.name(),
            AggregationSpec::GeohashGrid(agg) => agg.name(),
            AggregationSpec::GeoBounds(agg) => agg.name(),
            AggregationSpec::GeoDistance(agg) => agg.name(),
            AggregationSpec::ExtendedStats(agg) => agg.name(),
            AggregationSpec::MedianAbsoluteDeviation(agg) => agg.name(),
            AggregationSpec::WeightedAverage(agg) => agg.name(),
            AggregationSpec::StringStats(agg) => agg.name(),
            AggregationSpec::Boxplot(agg) => agg.name(),
            AggregationSpec::Rate(agg) => agg.name(),
            AggregationSpec::TTest(agg) => agg.name(),
            AggregationSpec::TopHits(agg) => agg.name(),
            AggregationSpec::ScriptedMetric(agg) => agg.name(),
            AggregationSpec::BucketScript(agg) => agg.name(),
            AggregationSpec::BucketSelector(agg) => agg.name(),
            AggregationSpec::BucketSort(agg) => agg.name(),
            AggregationSpec::CumulativeSum(agg) => agg.name(),
            AggregationSpec::CumulativeCardinality(agg) => agg.name(),
            AggregationSpec::Derivative(agg) => agg.name(),
            AggregationSpec::MovingAverage(agg) => agg.name(),
            AggregationSpec::MovingFunction(agg) => agg.name(),
            AggregationSpec::SerialDifferencing(agg) => agg.name(),
            AggregationSpec::Normalize(agg) => agg.name(),
            AggregationSpec::Children(agg) => agg.name(),
            AggregationSpec::Parent(agg) => agg.name(),
        }
    }
}
