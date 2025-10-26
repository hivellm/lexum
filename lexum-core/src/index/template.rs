//! Index template definitions and management
//!
//! Templates allow automatic configuration of indices based on naming patterns.
//! When a new index is created, it will automatically apply matching templates.

use crate::error::{Error, Result};
use crate::schema::{FieldConfig, SchemaBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tantivy::schema::Schema;
use utoipa::ToSchema;

/// Template name for identification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct TemplateName(String);

impl TemplateName {
    /// Create a new template name
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TemplateName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for TemplateName {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for TemplateName {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Index pattern for template matching
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, ToSchema)]
pub struct IndexPattern(String);

impl IndexPattern {
    /// Create a new index pattern
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    /// Get the inner string
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Check if this pattern matches an index name
    pub fn matches(&self, index_name: &str) -> bool {
        // Simple wildcard matching for now
        // TODO: Support more complex patterns like regex
        if self.0.contains('*') || self.0.contains('?') {
            self.matches_wildcard(index_name)
        } else {
            self.0 == index_name
        }
    }

    /// Wildcard pattern matching
    fn matches_wildcard(&self, index_name: &str) -> bool {
        let pattern = &self.0;
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let name_chars: Vec<char> = index_name.chars().collect();
        
        self.matches_wildcard_recursive(&pattern_chars, &name_chars, 0, 0)
    }
    
    /// Recursive wildcard matching helper
    fn matches_wildcard_recursive(&self, pattern: &[char], name: &[char], p_idx: usize, n_idx: usize) -> bool {
        // If we've consumed all pattern characters
        if p_idx >= pattern.len() {
            return n_idx >= name.len();
        }
        
        // If we've consumed all name characters but pattern has more
        if n_idx >= name.len() {
            // Only match if remaining pattern is all '*'
            return pattern[p_idx..].iter().all(|&c| c == '*');
        }
        
        match pattern[p_idx] {
            '*' => {
                // Try matching zero characters (skip the *)
                if self.matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx) {
                    return true;
                }
                // Try matching one or more characters
                if self.matches_wildcard_recursive(pattern, name, p_idx, n_idx + 1) {
                    return true;
                }
                false
            }
            '?' => {
                // Match exactly one character - must have a character to match
                if n_idx < name.len() {
                    self.matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx + 1)
                } else {
                    false
                }
            }
            _ => {
                // Match exact character
                if pattern[p_idx] == name[n_idx] {
                    self.matches_wildcard_recursive(pattern, name, p_idx + 1, n_idx + 1)
                } else {
                    false
                }
            }
        }
    }
}

impl std::fmt::Display for IndexPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for IndexPattern {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for IndexPattern {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Template definition for automatic index configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct IndexTemplate {
    /// Template name
    pub name: TemplateName,
    /// Index patterns this template applies to
    pub index_patterns: Vec<IndexPattern>,
    /// Template priority (higher number = higher priority)
    #[serde(default = "default_priority")]
    pub priority: i32,
    /// Template version
    #[serde(default = "default_version")]
    pub version: u32,
    /// Index settings to apply
    pub settings: TemplateSettings,
    /// Schema fields to apply
    pub mappings: TemplateMappings,
    /// Template order (for ordering when priority is equal)
    #[serde(default = "default_order")]
    pub order: i32,
}

fn default_priority() -> i32 {
    0
}

fn default_version() -> u32 {
    1
}

fn default_order() -> i32 {
    0
}

impl IndexTemplate {
    /// Create a new index template
    pub fn new(name: impl Into<TemplateName>) -> Self {
        Self {
            name: name.into(),
            index_patterns: Vec::new(),
            priority: 0,
            version: 1,
            settings: TemplateSettings::default(),
            mappings: TemplateMappings::default(),
            order: 0,
        }
    }

    /// Add an index pattern
    pub fn with_pattern(mut self, pattern: impl Into<IndexPattern>) -> Self {
        self.index_patterns.push(pattern.into());
        self
    }

    /// Set template priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set template version
    pub fn with_version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    /// Set template order
    pub fn with_order(mut self, order: i32) -> Self {
        self.order = order;
        self
    }

    /// Set template settings
    pub fn with_settings(mut self, settings: TemplateSettings) -> Self {
        self.settings = settings;
        self
    }

    /// Set template mappings
    pub fn with_mappings(mut self, mappings: TemplateMappings) -> Self {
        self.mappings = mappings;
        self
    }

    /// Check if this template matches an index name
    pub fn matches_index(&self, index_name: &str) -> bool {
        self.index_patterns
            .iter()
            .any(|pattern| pattern.matches(index_name))
    }

    /// Validate template configuration
    pub fn validate(&self) -> Result<()> {
        if self.index_patterns.is_empty() {
            return Err(Error::Validation(
                "Template must have at least one index pattern".to_string(),
            ));
        }

        // Validate patterns
        for pattern in &self.index_patterns {
            if pattern.as_str().is_empty() {
                return Err(Error::Validation(
                    "Index pattern cannot be empty".to_string(),
                ));
            }
        }

        // Validate settings
        self.settings.validate()?;

        // Validate mappings
        self.mappings.validate()?;

        Ok(())
    }
}

/// Template settings configuration
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateSettings {
    /// Number of shards
    #[serde(default = "default_shards")]
    pub number_of_shards: usize,
    /// Number of replicas
    #[serde(default = "default_replicas")]
    pub number_of_replicas: usize,
    /// Refresh interval in seconds
    #[serde(default = "default_refresh_interval")]
    pub refresh_interval: u64,
    /// Additional custom settings
    #[serde(default)]
    pub custom: HashMap<String, serde_json::Value>,
}

fn default_shards() -> usize {
    1
}

fn default_replicas() -> usize {
    0
}

fn default_refresh_interval() -> u64 {
    1
}

impl Default for TemplateSettings {
    fn default() -> Self {
        Self {
            number_of_shards: default_shards(),
            number_of_replicas: default_replicas(),
            refresh_interval: default_refresh_interval(),
            custom: HashMap::new(),
        }
    }
}

impl TemplateSettings {
    /// Create new template settings
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

