//! Index settings and configuration

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Settings for index creation
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexSettings {
    /// Number of shards (for future distributed support)
    #[serde(default = "default_shards")]
    pub number_of_shards: usize,

    /// Number of replicas
    #[serde(default = "default_replicas")]
    pub number_of_replicas: usize,

    /// Refresh interval in seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
}

fn default_shards() -> usize {
    5
}

fn default_replicas() -> usize {
    1
}

fn default_refresh_interval() -> u64 {
    1
}

impl Default for IndexSettings {
    fn default() -> Self {
        Self {
            number_of_shards: default_shards(),
            number_of_replicas: default_replicas(),
            refresh_interval: default_refresh_interval(),
        }
    }
}

impl IndexSettings {
    /// Create new index settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Set number of shards
    pub fn with_shards(mut self, shards: usize) -> Self {
        self.number_of_shards = shards;
        self
    }

    /// Set number of replicas
    pub fn with_replicas(mut self, replicas: usize) -> Self {
        self.number_of_replicas = replicas;
        self
    }

    /// Set refresh interval
    pub fn with_refresh_interval(mut self, interval: u64) -> Self {
        self.refresh_interval = interval;
        self
    }

    /// Validate settings
    pub fn validate(&self) -> crate::Result<()> {
        if self.number_of_shards == 0 {
            return Err(crate::Error::Validation(
                "Number of shards must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = IndexSettings::default();
        assert_eq!(settings.number_of_shards, 5);
        assert_eq!(settings.number_of_replicas, 1);
        assert_eq!(settings.refresh_interval, 1);
    }

    #[test]
    fn test_builder_pattern() {
        let settings = IndexSettings::new()
            .with_shards(3)
            .with_replicas(2)
            .with_refresh_interval(5);

        assert_eq!(settings.number_of_shards, 3);
        assert_eq!(settings.number_of_replicas, 2);
        assert_eq!(settings.refresh_interval, 5);
    }

    #[test]
    fn test_validation_zero_shards() {
        let mut settings = IndexSettings::default();
        settings.number_of_shards = 0;
        assert!(settings.validate().is_err());
    }
}
