//! Time Series Index Type
//!
//! Provides optimization for time series data with time-based indexing,
//! automatic time-based partitioning, and time series queries.

use crate::error::{Error, Result};
use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Time series index configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeSeriesConfig {
    /// Time field name (default: "@timestamp")
    #[serde(default = "default_time_field")]
    pub time_field: String,
    /// Time interval for partitioning (e.g., "1d", "1h", "1w")
    #[serde(default = "default_time_interval")]
    pub time_interval: String,
    /// Timezone for time-based operations (default: UTC)
    #[serde(default = "default_timezone")]
    pub timezone: String,
    /// Enable time-based optimization
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Retention period in days (None = no retention)
    #[serde(default)]
    pub retention_days: Option<u64>,
}

fn default_time_field() -> String {
    "@timestamp".to_string()
}

fn default_time_interval() -> String {
    "1d".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for TimeSeriesConfig {
    fn default() -> Self {
        Self {
            time_field: default_time_field(),
            time_interval: default_time_interval(),
            timezone: default_timezone(),
            enabled: default_true(),
            retention_days: None,
        }
    }
}

impl TimeSeriesConfig {
    /// Create new time series configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set time field name
    pub fn with_time_field(mut self, field: String) -> Self {
        self.time_field = field;
        self
    }

    /// Set time interval
    pub fn with_time_interval(mut self, interval: String) -> Self {
        self.time_interval = interval;
        self
    }

    /// Set timezone
    pub fn with_timezone(mut self, timezone: String) -> Self {
        self.timezone = timezone;
        self
    }

    /// Set retention period in days
    pub fn with_retention_days(mut self, days: Option<u64>) -> Self {
        self.retention_days = days;
        self
    }

    /// Enable or disable time series optimization
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Parse time interval to duration
    pub fn parse_interval(&self) -> Result<chrono::Duration> {
        parse_time_interval(&self.time_interval)
    }

    /// Get time field name
    pub fn time_field(&self) -> &str {
        &self.time_field
    }
}

/// Parse time interval string to Duration
/// Supports: "s", "m", "h", "d", "w", "M", "y" (seconds, minutes, hours, days, weeks, months, years)
pub fn parse_time_interval(interval: &str) -> Result<chrono::Duration> {
    if interval.is_empty() {
        return Err(Error::Config("Time interval cannot be empty".to_string()));
    }

    // Parse number and unit
    let (num_str, unit) = if let Some(pos) = interval.chars().position(|c| c.is_alphabetic()) {
        let (num, unit) = interval.split_at(pos);
        (num, unit)
    } else {
        return Err(Error::Config(format!(
            "Invalid time interval format: {interval}. Expected format: <number><unit> (e.g., '1h', '30m')"
        )));
    };

    let num: i64 = num_str
        .parse()
        .map_err(|_| Error::Config(format!("Invalid number in time interval: {num_str}")))?;

    let duration = match unit {
        "s" | "S" => chrono::Duration::seconds(num),
        "m" | "M" if unit.len() == 1 => chrono::Duration::minutes(num),
        "h" | "H" => chrono::Duration::hours(num),
        "d" | "D" => chrono::Duration::days(num),
        "w" | "W" => chrono::Duration::weeks(num),
        "M" if unit.len() == 1 => {
            // Months are approximate (30 days)
            chrono::Duration::days(num * 30)
        }
        "y" | "Y" => {
            // Years are approximate (365 days)
            chrono::Duration::days(num * 365)
        }
        _ => {
            return Err(Error::Config(format!(
                "Unsupported time unit: {unit}. Supported: s, m, h, d, w, M, y"
            )));
        }
    };

    Ok(duration)
}

/// Time series index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesMetadata {
    /// Configuration
    pub config: TimeSeriesConfig,
    /// Time partitions (start time -> partition info)
    pub partitions: HashMap<String, TimePartition>,
    /// Last update time
    pub last_updated: DateTime<Utc>,
}

/// Time partition information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimePartition {
    /// Partition start time
    pub start_time: DateTime<Utc>,
    /// Partition end time
    pub end_time: DateTime<Utc>,
    /// Document count in partition
    pub doc_count: u64,
    /// Partition size in bytes
    pub size_bytes: u64,
}

impl TimePartition {
    /// Create new time partition
    pub fn new(start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            start_time,
            end_time,
            doc_count: 0,
            size_bytes: 0,
        }
    }

    /// Check if timestamp is within partition
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        timestamp >= self.start_time && timestamp < self.end_time
    }
}

