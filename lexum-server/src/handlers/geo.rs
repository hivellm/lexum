//! Geographic point handlers

use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::error::ApiResult;
use lexum_core::schema::{GeoPoint, GeoPointFormat};

/// Geographic point validation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoPointValidationRequest {
    /// Geographic point in various formats
    pub point: GeoPointFormat,
}

/// Geographic point validation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoPointValidationResponse {
    /// Whether the point is valid
    pub valid: bool,
    /// The parsed geographic point (if valid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub point: Option<GeoPoint>,
    /// Error message (if invalid)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Distance calculation request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoDistanceRequest {
    /// First geographic point
    pub point1: GeoPointFormat,
    /// Second geographic point
    pub point2: GeoPointFormat,
}

/// Distance calculation response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoDistanceResponse {
    /// Distance in kilometers
    pub distance_km: f64,
    /// First point (parsed)
    pub point1: GeoPoint,
    /// Second point (parsed)
    pub point2: GeoPoint,
}

/// Validate geographic point
#[utoipa::path(
    post,
    path = "/api/v1/geo/validate",
    request_body = GeoPointValidationRequest,
    responses(
        (status = 200, description = "Point validation result", body = GeoPointValidationResponse),
        (status = 400, description = "Invalid request", body = crate::error::ApiError)
    ),
    tag = "Geo"
)]
pub async fn validate_geo_point(
    Json(request): Json<GeoPointValidationRequest>,
) -> ApiResult<Json<GeoPointValidationResponse>> {
    match request.point.to_geo_point() {
        Ok(point) => Ok(Json(GeoPointValidationResponse {
            valid: true,
            point: Some(point),
            error: None,
        })),
        Err(error) => Ok(Json(GeoPointValidationResponse {
            valid: false,
            point: None,
            error: Some(error),
        })),
    }
}

/// Calculate distance between two geographic points
#[utoipa::path(
    post,
    path = "/api/v1/geo/distance",
    request_body = GeoDistanceRequest,
    responses(
        (status = 200, description = "Distance calculation result", body = GeoDistanceResponse),
        (status = 400, description = "Invalid points or request", body = crate::error::ApiError)
    ),
    tag = "Geo"
)]
pub async fn calculate_distance(
    Json(request): Json<GeoDistanceRequest>,
) -> ApiResult<Json<GeoDistanceResponse>> {
    let point1 = request
        .point1
        .to_geo_point()
        .map_err(|e| crate::error::ApiError::InvalidRequest(format!("Invalid point1: {e}")))?;

    let point2 = request
        .point2
        .to_geo_point()
        .map_err(|e| crate::error::ApiError::InvalidRequest(format!("Invalid point2: {e}")))?;

    let distance_km = point1.distance_to(&point2);

    Ok(Json(GeoDistanceResponse {
        distance_km,
        point1,
        point2,
    }))
}

/// Bounding box format for serialization/deserialization
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum BoundsFormat {
    /// Array format: [min_lat, max_lat, min_lon, max_lon]
    Array([f64; 4]),
    /// Object format: {top_left: {lat, lon}, bottom_right: {lat, lon}}
    Object {
        top_left: GeoPointFormat,
        bottom_right: GeoPointFormat,
    },
}

/// Geographic bounds check request
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoBoundsCheckRequest {
    /// Point to check
    pub point: GeoPointFormat,
    /// Bounding box: [min_lat, max_lat, min_lon, max_lon] or {top_left: {lat, lon}, bottom_right: {lat, lon}}
    #[serde(with = "bounds_format")]
    pub bounds: [f64; 4],
}