    /// Add custom setting
    pub fn with_custom(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.custom.insert(key.into(), value);
        self
    }

    /// Validate settings
    pub fn validate(&self) -> Result<()> {
        if self.number_of_shards == 0 {
            return Err(Error::Validation(
                "Number of shards must be greater than 0".to_string(),
            ));
        }
        Ok(())
    }
}

/// Template mappings for field definitions
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TemplateMappings {
    /// Field mappings - using serde_json::Value to avoid ToSchema issues
    #[serde(default)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for TemplateMappings {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}

impl TemplateMappings {
    /// Create new template mappings
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a field mapping
    pub fn with_field(mut self, name: impl Into<String>, field: FieldConfig) -> Self {
        let field_value = serde_json::to_value(field).unwrap_or_default();
        self.properties.insert(name.into(), field_value);
        self
    }

    /// Add multiple field mappings
    pub fn with_fields(mut self, fields: HashMap<String, FieldConfig>) -> Self {
        for (name, field) in fields {
            let field_value = serde_json::to_value(field).unwrap_or_default();
            self.properties.insert(name, field_value);
        }
        self
    }

    /// Get field mapping as FieldConfig
    pub fn get_field(&self, name: &str) -> Option<FieldConfig> {
        self.properties.get(name)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Check if field exists
    pub fn has_field(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Convert to Tantivy schema
    pub fn to_schema(&self) -> Result<Schema> {
        let mut builder = SchemaBuilder::new();
        
        for (_name, field_value) in &self.properties {
            if let Ok(field_config) = serde_json::from_value::<FieldConfig>(field_value.clone()) {
                builder = builder.add_field(field_config);
            }
        }

        let (schema, _) = builder.build()?;
        Ok(schema)
    }

    /// Validate mappings
    pub fn validate(&self) -> Result<()> {
        for (name, field_value) in &self.properties {
            if name.is_empty() {
                return Err(Error::Validation(
                    "Field name cannot be empty".to_string(),
                ));
            }
            
            // Try to deserialize as FieldConfig to validate
            if let Err(e) = serde_json::from_value::<FieldConfig>(field_value.clone()) {
                return Err(Error::Validation(
                    format!("Invalid field configuration for '{}': {}", name, e),
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::FieldType;

    #[test]
    fn test_template_name() {
        let name = TemplateName::new("test_template");
        assert_eq!(name.as_str(), "test_template");
    }

    #[test]
    fn test_index_pattern_matching() {
        let pattern = IndexPattern::new("logs-*");
        assert!(pattern.matches("logs-2024"));
        assert!(pattern.matches("logs-app"));
        assert!(!pattern.matches("events-2024"));
        assert!(!pattern.matches("logs"));

        let exact_pattern = IndexPattern::new("exact-match");
        assert!(exact_pattern.matches("exact-match"));
        assert!(!exact_pattern.matches("exact-match-extra"));
    }

    #[test]
    fn test_index_template_creation() {
        let template = IndexTemplate::new("test")
            .with_pattern("logs-*")
            .with_priority(100)
            .with_version(2);

        assert_eq!(template.name.as_str(), "test");
        assert_eq!(template.priority, 100);
        assert_eq!(template.version, 2);
        assert!(template.matches_index("logs-2024"));
        assert!(!template.matches_index("events-2024"));
    }

    #[test]
    fn test_template_validation() {
        let mut template = IndexTemplate::new("test");
        
        // Should fail without patterns
        assert!(template.validate().is_err());

        // Should pass with patterns
        template = template.with_pattern("logs-*");
        assert!(template.validate().is_ok());
    }

    #[test]
    fn test_template_settings() {
        let settings = TemplateSettings::new()
            .with_shards(3)
            .with_replicas(1)
            .with_refresh_interval(5);

        assert_eq!(settings.number_of_shards, 3);
        assert_eq!(settings.number_of_replicas, 1);
        assert_eq!(settings.refresh_interval, 5);
    }

    #[test]
    fn test_template_mappings() {
        let mappings = TemplateMappings::new()
            .with_field("title", FieldConfig::new("title", FieldType::Text))
            .with_field("count", FieldConfig::new("count", FieldType::I64));

        assert!(mappings.has_field("title"));
        assert!(mappings.has_field("count"));
        assert!(!mappings.has_field("missing"));
    }

    #[test]
    fn test_wildcard_pattern_matching() {
        let pattern = IndexPattern::new("logs-*-*");
        assert!(pattern.matches("logs-app-2024"));
        assert!(pattern.matches("logs-web-2024"));
        assert!(!pattern.matches("logs-2024"));
        assert!(!pattern.matches("events-app-2024"));

        let question_mark_pattern = IndexPattern::new("logs-?");
        assert!(question_mark_pattern.matches("logs-1"));
        assert!(question_mark_pattern.matches("logs-a"));
        assert!(!question_mark_pattern.matches("logs-12"));
        assert!(!question_mark_pattern.matches("logs"));
    }
}