impl TimeSeriesMetadata {
    /// Create new time series metadata
    pub fn new(config: TimeSeriesConfig) -> Self {
        Self {
            config,
            partitions: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    /// Get partition for a given timestamp
    pub fn get_partition(&self, timestamp: DateTime<Utc>) -> Result<String> {
        let duration = self.config.parse_interval()?;

        // Calculate partition start time
        let partition_start = self.calculate_partition_start(timestamp, duration);
        let partition_key = partition_start.format("%Y-%m-%dT%H:%M:%S").to_string();

        Ok(partition_key)
    }

    /// Calculate partition start time for a given timestamp
    fn calculate_partition_start(
        &self,
        timestamp: DateTime<Utc>,
        duration: chrono::Duration,
    ) -> DateTime<Utc> {
        // For daily intervals, align to day boundary
        if self.config.time_interval.ends_with('d') {
            timestamp
                .date_naive()
                .and_hms_opt(0, 0, 0)
                .unwrap()
                .and_utc()
        } else if self.config.time_interval.ends_with('h') {
            // For hourly intervals, align to hour boundary
            timestamp
                .date_naive()
                .and_hms_opt(timestamp.time().hour(), 0, 0)
                .unwrap()
                .and_utc()
        } else {
            // For other intervals, use a simple division approach
            let epoch_seconds = timestamp.timestamp();
            let interval_seconds = duration.num_seconds();
            let partition_epoch = (epoch_seconds / interval_seconds) * interval_seconds;
            DateTime::from_timestamp(partition_epoch, 0).unwrap_or(timestamp)
        }
    }

    /// Add or update partition
    pub fn update_partition(&mut self, partition_key: String, doc_count: u64, size_bytes: u64) {
        if let Some(partition) = self.partitions.get_mut(&partition_key) {
            partition.doc_count = doc_count;
            partition.size_bytes = size_bytes;
        } else {
            // Parse partition key to get start time
            if let Ok(start_time) = DateTime::parse_from_rfc3339(&format!("{partition_key}Z")) {
                let duration = self
                    .config
                    .parse_interval()
                    .unwrap_or(chrono::Duration::days(1));
                let end_time = start_time + duration;
                self.partitions.insert(
                    partition_key,
                    TimePartition {
                        start_time: start_time.with_timezone(&Utc),
                        end_time: end_time.with_timezone(&Utc),
                        doc_count,
                        size_bytes,
                    },
                );
            }
        }
        self.last_updated = Utc::now();
    }

    /// Get partitions in time range
    pub fn get_partitions_in_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<&TimePartition> {
        self.partitions
            .values()
            .filter(|p| p.start_time < end && p.end_time > start)
            .collect()
    }

    /// Clean up old partitions based on retention policy
    pub fn cleanup_old_partitions(&mut self) -> Result<usize> {
        if let Some(retention_days) = self.config.retention_days {
            let cutoff = Utc::now() - chrono::Duration::days(retention_days as i64);
            let initial_count = self.partitions.len();

            self.partitions
                .retain(|_, partition| partition.end_time > cutoff);

            let removed = initial_count - self.partitions.len();
            if removed > 0 {
                self.last_updated = Utc::now();
            }

            Ok(removed)
        } else {
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_interval() {
        assert!(parse_time_interval("1h").is_ok());
        assert!(parse_time_interval("30m").is_ok());
        assert!(parse_time_interval("1d").is_ok());
        assert!(parse_time_interval("1w").is_ok());
        assert!(parse_time_interval("1M").is_ok());
        assert!(parse_time_interval("1y").is_ok());
        assert!(parse_time_interval("invalid").is_err());
    }

    #[test]
    fn test_time_series_config() {
        let config = TimeSeriesConfig::new()
            .with_time_field("timestamp".to_string())
            .with_time_interval("1h".to_string())
            .with_retention_days(Some(30));

        assert_eq!(config.time_field, "timestamp");
        assert_eq!(config.time_interval, "1h");
        assert_eq!(config.retention_days, Some(30));
    }

    #[test]
    fn test_time_partition_contains() {
        let start = Utc::now();
        let end = start + chrono::Duration::hours(1);
        let partition = TimePartition::new(start, end);

        assert!(partition.contains(start));
        assert!(partition.contains(start + chrono::Duration::minutes(30)));
        assert!(!partition.contains(end));
        assert!(!partition.contains(start - chrono::Duration::minutes(1)));
    }

    #[test]
    fn test_get_partition() {
        let config = TimeSeriesConfig::new().with_time_interval("1d".to_string());
        let metadata = TimeSeriesMetadata::new(config);
        let timestamp = Utc::now();

        let partition_key = metadata.get_partition(timestamp);
        assert!(partition_key.is_ok());
    }
}
