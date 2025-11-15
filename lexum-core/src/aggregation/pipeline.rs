//! Pipeline aggregation

use super::AggregationTrait;
use super::result::AggregationResult;
use crate::error::{Error, Result};
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Pipeline aggregation type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PipelineType {
    /// Moving average
    MovingAverage,
    /// Derivative
    Derivative,
    /// Cumulative sum
    CumulativeSum,
}

/// Pipeline aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PipelineAggregation {
    /// Pipeline type
    pub pipeline_type: PipelineType,
    /// Buckets path (e.g., "my_histogram>_count")
    pub buckets_path: String,
}

impl AggregationTrait for PipelineAggregation {
    fn name(&self) -> &str {
        match self.pipeline_type {
            PipelineType::MovingAverage => "moving_average",
            PipelineType::Derivative => "derivative",
            PipelineType::CumulativeSum => "cumulative_sum",
        }
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Pipeline aggregations operate on other aggregation results
        // They are computed after the parent aggregation
        Err(Error::Config(
            "Pipeline aggregations must be computed on parent aggregation results".to_string(),
        ))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Pipeline aggregations merge by applying the pipeline operation
        // This is a placeholder - full implementation would process buckets
        if let Some(first_result) = results.first() {
            Ok(first_result.clone())
        } else {
            Err(Error::Config("No results to merge".to_string()))
        }
    }
}

impl PipelineAggregation {
    /// Create new pipeline aggregation
    pub fn new(pipeline_type: PipelineType, buckets_path: impl Into<String>) -> Self {
        Self {
            pipeline_type,
            buckets_path: buckets_path.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;

    #[test]
    fn test_pipeline_aggregation_execute_error() {
        let agg = PipelineAggregation::new(PipelineType::MovingAverage, "my_histogram>_count");
        let field_cache = FieldCache::new();
        let hits: Vec<SearchHit> = vec![];

        let result = agg.execute(&hits, &field_cache);

        // Should return error since pipeline aggregations need parent results
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Pipeline aggregations"));
        }
    }

    #[test]
    fn test_pipeline_aggregation_types() {
        let moving_avg = PipelineAggregation::new(PipelineType::MovingAverage, "path");
        assert_eq!(moving_avg.name(), "moving_average");

        let derivative = PipelineAggregation::new(PipelineType::Derivative, "path");
        assert_eq!(derivative.name(), "derivative");

        let cumulative = PipelineAggregation::new(PipelineType::CumulativeSum, "path");
        assert_eq!(cumulative.name(), "cumulative_sum");
    }

    #[test]
    fn test_pipeline_aggregation_merge_empty() {
        let agg = PipelineAggregation::new(PipelineType::MovingAverage, "path");
        let results: Vec<AggregationResult> = vec![];

        let result = agg.merge(&results);

        // Should return error for empty results
        assert!(result.is_err());
    }

    #[test]
    fn test_pipeline_aggregation_merge_with_result() {
        let agg = PipelineAggregation::new(PipelineType::MovingAverage, "path");

        // Create a dummy bucket result
        let bucket_result = super::super::result::BucketAggregationResult::new(vec![]);
        let results = vec![AggregationResult::Buckets(bucket_result)];

        let result = agg.merge(&results);

        // Should succeed (returns first result as placeholder)
        assert!(result.is_ok());
    }
}
