//! Date range aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Date range definition
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum DateRange {
    /// Range with from and to dates
    FromTo {
        /// Lower bound (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        /// Upper bound (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<String>,
        /// Custom key for this range
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
    },
    /// Simple range with just a key
    KeyOnly {
        /// Custom key
        key: String,
        /// Lower bound (inclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        from: Option<String>,
        /// Upper bound (exclusive)
        #[serde(skip_serializing_if = "Option::is_none")]
        to: Option<String>,
    },
}

impl DateRange {
    /// Get the from value as timestamp
    fn get_from_timestamp(&self) -> Option<i64> {
        match self {
            DateRange::FromTo { from, .. } | DateRange::KeyOnly { from, .. } => {
                from.as_ref().and_then(|f| parse_date_string(f))
            }
        }
    }

    /// Get the to value as timestamp
    fn get_to_timestamp(&self) -> Option<i64> {
        match self {
            DateRange::FromTo { to, .. } | DateRange::KeyOnly { to, .. } => {
                to.as_ref().and_then(|t| parse_date_string(t))
            }
        }
    }

    /// Get the key, or generate one
    fn key(&self, format: Option<&str>) -> String {
        match self {
            DateRange::FromTo { key, from, to } => {
                if let Some(k) = key {
                    k.clone()
                } else {
                    format_date_range_key(from.as_deref(), to.as_deref(), format)
                }
            }
            DateRange::KeyOnly { key, .. } => key.clone(),
        }
    }

    /// Check if a timestamp matches this range
    fn matches(&self, timestamp: i64) -> bool {
        let from = self.get_from_timestamp().unwrap_or(i64::MIN);
        let to = self.get_to_timestamp().unwrap_or(i64::MAX);
        timestamp >= from && timestamp < to
    }
}

/// Date range aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateRangeAggregation {
    /// Field to aggregate on
    pub field: String,
    /// Date ranges to create buckets for
    pub ranges: Vec<DateRange>,
    /// Date format for bucket keys (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Timezone (optional, defaults to UTC)
    #[serde(default)]
    pub timezone: Option<String>,
    /// Return keyed response (key: bucket) instead of array
    #[serde(default)]
    pub keyed: bool,
}

impl AggregationTrait for DateRangeAggregation {
    fn name(&self) -> &str {
        "date_range"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let mut bucket_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges with 0 count
        for range in &self.ranges {
            let key = range.key(self.format.as_deref());
            bucket_counts.insert(key, 0);
        }

        // Process each hit
        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(timestamp) = parse_timestamp(field_value) {
                    // Check which range this timestamp belongs to
                    for range in &self.ranges {
                        if range.matches(timestamp) {
                            let key = range.key(self.format.as_deref());
                            *bucket_counts.entry(key).or_insert(0) += 1;
                            break; // Only count in first matching range
                        }
                    }
                }
            }
        }

        // Convert to buckets, preserving range order
        let mut bucket_vec: Vec<Bucket> = Vec::new();
        for range in &self.ranges {
            let key = range.key(self.format.as_deref());
            let count = bucket_counts.get(&key).copied().unwrap_or(0);
            bucket_vec.push(Bucket::new(JsonValue::String(key), count));
        }

        if self.keyed {
            // Return keyed format
            let mut keyed_map = HashMap::new();
            for bucket in bucket_vec {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_map.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_map),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                bucket_vec,
            )))
        }
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let mut merged_counts: HashMap<String, usize> = HashMap::new();

        // Initialize all ranges
        for range in &self.ranges {
            let key = range.key(self.format.as_deref());
            merged_counts.insert(key, 0);
        }

        // Merge results from all shards
        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let JsonValue::String(key) = &bucket.key {
                        *merged_counts.entry(key.clone()).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets, preserving range order
        let mut bucket_vec: Vec<Bucket> = Vec::new();
        for range in &self.ranges {
            let key = range.key(self.format.as_deref());
            let count = merged_counts.get(&key).copied().unwrap_or(0);
            bucket_vec.push(Bucket::new(JsonValue::String(key), count));
        }

        if self.keyed {
            // Return keyed format
            let mut keyed_map = HashMap::new();
            for bucket in bucket_vec {
                if let JsonValue::String(key) = &bucket.key {
                    keyed_map.insert(key.clone(), bucket);
                }
            }
            Ok(AggregationResult::Buckets(
                BucketAggregationResult::new_keyed(keyed_map),
            ))
        } else {
            Ok(AggregationResult::Buckets(BucketAggregationResult::new(
                bucket_vec,
            )))
        }
    }
}

