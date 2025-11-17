//! Geo Line Aggregation
//!
//! Creates a LineString from geo_point values ordered by a sort field.

use super::AggregationTrait;
use super::result::{AggregationResult, MetricAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::{SearchHit, SortOption, SortOrder};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Geo Line Aggregation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoLineAggregation {
    /// Field containing geo_point values
    pub field: String,
    /// Sort field to order points (default: "_key")
    #[serde(default = "default_sort_field")]
    pub sort_field: String,
    /// Sort order (default: "asc")
    #[serde(default)]
    pub sort_order: SortOrder,
    /// Include sort values in response (default: false)
    #[serde(default)]
    pub include_sort: bool,
    /// Size limit for number of points (default: 10000)
    #[serde(default = "default_size")]
    pub size: usize,
    /// Sub-aggregations
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub aggs: HashMap<String, crate::aggregation::AggregationSpec>,
}

fn default_sort_field() -> String {
    "_key".to_string()
}

fn default_size() -> usize {
    10000
}

/// Geo Line Result
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoLineResult {
    /// LineString coordinates [[lon, lat], ...]
    pub geometry: Vec<[f64; 2]>,
    /// Sort values for each point (if include_sort is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<JsonValue>>,
    /// Number of points in the line
    pub count: usize,
}

impl GeoLineAggregation {
    /// Create new geo line aggregation
    pub fn new(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            sort_field: "_key".to_string(),
            sort_order: SortOrder::Asc,
            include_sort: false,
            size: 10000,
            aggs: HashMap::new(),
        }
    }

    /// Set sort field
    pub fn sort_field(mut self, sort_field: impl Into<String>) -> Self {
        self.sort_field = sort_field.into();
        self
    }

    /// Set sort order
    pub fn sort_order(mut self, sort_order: SortOrder) -> Self {
        self.sort_order = sort_order;
        self
    }

    /// Set include sort values
    pub fn include_sort(mut self, include_sort: bool) -> Self {
        self.include_sort = include_sort;
        self
    }

    /// Set size limit
    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
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

impl AggregationTrait for GeoLineAggregation {
    fn name(&self) -> &str {
        "geo_line"
    }

    fn execute(&self, _hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Note: Geo Line Aggregation requires geo field support in Tantivy
        // This is a placeholder implementation
        // Full implementation would:
        // 1. Extract geo_point values from hits
        // 2. Sort points by sort_field
        // 3. Create LineString coordinates
        // 4. Return LineString geometry

        // For now, return placeholder result
        let result = GeoLineResult {
            geometry: vec![],
            sort: if self.include_sort {
                Some(vec![])
            } else {
                None
            },
            count: 0,
        };
        Ok(AggregationResult::Metric(MetricAggregationResult::new(
            serde_json::to_value(result)?,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        // Merge geo line results by combining and re-sorting points
        // In a full implementation, this would:
        // 1. Extract LineString coordinates from each shard result
        // 2. Combine all points
        // 3. Re-sort by sort_field
        // 4. Return merged LineString

        let mut all_points: Vec<([f64; 2], Option<JsonValue>)> = Vec::new();

        for result in results {
            if let AggregationResult::Metric(metric_result) = result {
                if let Ok(line_result) =
                    serde_json::from_value::<GeoLineResult>(metric_result.value.clone())
                {
                    for (idx, point) in line_result.geometry.iter().enumerate() {
                        let sort_value = if self.include_sort {
                            line_result.sort.as_ref().and_then(|s| s.get(idx).cloned())
                        } else {
                            None
                        };
                        all_points.push((*point, sort_value));
                    }
                }
            }
        }

        // Sort points (simplified - full implementation would sort by sort_field)
        // For now, just combine them
        let mut geometry = Vec::new();
        let mut sort_values = if self.include_sort {
            Some(Vec::new())
        } else {
            None
        };

        for (point, sort_value) in all_points {
            geometry.push(point);
            if let Some(ref mut sort) = sort_values {
                sort.push(sort_value.unwrap_or(JsonValue::Null));
            }
        }

        let merged_result = GeoLineResult {
            geometry,
            sort: sort_values,
            count: all_points.len(),
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
    fn test_geo_line_aggregation() {
        let agg = GeoLineAggregation::new("location");

        assert_eq!(agg.field, "location");
        assert_eq!(agg.sort_field, "_key");
        assert_eq!(agg.sort_order, SortOrder::Asc);
        assert_eq!(agg.size, 10000);
    }

    #[test]
    fn test_geo_line_aggregation_with_sort() {
        let agg = GeoLineAggregation::new("location")
            .sort_field("timestamp")
            .sort_order(SortOrder::Desc)
            .include_sort(true);

        assert_eq!(agg.sort_field, "timestamp");
        assert_eq!(agg.sort_order, SortOrder::Desc);
        assert!(agg.include_sort);
    }

    #[test]
    fn test_geo_line_aggregation_with_size() {
        let agg = GeoLineAggregation::new("location").size(5000);

        assert_eq!(agg.size, 5000);
    }

    #[test]
    fn test_geo_line_aggregation_serialization() {
        let agg = GeoLineAggregation::new("location")
            .sort_field("timestamp")
            .sort_order(SortOrder::Desc)
            .include_sort(true)
            .size(5000);

        let json = serde_json::to_string(&agg).unwrap();
        assert!(json.contains("field"));
        assert!(json.contains("sort_field"));
        assert!(json.contains("sort_order"));
        assert!(json.contains("include_sort"));
        assert!(json.contains("size"));

        let deserialized: GeoLineAggregation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.field, "location");
        assert_eq!(deserialized.sort_field, "timestamp");
        assert_eq!(deserialized.sort_order, SortOrder::Desc);
        assert!(deserialized.include_sort);
        assert_eq!(deserialized.size, 5000);
    }

    #[test]
    fn test_geo_line_result_serialization() {
        let result = GeoLineResult {
            geometry: vec![[-74.0060, 40.7128], [-73.9352, 40.7589]],
            sort: Some(vec![
                JsonValue::Number(serde_json::Number::from(1)),
                JsonValue::Number(serde_json::Number::from(2)),
            ]),
            count: 2,
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("geometry"));
        assert!(json.contains("sort"));
        assert!(json.contains("count"));

        let deserialized: GeoLineResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.geometry.len(), 2);
        assert_eq!(deserialized.count, 2);
    }
}
