//! Schema management for indices

pub mod builder;
pub mod converter;
pub mod field_type;
pub mod mapping;

pub use builder::SchemaBuilder;
pub use converter::{mapping_to_schema, schema_to_mapping};
pub use field_type::{FieldConfig, FieldType};
pub use mapping::{
    DynamicMapping, ElasticsearchFieldType, ElasticsearchMapping, FieldMapping, IndexOptions,
};
