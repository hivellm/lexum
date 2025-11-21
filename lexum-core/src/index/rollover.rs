//! Index rollover functionality
//!
//! Provides automatic index rotation based on configurable conditions.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

/// Rollover conditions that trigger index rotation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolloverConditions {
    /// Maximum number of documents before rollover
    pub max_docs: Option<u64>,
    /// Maximum age in seconds before rollover
    pub max_age: Option<u64>,
    /// Maximum size in bytes before rollover
    pub max_size: Option<u64>,
}

impl RolloverConditions {
    /// Create new rollover conditions with default values
    pub fn new() -> Self {
        Self {
            max_docs: None,
            max_age: None,
            max_size: None,
        }
    }

    /// Set maximum number of documents
    pub fn with_max_docs(mut self, max_docs: u64) -> Self {
        self.max_docs = Some(max_docs);
        self
    }

    /// Set maximum age in seconds
    pub fn with_max_age(mut self, max_age: u64) -> Self {
        self.max_age = Some(max_age);
        self
    }

    /// Set maximum size in bytes
    pub fn with_max_size(mut self, max_size: u64) -> Self {
        self.max_size = Some(max_size);
        self
    }

    /// Check if any condition is set
    pub fn is_empty(&self) -> bool {
        self.max_docs.is_none() && self.max_age.is_none() && self.max_size.is_none()
    }
}

impl Default for RolloverConditions {
    fn default() -> Self {
        Self::new()
    }
}

/// Rollover configuration for an alias
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct RolloverConfig {
    /// Alias name to rollover
    pub alias: String,
    /// New index name pattern (should contain %{{number}} placeholder)
    pub new_index: String,
    /// Conditions for rollover
    pub conditions: RolloverConditions,
}

impl RolloverConfig {
    /// Create new rollover config
    pub fn new(alias: String, new_index: String) -> Self {
        Self {
            alias,
            new_index,
            conditions: RolloverConditions::new(),
        }
    }

    /// Set rollover conditions
    pub fn with_conditions(mut self, conditions: RolloverConditions) -> Self {
        self.conditions = conditions;
        self
    }
}

/// Information about whether rollover should occur
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RolloverInfo {
    /// Whether rollover conditions are met
    pub rollover_needed: bool,
    /// Current document count
    pub current_docs: u64,
    /// Current age in seconds
    pub current_age: u64,
    /// Current size in bytes
    pub current_size: u64,
    /// Conditions that were met (if any)
    pub met_conditions: Vec<String>,
}

impl RolloverInfo {
    /// Create rollover info indicating no rollover needed
    pub fn no_rollover(current_docs: u64, current_age: u64, current_size: u64) -> Self {
        Self {
            rollover_needed: false,
            current_docs,
            current_age,
            current_size,
            met_conditions: Vec::new(),
        }
    }

    /// Create rollover info indicating rollover is needed
    pub fn rollover_needed(
        current_docs: u64,
        current_age: u64,
        current_size: u64,
        met_conditions: Vec<String>,
    ) -> Self {
        Self {
            rollover_needed: true,
            current_docs,
            current_age,
            current_size,
            met_conditions,
        }
    }
}

/// Result of a rollover operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(utoipa::ToSchema)]
pub struct RolloverResult {
    /// Whether rollover occurred
    pub rolled_over: bool,
    /// Old index name (if rollover occurred)
    pub old_index: Option<String>,
    /// New index name (if rollover occurred)
    pub new_index: Option<String>,
    /// Rollover information
    pub info: RolloverInfo,
}

impl RolloverResult {
    /// Create result for no rollover
    pub fn no_rollover(info: RolloverInfo) -> Self {
        Self {
            rolled_over: false,
            old_index: None,
            new_index: None,
            info,
        }
    }

    /// Create result for successful rollover
    pub fn rolled_over(old_index: String, new_index: String, info: RolloverInfo) -> Self {
        Self {
            rolled_over: true,
            old_index: Some(old_index),
            new_index: Some(new_index),
            info,
        }
    }
}

