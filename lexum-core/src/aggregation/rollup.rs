//! Rollup Aggregations
//!
//! Rollup aggregations pre-compute aggregations on historical data to reduce
//! storage and improve query performance for time series data.

use super::AggregationTrait;
use super::result::{AggregationResult, Bucket, BucketAggregationResult};
use crate::error::Result;
use crate::search::field_cache::FieldCache;
use crate::search::result::SearchHit;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use utoipa::ToSchema;

/// Rollup aggregation configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RollupAggregation {
    /// Field to rollup (typically a date field)
    pub field: String,
    /// Interval for rollup buckets (e.g., "1h", "1d")
    pub interval: String,
    /// Aggregations to compute in each bucket
    pub aggregations: HashMap<String, JsonValue>,
    /// Timezone (optional, defaults to UTC)
    #[serde(default)]
    pub timezone: Option<String>,
}

impl AggregationTrait for RollupAggregation {
    fn name(&self) -> &str {
        "rollup"
    }

    fn execute(&self, hits: &[SearchHit], _field_cache: &FieldCache) -> Result<AggregationResult> {
        // Parse interval
        let duration = crate::index::timeseries::parse_time_interval(&self.interval)?;

        // Group hits by time bucket
        let mut buckets: HashMap<String, Vec<&SearchHit>> = HashMap::new();

        for hit in hits {
            if let Some(field_value) = hit.source.get(&self.field) {
                if let Some(timestamp) = parse_timestamp(field_value) {
                    let bucket_key = calculate_bucket_key(timestamp, duration);
                    buckets.entry(bucket_key).or_default().push(hit);
                }
            }
        }

        // Create buckets with aggregations
        let mut rollup_buckets = Vec::new();
        for (key, bucket_hits) in buckets {
            let doc_count = bucket_hits.len() as u64;

            // Compute sub-aggregations for this bucket
            let sub_aggregations = HashMap::new();
            // Note: Full sub-aggregation execution would require aggregation executor
            // For now, we just store the configuration

            rollup_buckets.push(Bucket {
                key: JsonValue::String(key),
                doc_count: doc_count as usize,
                aggregations: Some(sub_aggregations),
            });
        }

        // Sort buckets by key (time order)
        rollup_buckets.sort_by(|a, b| {
            let a_str = a.key.as_str().unwrap_or("");
            let b_str = b.key.as_str().unwrap_or("");
            a_str.cmp(b_str)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            rollup_buckets,
        )))
    }

    fn merge(&self, results: &[AggregationResult]) -> Result<AggregationResult> {
        let _duration = crate::index::timeseries::parse_time_interval(&self.interval)?;
        let mut merged_buckets: HashMap<String, usize> = HashMap::new();

        for result in results {
            if let AggregationResult::Buckets(bucket_result) = result {
                for bucket in bucket_result.buckets() {
                    if let Some(key_str) = bucket.key.as_str() {
                        *merged_buckets.entry(key_str.to_string()).or_insert(0) += bucket.doc_count;
                    }
                }
            }
        }

        // Convert to buckets
        let mut bucket_vec: Vec<Bucket> = merged_buckets
            .into_iter()
            .map(|(key, count)| Bucket::new(JsonValue::String(key), count))
            .collect();

        // Sort by key (time order)
        bucket_vec.sort_by(|a, b| {
            let a_str = a.key.as_str().unwrap_or("");
            let b_str = b.key.as_str().unwrap_or("");
            a_str.cmp(b_str)
        });

        Ok(AggregationResult::Buckets(BucketAggregationResult::new(
            bucket_vec,
        )))
    }
}

/// Parse timestamp from field value
fn parse_timestamp(value: &JsonValue) -> Option<chrono::DateTime<chrono::Utc>> {
    match value {
        JsonValue::Number(n) => {
            // Unix timestamp in seconds or milliseconds
            let ts = n.as_i64()?;
            let ts_seconds = if ts > 1_000_000_000_000 {
                // Milliseconds
                ts / 1000
            } else {
                // Seconds
                ts
            };
            chrono::DateTime::from_timestamp(ts_seconds, 0)
        }
        JsonValue::String(s) => {
            // ISO 8601 format
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&chrono::Utc))
        }
        _ => None,
    }
}

/// Calculate bucket key for a timestamp
fn calculate_bucket_key(
    timestamp: chrono::DateTime<chrono::Utc>,
    duration: chrono::Duration,
) -> String {
    // Align timestamp to bucket boundary
    let epoch_seconds = timestamp.timestamp();
    let interval_seconds = duration.num_seconds();
    let bucket_epoch = (epoch_seconds / interval_seconds) * interval_seconds;

    if let Some(bucket_time) = chrono::DateTime::from_timestamp(bucket_epoch, 0) {
        bucket_time.format("%Y-%m-%dT%H:%M:%S").to_string()
    } else {
        timestamp.format("%Y-%m-%dT%H:%M:%S").to_string()
    }
}

