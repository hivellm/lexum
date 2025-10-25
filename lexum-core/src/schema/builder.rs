//! Schema builder for creating Tantivy schemas

use super::field_type::{FieldConfig, FieldType};
use crate::error::Result;
use std::collections::HashMap;
use tantivy::schema::*;

/// Builder for creating schemas
pub struct SchemaBuilder {
    fields: Vec<FieldConfig>,
}

impl SchemaBuilder {
    /// Create a new schema builder
    pub fn new() -> Self {
        Self { fields: Vec::new() }
    }

    /// Add a field to the schema
    pub fn add_field(mut self, config: FieldConfig) -> Self {
        self.fields.push(config);
        self
    }

    /// Add a text field
    pub fn add_text_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::Text))
    }

    /// Add a keyword field
    pub fn add_keyword_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::Keyword))
    }

    /// Add an i64 field
    pub fn add_i64_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::I64))
    }

    /// Add an f64 field
    pub fn add_f64_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::F64))
    }

    /// Add a date field
    pub fn add_date_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::Date))
    }

    /// Add a boolean field
    pub fn add_bool_field(self, name: impl Into<String>) -> Self {
        self.add_field(FieldConfig::new(name, FieldType::Boolean))
    }

    /// Build the Tantivy schema
    pub fn build(self) -> Result<(Schema, HashMap<String, Field>)> {
        let mut tantivy_builder = Schema::builder();
        let mut field_map = HashMap::new();

        for config in &self.fields {
            let field = match config.field_type {
                FieldType::Text => {
                    let mut options = TextOptions::default();
                    if config.indexed {
                        options = options.set_indexing_options(
                            TextFieldIndexing::default()
                                .set_tokenizer("default")
                                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
                        );
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast(Some("raw"));
                    }
                    tantivy_builder.add_text_field(&config.name, options)
                }
                FieldType::Keyword => {
                    let mut options = TextOptions::default();
                    if config.indexed {
                        options = options.set_indexing_options(
                            TextFieldIndexing::default()
                                .set_tokenizer("raw")
                                .set_index_option(IndexRecordOption::Basic),
                        );
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast(Some("raw"));
                    }
                    tantivy_builder.add_text_field(&config.name, options)
                }
                FieldType::I64 => {
                    let mut options = NumericOptions::default();
                    if config.indexed {
                        options = options.set_indexed();
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast();
                    }
                    tantivy_builder.add_i64_field(&config.name, options)
                }
                FieldType::F64 => {
                    let mut options = NumericOptions::default();
                    if config.indexed {
                        options = options.set_indexed();
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast();
                    }
                    tantivy_builder.add_f64_field(&config.name, options)
                }
                FieldType::Date => {
                    let mut options = DateOptions::default();
                    if config.indexed {
                        options = options.set_indexed();
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast();
                    }
                    tantivy_builder.add_date_field(&config.name, options)
                }
                FieldType::Boolean => {
                    // Boolean stored as u64 (0 or 1)
                    let mut options = NumericOptions::default();
                    if config.indexed {
                        options = options.set_indexed();
                    }
                    if config.stored {
                        options = options.set_stored();
                    }
                    if config.fast {
                        options = options.set_fast();
                    }
                    tantivy_builder.add_u64_field(&config.name, options)
                }
            };

            field_map.insert(config.name.clone(), field);
        }

        let schema = tantivy_builder.build();
        Ok((schema, field_map))
    }

    /// Build from field configs
    pub fn from_configs(configs: Vec<FieldConfig>) -> Result<(Schema, HashMap<String, Field>)> {
        let mut builder = Self::new();
        for config in configs {
            builder = builder.add_field(config);
        }
        builder.build()
    }
}

impl Default for SchemaBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_builder() {
        let (schema, fields) = SchemaBuilder::new()
            .add_text_field("title")
            .add_keyword_field("status")
            .add_i64_field("views")
            .add_date_field("created_at")
            .build()
            .unwrap();

        assert_eq!(fields.len(), 4);
        assert!(fields.contains_key("title"));
        assert!(fields.contains_key("status"));
        assert!(fields.contains_key("views"));
        assert!(fields.contains_key("created_at"));

        let field_entries: Vec<_> = schema.fields().collect();
        assert_eq!(field_entries.len(), 4);
    }

    #[test]
    fn test_field_types() {
        let (schema, fields) = SchemaBuilder::new()
            .add_field(
                FieldConfig::new("title", FieldType::Text)
                    .stored(true)
                    .indexed(true),
            )
            .add_field(FieldConfig::new("count", FieldType::I64).fast(true))
            .build()
            .unwrap();

        assert!(fields.contains_key("title"));
        assert!(fields.contains_key("count"));

        let field_entries: Vec<_> = schema.fields().collect();
        assert_eq!(field_entries.len(), 2);
    }
}