impl DateRangeAggregation {
    /// Create new date range aggregation
    pub fn new(field: impl Into<String>, ranges: Vec<DateRange>) -> Self {
        Self {
            field: field.into(),
            ranges,
            format: None,
            timezone: None,
            keyed: false,
        }
    }

    /// Set date format
    pub fn with_format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    /// Set timezone
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }

    /// Set keyed response
    pub fn with_keyed(mut self, keyed: bool) -> Self {
        self.keyed = keyed;
        self
    }
}

/// Parse date string to timestamp
fn parse_date_string(date_str: &str) -> Option<i64> {
    // Try ISO 8601 format first
    if let Ok(dt) = DateTime::parse_from_rfc3339(date_str) {
        return Some(dt.timestamp());
    }

    // Try parsing as Unix timestamp
    if let Ok(ts) = date_str.parse::<i64>() {
        return Some(ts);
    }

    // Try common date formats
    let formats = [
        ("%Y-%m-%d", false),
        ("%Y-%m-%d %H:%M:%S", false),
        ("%Y-%m-%dT%H:%M:%S", false),
        ("%Y-%m-%dT%H:%M:%S%z", false),
        ("%d/%m/%Y", false),
        ("%m/%d/%Y", false),
    ];

    for (format_str, has_time) in &formats {
        if *has_time {
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(date_str, format_str) {
                return Some(dt.and_utc().timestamp());
            }
        } else if let Ok(date) = chrono::NaiveDate::parse_from_str(date_str, format_str) {
            if let Some(dt) = date.and_hms_opt(0, 0, 0) {
                return Some(dt.and_utc().timestamp());
            }
        }
    }

    None
}

/// Parse timestamp from JSON value
fn parse_timestamp(value: &JsonValue) -> Option<i64> {
    if let Some(num) = value.as_i64() {
        Some(num)
    } else if let Some(num) = value.as_f64() {
        Some(num as i64)
    } else if let Some(s) = value.as_str() {
        parse_date_string(s)
    } else {
        None
    }
}

/// Format date range key
fn format_date_range_key(from: Option<&str>, to: Option<&str>, format: Option<&str>) -> String {
    let from_str = from
        .map(|f| format_date(f, format))
        .unwrap_or_else(|| "*".to_string());
    let to_str = to
        .map(|t| format_date(t, format))
        .unwrap_or_else(|| "*".to_string());
    format!("{from_str}-{to_str}")
}

