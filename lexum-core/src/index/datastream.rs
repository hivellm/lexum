//! Data Streams for Time Series
//!
//! Data streams provide an abstraction over multiple indices for time series data,
//! automatically managing index rollover and providing a unified query interface.

use crate::error::{Error, Result};
use crate::index::IndexSettings;
use crate::index::timeseries::TimeSeriesConfig;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Data stream configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Default)]
pub struct DataStreamConfig {
    /// Stream name
    pub name: String,
    /// Time series configuration
    #[serde(default)]
    pub time_series: TimeSeriesConfig,
    /// Index settings template
    #[serde(default)]
    pub index_settings: IndexSettings,
    /// Auto-rollover configuration
    #[serde(default)]
    pub auto_rollover: AutoRolloverConfig,
    /// Stream metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Auto-rollover configuration for data streams
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AutoRolloverConfig {
    /// Enable automatic rollover
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Maximum age for rollover (e.g., "1d", "7d")
    #[serde(default)]
    pub max_age: Option<String>,
    /// Maximum document count for rollover
    #[serde(default)]
    pub max_docs: Option<u64>,
    /// Maximum index size for rollover (in bytes)
    #[serde(default)]
    pub max_size: Option<u64>,
}

fn default_true() -> bool {
    true
}

impl Default for AutoRolloverConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_age: Some("1d".to_string()),
            max_docs: Some(1_000_000),
            max_size: None,
        }
    }
}

impl DataStreamConfig {
    /// Create new data stream configuration
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Set time series configuration
    pub fn with_time_series(mut self, config: TimeSeriesConfig) -> Self {
        self.time_series = config;
        self
    }

    /// Set index settings
    pub fn with_index_settings(mut self, settings: IndexSettings) -> Self {
        self.index_settings = settings;
        self
    }

    /// Set auto-rollover configuration
    pub fn with_auto_rollover(mut self, config: AutoRolloverConfig) -> Self {
        self.auto_rollover = config;
        self
    }
}

/// Data stream metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataStreamMetadata {
    /// Stream name
    pub name: String,
    /// Configuration
    pub config: DataStreamConfig,
    /// Indices in the stream (ordered by creation time)
    pub indices: Vec<StreamIndex>,
    /// Current write index
    pub write_index: Option<String>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub last_updated: DateTime<Utc>,
}

/// Stream index information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamIndex {
    /// Index name
    pub name: String,
    /// Index creation time
    pub created_at: DateTime<Utc>,
    /// Document count
    pub doc_count: u64,
    /// Index size in bytes
    pub size_bytes: u64,
    /// Is write index
    pub is_write_index: bool,
}

impl DataStreamMetadata {
    /// Create new data stream metadata
    pub fn new(config: DataStreamConfig) -> Self {
        Self {
            name: config.name.clone(),
            config,
            indices: Vec::new(),
            write_index: None,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Add index to stream
    pub fn add_index(&mut self, index_name: String, is_write: bool) {
        let stream_index = StreamIndex {
            name: index_name.clone(),
            created_at: Utc::now(),
            doc_count: 0,
            size_bytes: 0,
            is_write_index: is_write,
        };

        if is_write {
            // Remove write flag from previous write index
            for idx in &mut self.indices {
                if idx.is_write_index {
                    idx.is_write_index = false;
                }
            }
            self.write_index = Some(index_name.clone());
        }

        self.indices.push(stream_index);
        self.last_updated = Utc::now();
    }

    /// Get write index name
    pub fn get_write_index(&self) -> Option<&String> {
        self.write_index.as_ref()
    }

    /// Get all index names in stream
    pub fn get_indices(&self) -> Vec<&String> {
        self.indices.iter().map(|idx| &idx.name).collect()
    }

    /// Check if rollover is needed
    pub fn should_rollover(&self) -> Result<bool> {
        if !self.config.auto_rollover.enabled {
            return Ok(false);
        }

        let Some(write_idx) = self.write_index.as_ref() else {
            return Ok(false);
        };

        // Find write index metadata
        let write_index_meta = self
            .indices
            .iter()
            .find(|idx| idx.name == *write_idx)
            .ok_or_else(|| Error::Config("Write index not found in stream".to_string()))?;

        // Check max age
        if let Some(ref max_age) = self.config.auto_rollover.max_age {
            let max_age_duration = crate::index::timeseries::parse_time_interval(max_age)?;
            let age = Utc::now() - write_index_meta.created_at;
            if age > max_age_duration {
                return Ok(true);
            }
        }

        // Check max docs
        if let Some(max_docs) = self.config.auto_rollover.max_docs {
            if write_index_meta.doc_count >= max_docs {
                return Ok(true);
            }
        }

        // Check max size
        if let Some(max_size) = self.config.auto_rollover.max_size {
            if write_index_meta.size_bytes >= max_size {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Generate next index name for rollover
    pub fn generate_next_index_name(&self) -> Result<String> {
        let base_name = &self.name;
        let timestamp = Utc::now().format("%Y.%m.%d-%H%M%S");
        Ok(format!("{base_name}-{timestamp}"))
    }

    /// Update index statistics
    pub fn update_index_stats(&mut self, index_name: &str, doc_count: u64, size_bytes: u64) {
        for idx in &mut self.indices {
            if idx.name == index_name {
                idx.doc_count = doc_count;
                idx.size_bytes = size_bytes;
                break;
            }
        }
        self.last_updated = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_stream_config() {
        let config = DataStreamConfig::new("logs".to_string())
            .with_time_series(TimeSeriesConfig::new().with_time_interval("1d".to_string()));

        assert_eq!(config.name, "logs");
        assert_eq!(config.time_series.time_interval, "1d");
    }

    #[test]
    fn test_data_stream_metadata() {
        let config = DataStreamConfig::new("logs".to_string());
        let mut metadata = DataStreamMetadata::new(config);

        metadata.add_index("logs-2024-01-01".to_string(), true);
        assert_eq!(
            metadata.get_write_index(),
            Some(&"logs-2024-01-01".to_string())
        );

        metadata.add_index("logs-2024-01-02".to_string(), true);
        assert_eq!(
            metadata.get_write_index(),
            Some(&"logs-2024-01-02".to_string())
        );
    }

    #[test]
    fn test_should_rollover() {
        let config =
            DataStreamConfig::new("logs".to_string()).with_auto_rollover(AutoRolloverConfig {
                enabled: true,
                max_docs: Some(1000),
                max_age: None,
                max_size: None,
            });

        let mut metadata = DataStreamMetadata::new(config);
        metadata.add_index("logs-2024-01-01".to_string(), true);
        metadata.update_index_stats("logs-2024-01-01", 1001, 0);

        assert!(metadata.should_rollover().unwrap());
    }
}
