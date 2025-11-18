//! Schema management for indices

pub mod builder;
pub mod converter;
pub mod field_type;
pub mod mapping;

pub use builder::SchemaBuilder;
pub use converter::{mapping_to_schema, schema_to_mapping};
pub use field_type::{FieldConfig, FieldType, GeoPoint, GeoPointFormat};
pub use mapping::{
    DynamicMapping, ElasticsearchFieldType, ElasticsearchMapping, FieldMapping, IndexOptions,
};
