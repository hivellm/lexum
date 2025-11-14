//! Index settings and configuration

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Storage-related settings for an index.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StorageSettings {
    /// Enable memory-mapped directories for index files (default: true).
    #[serde(default = "default_enable_memory_mapped_storage")]
    pub enable_memory_mapped_storage: bool,

    /// Optional read-ahead hint in bytes for sequential I/O (future optimization).
    #[serde(default)]
    pub read_ahead_bytes: Option<usize>,
}

fn default_enable_memory_mapped_storage() -> bool {
    true
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            enable_memory_mapped_storage: default_enable_memory_mapped_storage(),
            read_ahead_bytes: None,
        }
    }
}

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

    /// Storage configuration (memory-mapped, read-ahead, etc.)
    #[serde(default)]
    pub storage: StorageSettings,
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
            storage: StorageSettings::default(),
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

    /// Set storage settings
    pub fn with_storage(mut self, storage: StorageSettings) -> Self {
        self.storage = storage;
        self
    }

    /// Enable or disable memory-mapped storage
    pub fn with_memory_mapped_storage(mut self, enabled: bool) -> Self {
        self.storage.enable_memory_mapped_storage = enabled;
        self
    }

    /// Configure read-ahead hint in bytes
    pub fn with_read_ahead_bytes(mut self, bytes: Option<usize>) -> Self {
        self.storage.read_ahead_bytes = bytes;
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
        assert!(settings.storage.enable_memory_mapped_storage);
    }

    #[test]
    fn test_new_settings() {
        let settings = IndexSettings::new();
        assert_eq!(settings.number_of_shards, 5);
        assert_eq!(settings.number_of_replicas, 1);
        assert_eq!(settings.refresh_interval, 1);
        assert!(settings.storage.enable_memory_mapped_storage);
    }

    #[test]
    fn test_builder_pattern() {
        let settings = IndexSettings::new()
            .with_shards(3)
            .with_replicas(2)
            .with_refresh_interval(5)
            .with_memory_mapped_storage(false)
            .with_read_ahead_bytes(Some(4096));

        assert_eq!(settings.number_of_shards, 3);
        assert_eq!(settings.number_of_replicas, 2);
        assert_eq!(settings.refresh_interval, 5);
        assert!(!settings.storage.enable_memory_mapped_storage);
        assert_eq!(settings.storage.read_ahead_bytes, Some(4096));
    }

    #[test]
    fn test_builder_pattern_chaining() {
        let settings = IndexSettings::new()
            .with_shards(10)
            .with_replicas(3)
            .with_refresh_interval(30);

        assert_eq!(settings.number_of_shards, 10);
        assert_eq!(settings.number_of_replicas, 3);
        assert_eq!(settings.refresh_interval, 30);
    }

    #[test]
    fn test_validation_zero_shards() {
        let settings = IndexSettings {
            number_of_shards: 0,
            ..Default::default()
        };
        let result = settings.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("shards"));
    }

    #[test]
    fn test_validation_valid_settings() {
        let settings = IndexSettings::new()
            .with_shards(3)
            .with_replicas(2)
            .with_refresh_interval(5)
            .with_memory_mapped_storage(false);

        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_validation_single_shard() {
        let settings = IndexSettings::new().with_shards(1);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_settings_serialization() {
        let settings = IndexSettings::new()
            .with_shards(3)
            .with_replicas(2)
            .with_refresh_interval(5)
            .with_memory_mapped_storage(false);

        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("number_of_shards"));
        assert!(json.contains("number_of_replicas"));
        assert!(json.contains("refresh_interval"));
        assert!(json.contains("storage"));

        let deserialized: IndexSettings = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.number_of_shards, 3);
        assert_eq!(deserialized.number_of_replicas, 2);
        assert_eq!(deserialized.refresh_interval, 5);
        assert!(!deserialized.storage.enable_memory_mapped_storage);
    }

    #[test]
    fn test_settings_deserialization_with_defaults() {
        // Test that defaults are applied when fields are missing
        let json = r"{}";
        let settings: IndexSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.number_of_shards, 5);
        assert_eq!(settings.number_of_replicas, 1);
        assert_eq!(settings.refresh_interval, 1);
    }

    #[test]
    fn test_settings_partial_deserialization() {
        let json = r#"{"number_of_shards": 7}"#;
        let settings: IndexSettings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.number_of_shards, 7);
        assert_eq!(settings.number_of_replicas, 1); // Default
        assert_eq!(settings.refresh_interval, 1); // Default
        assert!(settings.storage.enable_memory_mapped_storage);
    }
}