/// Generate next index name from pattern
pub fn generate_next_index_name(pattern: &str) -> Result<String> {
    // Look for {{number}} or %{{number}} pattern
    if pattern.contains("{{number}}") {
        // For now, use timestamp as number
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Config(format!("Time error: {e}")))?
            .as_secs();

        Ok(pattern.replace("{{number}}", &timestamp.to_string()))
    } else if pattern.contains("%{number}") {
        // Alternative format
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::Config(format!("Time error: {e}")))?
            .as_secs();

        Ok(pattern.replace("%{number}", &timestamp.to_string()))
    } else {
        // If no pattern, append timestamp
        Ok(format!(
            "{}-{}",
            pattern,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| Error::Config(format!("Time error: {e}")))?
                .as_secs()
        ))
    }
}

/// Check if rollover conditions are met
pub fn should_rollover(conditions: &RolloverConditions, stats: &super::IndexStats) -> RolloverInfo {
    let mut met_conditions = Vec::new();
    let mut rollover_needed = false;

    // Check max_docs condition
    if let Some(max_docs) = conditions.max_docs {
        if stats.num_docs >= max_docs {
            met_conditions.push(format!("max_docs ({}) >= {}", stats.num_docs, max_docs));
            rollover_needed = true;
        }
    }

    // Check max_age condition (simplified - using index creation time would be better)
    if let Some(max_age) = conditions.max_age {
        // For now, assume current_age is 0 since we don't track creation time
        // This would need to be enhanced with proper index metadata
        let current_age = 0u64;
        if current_age >= max_age {
            met_conditions.push(format!("max_age ({current_age}) >= {max_age}"));
            rollover_needed = true;
        }
    }

    // Check max_size condition (simplified - estimating based on segments)
    if let Some(max_size) = conditions.max_size {
        // Rough estimation: assume ~1MB per segment
        let estimated_size = stats.num_segments as u64 * 1_000_000;
        if estimated_size >= max_size {
            met_conditions.push(format!("max_size ({estimated_size}) >= {max_size}"));
            rollover_needed = true;
        }
    }

    if rollover_needed {
        RolloverInfo::rollover_needed(stats.num_docs, 0, 0, met_conditions)
    } else {
        RolloverInfo::no_rollover(stats.num_docs, 0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::IndexStats;

    #[test]
    fn test_rollover_conditions_empty() {
        let conditions = RolloverConditions::new();
        assert!(conditions.is_empty());
    }

    #[test]
    fn test_rollover_conditions_with_values() {
        let conditions = RolloverConditions::new()
            .with_max_docs(1000)
            .with_max_age(86400)
            .with_max_size(100_000_000);

        assert!(!conditions.is_empty());
        assert_eq!(conditions.max_docs, Some(1000));
        assert_eq!(conditions.max_age, Some(86400));
        assert_eq!(conditions.max_size, Some(100_000_000));
    }

    #[test]
    fn test_should_rollover_max_docs() {
        let conditions = RolloverConditions::new().with_max_docs(100);
        let stats = IndexStats {
            name: "test".to_string(),
            num_docs: 150,
            num_segments: 5,
        };

        let info = should_rollover(&conditions, &stats);
        assert!(info.rollover_needed);
        assert!(!info.met_conditions.is_empty());
        assert!(info.met_conditions[0].contains("max_docs"));
    }

    #[test]
    fn test_should_rollover_no_conditions() {
        let conditions = RolloverConditions::new();
        let stats = IndexStats {
            name: "test".to_string(),
            num_docs: 50,
            num_segments: 2,
        };

        let info = should_rollover(&conditions, &stats);
        assert!(!info.rollover_needed);
        assert!(info.met_conditions.is_empty());
    }

    #[test]
    fn test_generate_next_index_name() {
        let pattern = "logs-{{number}}";
        let result = generate_next_index_name(pattern);
        assert!(result.is_ok());
        let name = result.unwrap();
        assert!(name.starts_with("logs-"));
        assert!(name.len() > 5); // Should have timestamp
    }

    #[test]
    fn test_rollover_config() {
        let config = RolloverConfig::new("logs".to_string(), "logs-{{number}}".to_string())
            .with_conditions(
                RolloverConditions::new()
                    .with_max_docs(1000)
                    .with_max_size(100_000_000),
            );

        assert_eq!(config.alias, "logs");
        assert_eq!(config.new_index, "logs-{{number}}");
        assert!(!config.conditions.is_empty());
    }
}