mod bounds_format {
    use super::BoundsFormat;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(bounds: &[f64; 4], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        bounds.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[f64; 4], D::Error>
    where
        D: Deserializer<'de>,
    {
        let format = BoundsFormat::deserialize(deserializer)?;
        match format {
            BoundsFormat::Array(arr) => Ok(arr),
            BoundsFormat::Object {
                top_left,
                bottom_right,
            } => {
                let top_left_point = top_left.to_geo_point().map_err(serde::de::Error::custom)?;
                let bottom_right_point = bottom_right
                    .to_geo_point()
                    .map_err(serde::de::Error::custom)?;

                // Convert to [min_lat, max_lat, min_lon, max_lon]
                Ok([
                    bottom_right_point.lat.min(top_left_point.lat), // min_lat
                    bottom_right_point.lat.max(top_left_point.lat), // max_lat
                    top_left_point.lon.min(bottom_right_point.lon), // min_lon
                    top_left_point.lon.max(bottom_right_point.lon), // max_lon
                ])
            }
        }
    }
}

/// Geographic bounds check response
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GeoBoundsCheckResponse {
    /// Whether point is within bounds
    pub within_bounds: bool,
    /// The parsed point
    pub point: GeoPoint,
    /// The bounds used for checking
    pub bounds: [f64; 4],
}

/// Check if point is within geographic bounds
#[utoipa::path(
    post,
    path = "/api/v1/geo/bounds",
    request_body = GeoBoundsCheckRequest,
    responses(
        (status = 200, description = "Bounds check result", body = GeoBoundsCheckResponse),
        (status = 400, description = "Invalid point or bounds", body = crate::error::ApiError)
    ),
    tag = "Geo"
)]
pub async fn check_bounds(
    Json(request): Json<GeoBoundsCheckRequest>,
) -> ApiResult<Json<GeoBoundsCheckResponse>> {
    let point = request
        .point
        .to_geo_point()
        .map_err(|e| crate::error::ApiError::InvalidRequest(format!("Invalid point: {e}")))?;

    let [min_lat, max_lat, min_lon, max_lon] = request.bounds;
    let within_bounds = point.within_bounds(min_lat, max_lat, min_lon, max_lon);

    Ok(Json(GeoBoundsCheckResponse {
        within_bounds,
        point,
        bounds: request.bounds,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexum_core::schema::GeoPointFormat;

    #[lexum_macros::tokio_test]
    async fn test_validate_geo_point_valid() {
        let request = GeoPointValidationRequest {
            point: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
        };

        let result = validate_geo_point(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.valid);
        assert!(response.point.is_some());
        assert!(response.error.is_none());

        let point = response.point.unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);
    }

    #[lexum_macros::tokio_test]
    async fn test_validate_geo_point_invalid() {
        let request = GeoPointValidationRequest {
            point: GeoPointFormat::LatLon {
                lat: 91.0,
                lon: 0.0,
            }, // Invalid latitude
        };

        let result = validate_geo_point(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.valid);
        assert!(response.point.is_none());
        assert!(response.error.is_some());
    }

    #[lexum_macros::tokio_test]
    async fn test_calculate_distance() {
        let request = GeoDistanceRequest {
            point1: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            }, // NYC
            point2: GeoPointFormat::LatLon {
                lat: 51.5074,
                lon: -0.1278,
            }, // London
        };

        let result = calculate_distance(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        // Distance should be around 5570 km
        assert!(response.distance_km > 5500.0 && response.distance_km < 5700.0);
        assert_eq!(response.point1.lat, 40.7128);
        assert_eq!(response.point1.lon, -74.0060);
        assert_eq!(response.point2.lat, 51.5074);
        assert_eq!(response.point2.lon, -0.1278);
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_within() {
        let request = GeoBoundsCheckRequest {
            point: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
            bounds: [30.0, 50.0, -80.0, -70.0], // NYC bounds
        };

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(response.within_bounds);
        assert_eq!(response.point.lat, 40.7128);
        assert_eq!(response.point.lon, -74.0060);
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_outside() {
        let request = GeoBoundsCheckRequest {
            point: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
            bounds: [50.0, 60.0, -80.0, -70.0], // Wrong latitude range
        };

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        assert!(!response.within_bounds);
    }

    #[test]
    fn test_geo_point_validation_request() {
        let request = GeoPointValidationRequest {
            point: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
        };

        // Test serialization (basic check)
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("40.7128"));
        assert!(json.contains("-74.0060"));
    }

    #[test]
    fn test_geo_distance_request() {
        let request = GeoDistanceRequest {
            point1: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
            point2: GeoPointFormat::LatLon {
                lat: 51.5074,
                lon: -0.1278,
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("point1"));
        assert!(json.contains("point2"));
    }

    #[test]
    fn test_geo_bounds_request() {
        let request = GeoBoundsCheckRequest {
            point: GeoPointFormat::LatLon {
                lat: 40.7128,
                lon: -74.0060,
            },
            bounds: [30.0, 50.0, -80.0, -70.0],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("point"));
        assert!(json.contains("bounds"));
    }

    #[test]
    fn test_bounds_deserialization_array_format() {
        // Test deserialization of bounds in array format
        let json =
            r#"{"point": {"lat": 40.7128, "lon": -74.0060}, "bounds": [30.0, 50.0, -80.0, -70.0]}"#;
        let request: GeoBoundsCheckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(request.bounds, [30.0, 50.0, -80.0, -70.0]);
    }

    #[test]
    fn test_bounds_deserialization_object_format() {
        // Test deserialization of bounds in object format (top_left/bottom_right)
        let json = r#"{
            "point": {"lat": 40.7128, "lon": -74.0060},
            "bounds": {
                "top_left": {"lat": 50.0, "lon": -80.0},
                "bottom_right": {"lat": 30.0, "lon": -70.0}
            }
        }"#;
        let request: GeoBoundsCheckRequest = serde_json::from_str(json).unwrap();
        // Should be converted to [min_lat, max_lat, min_lon, max_lon]
        assert_eq!(request.bounds[0], 30.0); // min_lat
        assert_eq!(request.bounds[1], 50.0); // max_lat
        assert_eq!(request.bounds[2], -80.0); // min_lon
        assert_eq!(request.bounds[3], -70.0); // max_lon
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_point_on_boundary() {
        // Test point exactly on the boundary
        let request = GeoBoundsCheckRequest {
            point: GeoPointFormat::LatLon {
                lat: 30.0,
                lon: -80.0,
            }, // Exactly on min_lat, min_lon
            bounds: [30.0, 50.0, -80.0, -70.0],
        };

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        // Point on boundary should be considered within bounds
        assert!(response.within_bounds);
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_invalid_point() {
        // Test with invalid point (latitude out of range)
        let request = GeoBoundsCheckRequest {
            point: GeoPointFormat::LatLon {
                lat: 91.0,
                lon: -74.0060,
            }, // Invalid latitude
            bounds: [30.0, 50.0, -80.0, -70.0],
        };

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_err());
        match result.unwrap_err() {
            crate::error::ApiError::InvalidRequest(msg) => {
                assert!(msg.contains("Invalid point"));
            }
            _ => panic!("Expected InvalidRequest error"),
        }
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_object_format() {
        // Test bounds check with object format bounds
        let json = r#"{
            "point": {"lat": 40.7128, "lon": -74.0060},
            "bounds": {
                "top_left": {"lat": 50.0, "lon": -80.0},
                "bottom_right": {"lat": 30.0, "lon": -70.0}
            }
        }"#;
        let request: GeoBoundsCheckRequest = serde_json::from_str(json).unwrap();

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        // NYC (40.7128, -74.0060) should be within bounds [30.0, 50.0, -80.0, -70.0]
        assert!(response.within_bounds);
        assert_eq!(response.point.lat, 40.7128);
        assert_eq!(response.point.lon, -74.0060);
    }

    #[lexum_macros::tokio_test]
    async fn test_check_bounds_reversed_bounds() {
        // Test with bounds where top_left and bottom_right are reversed
        // The deserializer should handle this correctly
        let json = r#"{
            "point": {"lat": 40.7128, "lon": -74.0060},
            "bounds": {
                "top_left": {"lat": 30.0, "lon": -70.0}, // Reversed
                "bottom_right": {"lat": 50.0, "lon": -80.0} // Reversed
            }
        }"#;
        let request: GeoBoundsCheckRequest = serde_json::from_str(json).unwrap();

        let result = check_bounds(axum::Json(request)).await;
        assert!(result.is_ok());

        let response = result.unwrap().0;
        // Should still work correctly (min/max calculation handles this)
        assert!(response.within_bounds);
    }
}
