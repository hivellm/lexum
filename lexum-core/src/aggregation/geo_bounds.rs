//! Geo Bounds Aggregation
//!
//! Calculates the bounding box containing all geo_point values in a field.

use super::AggregationTrait;
use super::result::AggregationResult;
use crate::error::Result;
use crate::query::GeoPoint;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Geo Bounds Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoBoundsAggregation {
    /// Field containing geo_point values
    pub field: String,
    /// Whether to wrap longitudes around the date line
    #[serde(default)]
    pub wrap_longitude: bool,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

impl GeoBoundsAggregation {
    /// Create new geo bounds aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            wrap_longitude: false,
            aggs: HashMap::new(),
        }
    }

    /// Set wrap longitude flag
    pub fn wrap_longitude(mut self, wrap: bool) -> Self {
        self.wrap_longitude = wrap;
        self
    }

    /// Add sub-aggregation
    pub fn agg(
        mut self,
        name: impl Into<String>,
        agg: crate::aggregation::AggregationSpec,
    ) -> Self {
        self.aggs.insert(name.into(), agg);
        self
    }
}

impl AggregationTrait for GeoBoundsAggregation {
    fn name(&self) -> &str {
        "geo_bounds"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Extracting geo_point values from documents
        // 2. Finding min/max lat and lon values
        // 3. Handling wrap_longitude flag for date line crossing
        // 4. Returning bounding box with top_left and bottom_right
        //
        // For now, return empty bounds as placeholder
        let bounds = GeoBoundsResult {
            top_left: GeoPoint { lat: 0.0, lon: 0.0 },
            bottom_right: GeoPoint { lat: 0.0, lon: 0.0 },
        };

        Ok(AggregationResult::GeoBounds(bounds))
    }

    fn merge(&self, results: &[AggregationResult]) -> crate::error::Result<AggregationResult> {
        // Merge geo bounds by finding the overall bounding box
        let mut min_lat = f64::MAX;
        let mut max_lat = f64::MIN;
        let mut min_lon = f64::MAX;
        let mut max_lon = f64::MIN;

        for result in results {
            if let AggregationResult::GeoBounds(bounds) = result {
                min_lat = min_lat
                    .min(bounds.top_left.lat)
                    .min(bounds.bottom_right.lat);
                max_lat = max_lat
                    .max(bounds.top_left.lat)
                    .max(bounds.bottom_right.lat);
                min_lon = min_lon
                    .min(bounds.top_left.lon)
                    .min(bounds.bottom_right.lon);
                max_lon = max_lon
                    .max(bounds.top_left.lon)
                    .max(bounds.bottom_right.lon);
            }
        }

        let bounds = GeoBoundsResult {
            top_left: GeoPoint {
                lat: max_lat,
                lon: min_lon,
            },
            bottom_right: GeoPoint {
                lat: min_lat,
                lon: max_lon,
            },
        };

        Ok(AggregationResult::GeoBounds(bounds))
    }
}

/// Geo bounds result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoBoundsResult {
    /// Top-left corner of bounding box
    pub top_left: GeoPoint,
    /// Bottom-right corner of bounding box
    pub bottom_right: GeoPoint,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_bounds_aggregation() {
        let agg = GeoBoundsAggregation::new("location");

        assert_eq!(agg.field, "location");
        assert!(!agg.wrap_longitude);
    }

    #[test]
    fn test_geo_bounds_aggregation_with_wrap() {
        let agg = GeoBoundsAggregation::new("location").wrap_longitude(true);

        assert!(agg.wrap_longitude);
    }

    #[test]
    fn test_geo_bounds_aggregation_serialization() {
        let agg = GeoBoundsAggregation::new("location").wrap_longitude(true);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("location"));
        assert!(json.contains("wrap_longitude"));

        let deserialized: GeoBoundsAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert!(deserialized.wrap_longitude);
    }
}
