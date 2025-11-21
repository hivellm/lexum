//! Field type definitions

use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// Field types supported by Lexum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    /// Full-text searchable text
    Text,
    /// Exact-match keyword (not analyzed)
    Keyword,
    /// 64-bit signed integer
    I64,
    /// 64-bit floating point
    F64,
    /// Date/timestamp
    Date,
    /// Boolean value
    Boolean,
    /// Geographic point (latitude, longitude)
    GeoPoint,
    /// IP address (IPv4 or IPv6)
    IpAddress,
}

/// Field configuration options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldConfig {
    /// Field name
    pub name: String,

    /// Field type
    #[serde(rename = "type")]
    pub field_type: FieldType,

    /// Whether field is stored (retrievable)
    #[serde(default = "default_true")]
    pub stored: bool,

    /// Whether field is indexed (searchable)
    #[serde(default = "default_true")]
    pub indexed: bool,

    /// Whether field has fast field (for sorting/aggregations)
    #[serde(default)]
    pub fast: bool,
}

fn default_true() -> bool {
    true
}

impl FieldConfig {
    /// Create new field configuration
    pub fn new(name: impl Into<String>, field_type: FieldType) -> Self {
        Self {
            name: name.into(),
            field_type,
            stored: true,
            indexed: true,
            fast: false,
        }
    }

    /// Set stored flag
    pub fn stored(mut self, stored: bool) -> Self {
        self.stored = stored;
        self
    }

    /// Set indexed flag
    pub fn indexed(mut self, indexed: bool) -> Self {
        self.indexed = indexed;
        self
    }

    /// Set fast field flag
    pub fn fast(mut self, fast: bool) -> Self {
        self.fast = fast;
        self
    }
}

impl FieldType {
    /// Validate if a string value is valid for this field type
    pub fn validate_value(&self, value: &str) -> Result<(), String> {
        match self {
            FieldType::IpAddress => value
                .parse::<IpAddr>()
                .map(|_| ())
                .map_err(|e| format!("Invalid IP address '{value}': {e}")),
            _ => Ok(()), // Other types don't have string validation yet
        }
    }

    /// Check if a string looks like an IP address
    pub fn is_ip_address(value: &str) -> bool {
        value.parse::<IpAddr>().is_ok()
    }
}

/// Geographic point representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct GeoPoint {
    /// Latitude in degrees (-90.0 to 90.0)
    pub lat: f64,
    /// Longitude in degrees (-180.0 to 180.0)
    pub lon: f64,
}

impl GeoPoint {
    /// Create a new geographic point
    pub fn new(lat: f64, lon: f64) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(format!(
                "Latitude must be between -90.0 and 90.0, got {lat}",
            ));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(format!(
                "Longitude must be between -180.0 and 180.0, got {lon}",
            ));
        }
        Ok(Self { lat, lon })
    }

    /// Calculate distance to another point using Haversine formula (approximate)
    pub fn distance_to(&self, other: &GeoPoint) -> f64 {
        let lat1_rad = self.lat.to_radians();
        let lat2_rad = other.lat.to_radians();
        let delta_lat = (other.lat - self.lat).to_radians();
        let delta_lon = (other.lon - self.lon).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        // Earth radius in kilometers
        6371.0 * c
    }

    /// Check if point is within bounding box
    pub fn within_bounds(&self, min_lat: f64, max_lat: f64, min_lon: f64, max_lon: f64) -> bool {
        self.lat >= min_lat && self.lat <= max_lat && self.lon >= min_lon && self.lon <= max_lon
    }
}

/// Different formats for representing geo points
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum GeoPointFormat {
    /// Latitude/Longitude object
    /// Latitude and longitude coordinates
    LatLon {
        /// Latitude (-90.0 to 90.0)
        lat: f64,
        /// Longitude (-180.0 to 180.0)
        lon: f64,
    },
    /// Longitude/Latitude array [lon, lat]
    LonLatArray([f64; 2]),
    /// Latitude/Longitude array [lat, lon]
    LatLonArray([f64; 2]),
    /// Geohash string
    Geohash(String),
    /// Well-known text point format
    WktPoint(String),
}

