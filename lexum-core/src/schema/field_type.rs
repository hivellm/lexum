//! Field type definitions

use serde::{Deserialize, Serialize};

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
    fn test_field_type_serialization() {
        let field_type = FieldType::Text;
        let json = serde_json::to_string(&field_type).unwrap();
        assert_eq!(json, "\"text\"");

        let deserialized: FieldType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, FieldType::Text);
    }
}
