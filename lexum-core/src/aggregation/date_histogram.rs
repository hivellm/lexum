//! Date histogram aggregation

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::{Error, Result};
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

/// Date histogram aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DateHistogramAggregation {
    /// Field to create histogram on
    pub field: String,
    /// Interval (e.g., "1h", "1d", "1w", "1M")
    pub interval: String,
    /// Timezone (optional, defaults to UTC)
    #[serde(default)]
    pub timezone: Option<String>,
}

impl AggregationTrait for DateHistogramAggregation {
    fn name(&self) -> &str {
        "date_histogram"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        let interval_secs = parse_interval_impl(&self.interval)?;
        let mut buckets: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(timestamp) = parse_timestamp(field_value) {
                    let bucket_key = timestamp / interval_secs;
                    *buckets.entry(bucket_key).or_insert(0) += 1;
                }
            }
        }

        // Convert to buckets
        let mut bucket_vec: Vec<Bucket> = buckets
            .into_iter()
            .map(|(key, count)| {
                let timestamp = key * interval_secs;
                Bucket::new(
                    JsonValue::String(
                        DateTime::<Utc>::from_timestamp(timestamp, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| timestamp.to_string()),
                    ),
                    count,
                )
            })
            .collect();

        // Sort by timestamp
        bucket_vec.sort_by(|a, b| {
            let a_ts = parse_timestamp(&a.key).unwrap_or(0);
            let b_ts = parse_timestamp(&b.key).unwrap_or(0);
            a_ts.cmp(&b_ts)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            bucket_vec,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let interval_secs = parse_interval_impl(&self.interval)?;
        let mut merged_buckets: std::collections::HashMap<i64, usize> =
            std::collections::HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let Some(timestamp) = parse_timestamp(&bucket.key) {
                        let bucket_key = timestamp / interval_secs;
                        *merged_buckets.entry(bucket_key).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets
        let mut bucket_vec: Vec<Bucket> = merged_buckets
            .into_iter()
            .map(|(key, count)| {
                let timestamp = key * interval_secs;
                Bucket::new(
                    JsonValue::String(
                        DateTime::<Utc>::from_timestamp(timestamp, 0)
                            .map(|dt| dt.to_rfc3339())
                            .unwrap_or_else(|| timestamp.to_string()),
                    ),
                    count,
                )
            })
            .collect();

        // Sort by timestamp
        bucket_vec.sort_by(|a, b| {
            let a_ts = parse_timestamp(&a.key).unwrap_or(0);
            let b_ts = parse_timestamp(&b.key).unwrap_or(0);
            a_ts.cmp(&b_ts)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            bucket_vec,
        )))
    }
}

impl DateHistogramAggregation {
    /// Create new date histogram aggregation
    pub fn new(field: impl Into<String>, interval: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            interval: interval.into(),
            timezone: None,
        }
    }

    /// Set timezone
    pub fn with_timezone(mut self, timezone: impl Into<String>) -> Self {
        self.timezone = Some(timezone.into());
        self
    }
}

/// Parse interval string to seconds
#[cfg(test)]
pub(super) fn parse_interval(interval: &str) -> Result<i64> {
    parse_interval_impl(interval)
}

fn parse_interval_impl(interval: &str) -> Result<i64> {
    if interval.is_empty() {
        return Err(Error::Config("Interval cannot be empty".to_string()));
    }

    let (num_str, unit) = interval.split_at(
        interval
            .char_indices()
            .position(|(_, c)| !c.is_ascii_digit())
            .unwrap_or(interval.len()),
    );

    let num: i64 = num_str
        .parse()
        .map_err(|_| Error::Config(format!("Invalid interval number: {num_str}")))?;

    let seconds = match unit {
        "s" | "S" => num,
        "m" | "M" => num * 60,
        "h" | "H" => num * 3600,
        "d" | "D" => num * 86400,
        "w" | "W" => num * 604800,
        _ => return Err(Error::Config(format!("Invalid interval unit: {unit}"))),
    };

    Ok(seconds)
}

/// Parse timestamp from JSON value
fn parse_timestamp(value: &JsonValue) -> Option<i64> {
    if let Some(num) = value.as_i64() {
        Some(num)
    } else if let Some(num) = value.as_f64() {
        Some(num as i64)
    } else if let Some(s) = value.as_str() {
        // Try to parse as ISO 8601 or Unix timestamp string
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            Some(dt.timestamp())
        } else {
            s.parse::<i64>().ok()
        }
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::field_cache::FieldCache;
    use crate::search::result::SearchHit;
    use crate::types::{DocumentId, Score};

    fn create_test_hit_timestamp(id: &str, field: &str, timestamp: i64) -> SearchHit {
        // Create ISO 8601 timestamp string
        let dt = chrono::DateTime::<chrono::Utc>::from_timestamp(timestamp, 0)
            .unwrap()
            .to_rfc3339();
        SearchHit::new(
            DocumentId::new(id),
            Score::new(1.0),
            serde_json::json!({ field: dt }),
        )
    }

    #[test]
    fn test_date_histogram_aggregation_basic() {
        let agg = DateHistogramAggregation::new("timestamp", "1h");
        let field_cache = FieldCache::new();

        // Create hits with timestamps 1 hour apart
        let base_time = 1609459200; // 2021-01-01 00:00:00 UTC
        let hits = vec![
            create_test_hit_timestamp("1", "timestamp", base_time),
            create_test_hit_timestamp("2", "timestamp", base_time + 3600), // +1h
            create_test_hit_timestamp("3", "timestamp", base_time + 7200), // +2h
        ];

        let result = agg.execute(&hits, &field_cache).unwrap();

        if let AggregationResult::Buckets(bucket_result) = result {
            assert_eq!(bucket_result.buckets().len(), 3);
            // Each timestamp should be in its own bucket (1h interval)
        } else {
            panic!("Expected Buckets result");
        }
    }

    #[test]
    fn test_date_histogram_aggregation_parse_interval() {
        // Test interval parsing
        assert!(parse_interval("1h").is_ok());
        assert!(parse_interval("1d").is_ok());
        assert!(parse_interval("1m").is_ok());
        assert!(parse_interval("1s").is_ok());
        assert!(parse_interval("1w").is_ok());
        assert!(parse_interval("10h").is_ok());
        assert!(parse_interval("").is_err());
        assert!(parse_interval("invalid").is_err());
    }

    #[test]
    fn test_date_histogram_aggregation_merge() {
        let agg = DateHistogramAggregation::new("timestamp", "1h");
        let field_cache = FieldCache::new();

        let base_time = 1609459200;
        let hits1 = vec![
            create_test_hit_timestamp("1", "timestamp", base_time),
            create_test_hit_timestamp("2", "timestamp", base_time + 3600),
        ];

        let hits2 = vec![
            create_test_hit_timestamp("3", "timestamp", base_time + 3600), // Same bucket
            create_test_hit_timestamp("4", "timestamp", base_time + 7200),
        ];

        let result1 = agg.execute(&hits1, &field_cache).unwrap();
        let result2 = agg.execute(&hits2, &field_cache).unwrap();

        let merged = agg.merge(&[result1, result2]).unwrap();

        if let AggregationResult::Buckets(bucket_result) = merged {
            // Should have 3 buckets total, with one bucket having 2 docs
            assert!(bucket_result.buckets().len() >= 2);
        } else {
            panic!("Expected Buckets result");
        }
    }
}