impl GeoPointFormat {
    /// Convert to GeoPoint
    pub fn to_geo_point(&self) -> Result<GeoPoint, String> {
        match self {
            GeoPointFormat::LatLon { lat, lon } => GeoPoint::new(*lat, *lon),
            GeoPointFormat::LonLatArray([lon, lat]) => GeoPoint::new(*lat, *lon),
            GeoPointFormat::LatLonArray([lat, lon]) => GeoPoint::new(*lat, *lon),
            GeoPointFormat::Geohash(hash) => {
                // Basic geohash decoding (simplified)
                // In a real implementation, you'd use a proper geohash library
                Err(format!("Geohash decoding not implemented yet: {hash}"))
            }
            GeoPointFormat::WktPoint(wkt) => {
                // Parse WKT POINT(lon lat) format
                if wkt.starts_with("POINT(") && wkt.ends_with(")") {
                    let coords: String = wkt.chars().skip(6).take(wkt.len() - 7).collect();
                    let parts: Vec<&str> = coords.split_whitespace().collect();
                    if parts.len() == 2 {
                        let lon: f64 = parts[0].parse().map_err(|_| "Invalid longitude")?;
                        let lat: f64 = parts[1].parse().map_err(|_| "Invalid latitude")?;
                        GeoPoint::new(lat, lon)
                    } else {
                        Err("Invalid WKT POINT format".to_string())
                    }
                } else {
                    Err("Invalid WKT format".to_string())
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_config_builder() {
        let config = FieldConfig::new("title", FieldType::Text)
            .stored(true)
            .indexed(true)
            .fast(false);

        assert_eq!(config.name, "title");
        assert_eq!(config.field_type, FieldType::Text);
        assert!(config.stored);
        assert!(config.indexed);
        assert!(!config.fast);
    }

    #[test]
    fn test_field_config_defaults() {
        let config = FieldConfig::new("title", FieldType::Text);
        assert!(config.stored);
        assert!(config.indexed);
        assert!(!config.fast);
    }

    #[test]
    fn test_field_config_chaining() {
        let config = FieldConfig::new("title", FieldType::Text)
            .stored(false)
            .indexed(false)
            .fast(true);

        assert!(!config.stored);
        assert!(!config.indexed);
        assert!(config.fast);
    }

    #[test]
    fn test_all_field_types() {
        let field_types = vec![
            FieldType::Text,
            FieldType::Keyword,
            FieldType::I64,
            FieldType::F64,
            FieldType::Date,
            FieldType::Boolean,
            FieldType::GeoPoint,
            FieldType::IpAddress,
        ];

        for field_type in field_types {
            let config = FieldConfig::new("test_field", field_type.clone());
            assert_eq!(config.field_type, field_type);
        }
    }

    #[test]
    fn test_field_type_serialization() {
        let field_type = FieldType::Text;
        let json = serde_json::to_string(&field_type).unwrap();
        assert_eq!(json, "\"text\"");

        let deserialized: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FieldType::Text);
    }

    #[test]
    fn test_all_field_types_serialization() {
        let test_cases = vec![
            (FieldType::Text, "\"text\""),
            (FieldType::Keyword, "\"keyword\""),
            (FieldType::I64, "\"i64\""),
            (FieldType::F64, "\"f64\""),
            (FieldType::Date, "\"date\""),
            (FieldType::Boolean, "\"boolean\""),
            (FieldType::GeoPoint, "\"geopoint\""),
            (FieldType::IpAddress, "\"ipaddress\""),
        ];

        for (field_type, expected_json) in test_cases {
            let json = serde_json::to_string(&field_type).unwrap();
            assert_eq!(json, expected_json);

            let deserialized: FieldType = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, field_type);
        }
    }

    #[test]
    fn test_ip_address_field_type() {
        let field_type = FieldType::IpAddress;
        let json = serde_json::to_string(&field_type).unwrap();
        assert_eq!(json, "\"ipaddress\"");

        let deserialized: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, field_type);
    }

    #[test]
    fn test_ip_address_validation() {
        // Valid IPv4
        assert!(FieldType::IpAddress.validate_value("192.168.1.1").is_ok());
        assert!(FieldType::IpAddress.validate_value("127.0.0.1").is_ok());
        assert!(FieldType::IpAddress.validate_value("0.0.0.0").is_ok());
        assert!(
            FieldType::IpAddress
                .validate_value("255.255.255.255")
                .is_ok()
        );
        assert!(FieldType::IpAddress.validate_value("10.0.0.1").is_ok());

        // Valid IPv6
        assert!(FieldType::IpAddress.validate_value("::1").is_ok());
        assert!(FieldType::IpAddress.validate_value("2001:db8::1").is_ok());
        assert!(FieldType::IpAddress.validate_value("fe80::1").is_ok());
        assert!(
            FieldType::IpAddress
                .validate_value("2001:0db8:85a3:0000:0000:8a2e:0370:7334")
                .is_ok()
        );
        assert!(
            FieldType::IpAddress
                .validate_value("2001:db8:85a3::8a2e:370:7334")
                .is_ok()
        );

        // Invalid IP addresses
        assert!(FieldType::IpAddress.validate_value("not.an.ip").is_err());
        assert!(
            FieldType::IpAddress
                .validate_value("999.999.999.999")
                .is_err()
        );
        assert!(FieldType::IpAddress.validate_value("256.1.1.1").is_err());
        assert!(FieldType::IpAddress.validate_value("192.168.1").is_err());
        assert!(
            FieldType::IpAddress
                .validate_value("192.168.1.1.1")
                .is_err()
        );
        assert!(FieldType::IpAddress.validate_value("").is_err());
        // Note: "::" is technically a valid IPv6 address (unspecified address)
        // but we may want to reject it in some contexts
        // For now, we accept it as valid
    }

    #[test]
    fn test_is_ip_address() {
        // Valid IPv4
        assert!(FieldType::is_ip_address("192.168.1.1"));
        assert!(FieldType::is_ip_address("127.0.0.1"));

        // Valid IPv6
        assert!(FieldType::is_ip_address("::1"));
        assert!(FieldType::is_ip_address("2001:db8::1"));

        // Invalid IP addresses
        assert!(!FieldType::is_ip_address("not.an.ip"));
        assert!(!FieldType::is_ip_address("999.999.999.999"));
    }

    #[test]
    fn test_ip_address_field_config() {
        let config = FieldConfig::new("ip", FieldType::IpAddress)
            .stored(true)
            .indexed(true);

        assert_eq!(config.name, "ip");
        assert_eq!(config.field_type, FieldType::IpAddress);
        assert!(config.stored);
        assert!(config.indexed);
    }

    #[test]
    fn test_field_config_serialization() {
        let config = FieldConfig::new("title", FieldType::Text)
            .stored(true)
            .indexed(true)
            .fast(false);

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("text"));
        assert!(json.contains("stored"));
        assert!(json.contains("indexed"));
        assert!(json.contains("fast"));

        let deserialized: FieldConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "title");
        assert_eq!(deserialized.field_type, FieldType::Text);
        assert!(deserialized.stored);
        assert!(deserialized.indexed);
        assert!(!deserialized.fast);
    }

    #[test]
    fn test_field_config_deserialization_with_defaults() {
        // Test that defaults are applied when fields are missing
        let json = r#"{"name": "title", "type": "text"}"#;
        let config: FieldConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "title");
        assert_eq!(config.field_type, FieldType::Text);
        assert!(config.stored); // Should default to true
        assert!(config.indexed); // Should default to true
        assert!(!config.fast); // Should default to false
    }

    #[test]
    fn test_field_type_equality() {
        assert_eq!(FieldType::Text, FieldType::Text);
        assert_eq!(FieldType::Keyword, FieldType::Keyword);
        assert_ne!(FieldType::Text, FieldType::Keyword);
    }

    #[test]
    fn test_geo_point_creation() {
        // Valid coordinates
        let point = GeoPoint::new(40.7128, -74.0060).unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);

        // Invalid latitude
        assert!(GeoPoint::new(91.0, 0.0).is_err());
        assert!(GeoPoint::new(-91.0, 0.0).is_err());

        // Invalid longitude
        assert!(GeoPoint::new(0.0, 181.0).is_err());
        assert!(GeoPoint::new(0.0, -181.0).is_err());
    }

    #[test]
    fn test_geo_point_distance() {
        let nyc = GeoPoint::new(40.7128, -74.0060).unwrap();
        let london = GeoPoint::new(51.5074, -0.1278).unwrap();

        let distance = nyc.distance_to(&london);
        // Approximate distance between NYC and London
        assert!(distance > 5500.0 && distance < 5700.0); // Should be around 5570 km
    }

    #[test]
    fn test_geo_point_bounds() {
        let point = GeoPoint::new(40.7128, -74.0060).unwrap();

        // Within bounds
        assert!(point.within_bounds(30.0, 50.0, -80.0, -70.0));

        // Outside bounds
        assert!(!point.within_bounds(50.0, 60.0, -80.0, -70.0)); // Wrong latitude range
        assert!(!point.within_bounds(30.0, 50.0, -60.0, -50.0)); // Wrong longitude range
    }

    #[test]
    fn test_geo_point_format_conversion() {
        // LatLon object
        let format = GeoPointFormat::LatLon {
            lat: 40.7128,
            lon: -74.0060,
        };
        let point = format.to_geo_point().unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);

        // LonLat array
        let format = GeoPointFormat::LonLatArray([-74.0060, 40.7128]);
        let point = format.to_geo_point().unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);

        // LatLon array
        let format = GeoPointFormat::LatLonArray([40.7128, -74.0060]);
        let point = format.to_geo_point().unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);

        // WKT Point
        let format = GeoPointFormat::WktPoint("POINT(-74.0060 40.7128)".to_string());
        let point = format.to_geo_point().unwrap();
        assert_eq!(point.lat, 40.7128);
        assert_eq!(point.lon, -74.0060);

        // Invalid WKT
        let format = GeoPointFormat::WktPoint("INVALID".to_string());
        assert!(format.to_geo_point().is_err());

        // Geohash (not implemented yet)
        let format = GeoPointFormat::Geohash("dr5rus".to_string());
        assert!(format.to_geo_point().is_err());
    }

    #[test]
    fn test_geo_point_field_type() {
        let field_type = FieldType::GeoPoint;
        let json = serde_json::to_string(&field_type).unwrap();
        assert_eq!(json, "\"geopoint\"");

        let deserialized: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FieldType::GeoPoint);
    }
}