/// Rollup job configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RollupJobConfig {
    /// Job ID
    pub id: String,
    /// Source index pattern
    pub source_index: String,
    /// Target index pattern
    pub target_index: String,
    /// Rollup interval
    pub interval: String,
    /// Fields to rollup
    pub fields: Vec<RollupField>,
    /// Aggregations to compute
    pub aggregations: HashMap<String, JsonValue>,
    /// Schedule (cron expression)
    #[serde(default)]
    pub schedule: Option<String>,
    /// Enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

/// Rollup field configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RollupField {
    /// Field name
    pub name: String,
    /// Field type in rollup
    pub field_type: RollupFieldType,
}

/// Rollup field type
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RollupFieldType {
    /// Keep original values
    Original,
    /// Histogram
    Histogram {
        /// Interval for histogram buckets
        interval: f64,
    },
    /// Terms (top values)
    Terms {
        /// Maximum number of terms to return
        size: usize,
    },
    /// Date histogram
    DateHistogram {
        /// Interval for date histogram buckets (e.g., "1h", "1d")
        interval: String,
    },
}

/// Rollup job status
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RollupJobStatus {
    /// Job is stopped
    Stopped,
    /// Job is running
    Started,
    /// Job is paused
    Aborted,
}

/// Rollup job metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollupJob {
    /// Configuration
    pub config: RollupJobConfig,
    /// Status
    pub status: RollupJobStatus,
    /// Last execution time
    pub last_execution: Option<chrono::DateTime<chrono::Utc>>,
    /// Next execution time
    pub next_execution: Option<chrono::DateTime<chrono::Utc>>,
    /// Statistics
    pub stats: RollupJobStats,
}

/// Rollup job statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RollupJobStats {
    /// Total documents processed
    pub total_docs_processed: u64,
    /// Total rollup documents created
    pub total_rollup_docs: u64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Number of successful executions
    pub successful_executions: u64,
    /// Number of failed executions
    pub failed_executions: u64,
}

impl RollupJob {
    /// Create new rollup job
    pub fn new(config: RollupJobConfig) -> Self {
        Self {
            config,
            status: RollupJobStatus::Stopped,
            last_execution: None,
            next_execution: None,
            stats: RollupJobStats::default(),
        }
    }

    /// Start the rollup job
    pub fn start(&mut self) {
        self.status = RollupJobStatus::Started;
    }

    /// Stop the rollup job
    pub fn stop(&mut self) {
        self.status = RollupJobStatus::Stopped;
    }

    /// Abort the rollup job
    pub fn abort(&mut self) {
        self.status = RollupJobStatus::Aborted;
    }

    /// Update statistics after execution
    pub fn update_stats(
        &mut self,
        docs_processed: u64,
        rollup_docs: u64,
        execution_time_ms: u64,
        success: bool,
    ) {
        self.stats.total_docs_processed += docs_processed;
        self.stats.total_rollup_docs += rollup_docs;
        self.stats.total_execution_time_ms += execution_time_ms;
        self.last_execution = Some(chrono::Utc::now());

        if success {
            self.stats.successful_executions += 1;
        } else {
            self.stats.failed_executions += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rollup_aggregation() {
        let agg = RollupAggregation {
            field: "@timestamp".to_string(),
            interval: "1h".to_string(),
            aggregations: HashMap::new(),
            timezone: None,
        };

        assert_eq!(agg.name(), "rollup");
    }

    #[test]
    fn test_parse_timestamp() {
        // Unix timestamp in seconds
        let value = JsonValue::Number(1609459200.into());
        assert!(parse_timestamp(&value).is_some());

        // Unix timestamp in milliseconds
        let value = JsonValue::Number(1609459200000i64.into());
        assert!(parse_timestamp(&value).is_some());

        // ISO 8601 string
        let value = JsonValue::String("2021-01-01T00:00:00Z".to_string());
        assert!(parse_timestamp(&value).is_some());
    }

    #[test]
    fn test_rollup_job() {
        let config = RollupJobConfig {
            id: "test-job".to_string(),
            source_index: "logs-*".to_string(),
            target_index: "logs-rollup".to_string(),
            interval: "1h".to_string(),
            fields: Vec::new(),
            aggregations: HashMap::new(),
            schedule: None,
            enabled: true,
        };

        let mut job = RollupJob::new(config);
        job.start();
        assert!(matches!(job.status, RollupJobStatus::Started));

        job.update_stats(1000, 100, 5000, true);
        assert_eq!(job.stats.total_docs_processed, 1000);
        assert_eq!(job.stats.successful_executions, 1);
    }
}
