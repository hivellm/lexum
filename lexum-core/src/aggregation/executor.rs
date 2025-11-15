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
                super::AggregationSpec::Stats(stats_agg) => {
                    stats_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::Histogram(hist_agg) => {
                    hist_agg.execute(hits, &self.field_cache)?
                }
                super::AggregationSpec::DateHistogram(date_hist_agg) => {
                    date_hist_agg.execute(hits, &self.field_cache)?
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
                super::AggregationSpec::Pipeline(pipeline_agg) => {
                    pipeline_agg.execute(hits, &self.field_cache)?
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
            super::AggregationSpec::Stats(stats_agg) => stats_agg.merge(results),
            super::AggregationSpec::Histogram(hist_agg) => hist_agg.merge(results),
            super::AggregationSpec::DateHistogram(date_hist_agg) => date_hist_agg.merge(results),
            super::AggregationSpec::Percentile(percentile_agg) => percentile_agg.merge(results),
            super::AggregationSpec::Cardinality(cardinality_agg) => cardinality_agg.merge(results),
            super::AggregationSpec::Nested(nested_agg) => nested_agg.merge(results),
            super::AggregationSpec::Pipeline(pipeline_agg) => pipeline_agg.merge(results),
        }
    }
}