/// Format date according to format string
fn format_date(date_str: &str, format: Option<&str>) -> String {
    if let Some(timestamp) = parse_date_string(date_str) {
        if let Some(dt) = DateTime::<Utc>::from_timestamp(timestamp, 0) {
            if let Some(fmt) = format {
                // Simple format implementation
                // For full format support, would need a date formatting library
                return dt.format(fmt).to_string();
            }
            // Default to ISO 8601
            return dt.to_rfc3339();
        }
    }
    date_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit_date(id: &str, field: &str, date_str: &str) -> SearchHit {
        SearchHit {
            id: DocumentId::new(id),
            score: Score::new(1.0),
            source: serde_json::json!({ field: date_str }),
        }
    }

    fn create_test_hit_timestamp(id: &str, field: &str, timestamp: i64) -> SearchHit {
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
            .unwrap()
            .to_rfc3339();
        create_test_hit_date(id, field, &dt)
    }

    #[test]
    fn test_date_range_aggregation_basic() {
        let base_time = 1609459200; // 2021-01-01 00:00:00 UTC
        let ranges = vec![
            DateRange::FromTo {
                from: Some("2021-01-01T00:00:00Z".to_string()),
                to: Some("2021-01-02T00:00:00Z".to_string()),
                key: None,
            },
            DateRange::FromTo {
                from: Some("2021-01-02T00:00:00Z".to_string()),
                to: Some("2021-01-03T00:00:00Z".to_string()),
                key: None,
            },
        ];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_timestamp("1", "date", base_time),
            create_test_hit_timestamp("2", "date", base_time + 3600), // Still in first range
            create_test_hit_timestamp("3", "date", base_time + 86400), // In second range
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
            let buckets = bucket_result.buckets();
            assert_eq!(buckets[0].doc_count, 2); // First range
            assert_eq!(buckets[1].doc_count, 1); // Second range
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_keyed() {
        let base_time = 1609459200;
        let ranges = vec![
            DateRange::FromTo {
                from: Some("2021-01-01T00:00:00Z".to_string()),
                to: Some("2021-01-02T00:00:00Z".to_string()),
                key: Some("day1".to_string()),
            },
            DateRange::FromTo {
                from: Some("2021-01-02T00:00:00Z".to_string()),
                to: Some("2021-01-03T00:00:00Z".to_string()),
                key: Some("day2".to_string()),
            },
        ];

        let agg = DateRangeAggregation::new("date", ranges).with_keyed(true);
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_timestamp("1", "date", base_time)];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert!(bucket_result.is_keyed());
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_open_ended() {
        let base_time = 1609459200;
        let ranges = vec![
            DateRange::FromTo {
                from: None,
                to: Some("2021-01-02T00:00:00Z".to_string()),
                key: Some("before".to_string()),
            },
            DateRange::FromTo {
                from: Some("2021-01-02T00:00:00Z".to_string()),
                to: None,
                key: Some("after".to_string()),
            },
        ];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![
            create_test_hit_timestamp("1", "date", base_time), // Before
            create_test_hit_timestamp("2", "date", base_time + 86400), // After
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_merge() {
        let base_time = 1609459200;
        let ranges = vec![DateRange::FromTo {
            from: Some("2021-01-01T00:00:00Z".to_string()),
            to: Some("2021-01-02T00:00:00Z".to_string()),
            key: None,
        }];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();

        let hits1 = vec![create_test_hit_timestamp("1", "date", base_time)];
        let hits2 = vec![create_test_hit_timestamp("2", "date", base_time + 3600)];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 2);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_empty_hits() {
        let ranges = vec![DateRange::FromTo {
            from: Some("2021-01-01T00:00:00Z".to_string()),
            to: Some("2021-01-02T00:00:00Z".to_string()),
            key: None,
        }];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();
        let hits = vec![];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_missing_field() {
        let ranges = vec![DateRange::FromTo {
            from: Some("2021-01-01T00:00:00Z".to_string()),
            to: Some("2021-01-02T00:00:00Z".to_string()),
            key: None,
        }];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![SearchHit {
            id: DocumentId::new("1"),
            score: Score::new(1.0),
            source: serde_json::json!({ "other_field": "value" }),
        }];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 1);
            assert_eq!(bucket_result.buckets()[0].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_range_aggregation_custom_keys() {
        let base_time = 1609459200;
        let ranges = vec![
            DateRange::KeyOnly {
                key: "custom_key_1".to_string(),
                from: Some("2021-01-01T00:00:00Z".to_string()),
                to: Some("2021-01-02T00:00:00Z".to_string()),
            },
            DateRange::KeyOnly {
                key: "custom_key_2".to_string(),
                from: Some("2021-01-02T00:00:00Z".to_string()),
                to: Some("2021-01-03T00:00:00Z".to_string()),
            },
        ];

        let agg = DateRangeAggregation::new("date", ranges);
        let field_cache = FieldCache::new();

        let hits = vec![create_test_hit_timestamp("1", "date", base_time)];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 2);
            let buckets = bucket_result.buckets();
            assert_eq!(buckets[0].doc_count, 1);
            assert_eq!(buckets[1].doc_count, 0);
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_parse_date_string() {
        // Test ISO 8601
        assert!(parse_date_string("2021-01-01T00:00:00Z").is_some());
        assert!(parse_date_string("2021-01-01T00:00:00+00:00").is_some());

        // Test Unix timestamp
        assert_eq!(parse_date_string("1609459200"), Some(1609459200));

        // Test common formats
        assert!(parse_date_string("2021-01-01").is_some());
        assert!(parse_date_string("2021-01-01 00:00:00").is_some());

        // Test invalid
        assert!(parse_date_string("invalid").is_none());
    }
}
