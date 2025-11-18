//! Geo Centroid Aggregation
//!
//! Calculates the geographic centroid (center point) of geo_point values.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Geo Centroid Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoCentroidAggregation {
    /// Field containing geo_point values
    pub field: String,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

/// Geo Centroid Result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoCentroidResult {
    /// Latitude of the centroid
    pub lat: f64,
    /// Longitude of the centroid
    pub lon: f64,
    /// Number of geo_point values used in calculation
    pub count: usize,
}

impl GeoCentroidAggregation {
    /// Create new geo centroid aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            aggs: HashMap::new(),
        }
    }

    /// Add sub-aggregation
    pub fn sub_aggregation(
        mut self,
        name: impl Into<String>,
        agg: crate::aggregation::AggregationSpec,
    ) -> Self {
        self.aggs.insert(name.into(), agg);
        self
    }
}

impl AggregationTrait for GeoCentroidAggregation {
    fn name(&self) -> &str {
        "geo_centroid"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Geo Centroid Aggregation requires geo field support in Tantivy
        // This is a placeholder implementation
        // Full implementation would:
        // 1. Extract geo_point values from hits
        // 2. Calculate weighted centroid (average of lat/lon)
        // 3. Return centroid coordinates

        // For now, return placeholder result
        let result = GeoCentroidResult {
            lat: 0.0,
            lon: 0.0,
            count: 0,
        };
        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge geo centroid results by calculating weighted average
        // In a full implementation, this would:
        // 1. Extract centroids from each shard result
        // 2. Calculate weighted average based on counts
        // 3. Return merged centroid

        let mut total_lat = 0.0;
        let mut total_lon = 0.0;
        let mut total_count = 0;

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(centroid_result) =
                    serde_json::from_value::<GeoCentroidResult>(metric_result.value.clone())
                {
                    total_lat += centroid_result.lat * centroid_result.count as f64;
                    total_lon += centroid_result.lon * centroid_result.count as f64;
                    total_count += centroid_result.count;
                }
            }
        }

        let merged_result = if total_count > 0 {
            GeoCentroidResult {
                lat: total_lat / total_count as f64,
                lon: total_lon / total_count as f64,
                count: total_count,
            }
        } else {
            GeoCentroidResult {
                lat: 0.0,
                lon: 0.0,
                count: 0,
            }
        };

        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(merged_result)?,
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_centroid_aggregation() {
        let agg = GeoCentroidAggregation::new("location");

        assert_eq!(agg.field, "location");
    }

    #[test]
    fn test_geo_centroid_aggregation_serialization() {
        let agg = GeoCentroidAggregation::new("location");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("location"));

        let deserialized: GeoCentroidAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
    }

    #[test]
    fn test_geo_centroid_result_serialization() {
        let result = GeoCentroidResult {
            lat: 40.7128,
            lon: -74.0060,
            count: 10,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("lat"));
        assert!(json.contains("lon"));
        assert!(json.contains("count"));

        let deserialized: GeoCentroidResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.lat, 40.7128);
        assert_eq!(deserialized.lon, -74.0060);
        assert_eq!(deserialized.count, 10);
    }

    #[test]
    fn test_geo_centroid_merge() {
        let agg = GeoCentroidAggregation::new("location");

        let result1 = GeoCentroidResult {
            lat: 40.0,
            lon: -74.0,
            count: 5,
        };
        let result2 = GeoCentroidResult {
            lat: 41.0,
            lon: -75.0,
            count: 5,
        };

        let results = vec![
            AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::to_value(result1).unwrap(),
            )),
            AggregationResult::Metric(MetricAggregationResult::new(
                serde_json::to_value(result2).unwrap(),
            )),
        ];

        let merged = agg.merge(&results).unwrap();
        if let AggregationResult::Metric(metric_result) = merged {
            let centroid: GeoCentroidResult = serde_json::from_value(metric_result.value).unwrap();
            assert_eq!(centroid.lat, 40.5); // (40*5 + 41*5) / 10
            assert_eq!(centroid.lon, -74.5); // (-74*5 + -75*5) / 10
            assert_eq!(centroid.count, 10);
        } else {
            panic!("Expected Metric result");
        }
    }
}
