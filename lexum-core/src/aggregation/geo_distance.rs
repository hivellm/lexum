//! Geo Distance Aggregation
//!
//! Groups documents into distance-based buckets from a central point.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::query::GeoPoint;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Geo Distance Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoDistanceAggregation {
    /// Field containing geo_point values
    pub field: String,
    /// Origin point for distance calculation
    pub origin: GeoPoint,
    /// Distance ranges (e.g., ["0-100km", "100-500km"])
    pub ranges: Vec<DistanceRange>,
    /// Distance unit (km, mi, m, yd, ft, in, nmi)
    #[serde(default = "default_unit")]
    pub unit: String,
    /// Distance calculation method (arc, plane)
    #[serde(default = "default_distance_type")]
    pub distance_type: String,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_unit() -> String {
    "km".to_string()
}

fn default_distance_type() -> String {
    "arc".to_string()
}

/// Distance range specification
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum DistanceRange {
    /// Range with key (e.g., "0-100km")
    Keyed {
        /// Range key/name
        key: String,
        /// From distance (optional, 0 if not specified)
        from: Option<f64>,
        /// To distance (optional, infinity if not specified)
        to: Option<f64>,
    },
    /// Simple range string (e.g., "0-100km")
    String(String),
}

impl GeoDistanceAggregation {
    /// Create new geo distance aggregation
    pub fn new(field: impl Into<String>, origin: GeoPoint) -> Self {
        Self {
            field: field.into(),
            origin,
            ranges: Vec::new(),
            unit: "km".to_string(),
            distance_type: "arc".to_string(),
            aggs: HashMap::new(),
        }
    }

    /// Add distance range
    pub fn range(mut self, range: DistanceRange) -> Self {
        self.ranges.push(range);
        self
    }

    /// Add multiple distance ranges
    pub fn ranges(mut self, ranges: Vec<DistanceRange>) -> Self {
        self.ranges = ranges;
        self
    }

    /// Set distance unit
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Set distance calculation type
    pub fn distance_type(mut self, distance_type: impl Into<String>) -> Self {
        self.distance_type = distance_type.into();
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

impl AggregationTrait for GeoDistanceAggregation {
    fn name(&self) -> &str {
        "geo_distance"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Full implementation would require:
        // 1. Extracting geo_point values from documents
        // 2. Calculating distance from origin using Haversine formula (arc) or simple plane distance
        // 3. Converting distance units (km, mi, m, etc.)
        // 4. Grouping documents into distance range buckets
        // 5. Executing sub-aggregations for each bucket
        //
        // For now, return empty buckets as placeholder
        let buckets: Vec<Bucket> = Vec::new();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> crate::error::Result<AggregationResult> {
        // Merge geo distance results by combining buckets from all shards
        let mut merged_buckets: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    let key = bucket.key.to_string();
                    *merged_buckets.entry(key).or_insert(0) += bucket.doc_count;
                }
            }
        }

        let buckets: Vec<Bucket> = merged_buckets
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            buckets,
        )))
    }
}

/// Geo distance bucket result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoDistanceBucket {
    /// Range key (if keyed) or auto-generated key
    pub key: String,
    /// From distance
    pub from: Option<f64>,
    /// To distance
    pub to: Option<f64>,
    /// Number of documents in this range
    pub doc_count: u64,
    /// Sub-aggregation results
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::result::AggregationResult>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_geo_distance_aggregation() {
        let origin = GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
        };
        let agg = GeoDistanceAggregation::new("location", origin);

        assert_eq!(agg.field, "location");
        assert_eq!(agg.origin.lat, 40.7128);
        assert_eq!(agg.origin.lon, -74.0060);
        assert_eq!(agg.unit, "km");
        assert_eq!(agg.distance_type, "arc");
    }

    #[test]
    fn test_geo_distance_aggregation_with_unit() {
        let origin = GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
        };
        let agg = GeoDistanceAggregation::new("location", origin).unit("mi");

        assert_eq!(agg.unit, "mi");
    }

    #[test]
    fn test_geo_distance_aggregation_with_ranges() {
        let origin = GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
        };
        let range1 = DistanceRange::String("0-100km".to_string());
        let range2 = DistanceRange::Keyed {
            key: "far".to_string(),
            from: Some(100.0),
            to: None,
        };
        let agg = GeoDistanceAggregation::new("location", origin)
            .range(range1)
            .range(range2);

        assert_eq!(agg.ranges.len(), 2);
    }

    #[test]
    fn test_geo_distance_aggregation_serialization() {
        let origin = GeoPoint {
            lat: 40.7128,
            lon: -74.0060,
        };
        let range = DistanceRange::String("0-100km".to_string());
        let agg = GeoDistanceAggregation::new("location", origin)
            .range(range)
            .unit("mi")
            .distance_type("plane");

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("location"));
        assert!(json.contains("origin"));
        assert!(json.contains("ranges"));
        assert!(json.contains("unit"));
        assert!(json.contains("distance_type"));

        let deserialized: GeoDistanceAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert_eq!(deserialized.unit, "mi");
        assert_eq!(deserialized.distance_type, "plane");
    }
}
