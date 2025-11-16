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

pub mod cardinality;
pub mod composite;
pub mod date_histogram;
pub mod date_range;
pub mod executor;
pub mod filters;
pub mod global;
pub mod histogram;
pub mod missing;
pub mod nested;
pub mod percentile;
pub mod pipeline;
pub mod range;
pub mod result;
pub mod stats;
pub mod terms;

pub use cardinality::CardinalityAggregation;
pub use composite::CompositeAggregation;
pub use date_histogram::DateHistogramAggregation;
pub use date_range::DateRangeAggregation;
pub use executor::AggregationExecutor;
pub use filters::FiltersAggregation;
pub use global::GlobalAggregation;
pub use histogram::HistogramAggregation;
pub use missing::MissingAggregation;
pub use nested::NestedAggregation;
pub use percentile::PercentileAggregation;
pub use pipeline::PipelineAggregation;
pub use range::RangeAggregation;
pub use result::{
    AggregationResult, Bucket, BucketAggregationResult, MetricAggregationResult,
    SingleBucketAggregationResult,
};
pub use stats::StatsAggregation;
pub use terms::TermsAggregation;

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
    /// Stats aggregation (min, max, avg, sum, count)
    Stats(StatsAggregation),
    /// Histogram aggregation (numeric buckets)
    Histogram(HistogramAggregation),
    /// Date histogram aggregation (time-based buckets)
    DateHistogram(DateHistogramAggregation),
    /// Date range aggregation (date ranges)
    DateRange(DateRangeAggregation),
    /// Percentile aggregation
    Percentile(PercentileAggregation),
    /// Cardinality aggregation (unique count)
    Cardinality(CardinalityAggregation),
    /// Nested aggregation (sub-aggregations)
    Nested(NestedAggregation),
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
}

impl AggregationSpec {
    /// Get aggregation name
    pub fn name(&self) -> &str {
        match self {
            AggregationSpec::Terms(agg) => agg.name(),
            AggregationSpec::Stats(agg) => agg.name(),
            AggregationSpec::Histogram(agg) => agg.name(),
            AggregationSpec::DateHistogram(agg) => agg.name(),
            AggregationSpec::DateRange(agg) => agg.name(),
            AggregationSpec::Percentile(agg) => agg.name(),
            AggregationSpec::Cardinality(agg) => agg.name(),
            AggregationSpec::Nested(agg) => agg.name(),
            AggregationSpec::Pipeline(agg) => agg.name(),
            AggregationSpec::Range(agg) => agg.name(),
            AggregationSpec::Filters(agg) => agg.name(),
            AggregationSpec::Missing(agg) => agg.name(),
            AggregationSpec::Global(agg) => agg.name(),
            AggregationSpec::Composite(agg) => agg.name(),
        }
    }
}
