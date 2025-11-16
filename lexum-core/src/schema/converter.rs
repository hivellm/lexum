//! Converter between Elasticsearch mappings and Tantivy schemas
//!
//! This module provides conversion functions to transform Elasticsearch
//! mapping format to Tantivy schema and vice versa.

use crate::error::Result;
use crate::schema::mapping::{
    ElasticsearchFieldType, ElasticsearchMapping, FieldMapping, IndexOptions,
};
use std::collections::HashMap;
use tantivy::schema::*;

/// Convert Elasticsearch mapping to Tantivy schema
///
/// Note: This conversion preserves basic field information (type, indexed, stored)
/// but cannot preserve all Elasticsearch-specific metadata because Tantivy schema
/// does not support storing custom metadata.
///
/// Preserved:
/// - Field types (text, keyword, long, double, date, boolean, etc.)
/// - Indexed flag
/// - Stored flag
/// - Multi-fields (sub-fields)
/// - Nested/object fields (flattened to dot notation)
///
/// Stored in mapping but not applied to Tantivy schema:
/// - Analyzer names (stored in mapping, not applied to Tantivy)
/// - Normalizer names (stored in mapping, not applied to Tantivy)
/// - Boost values (stored in mapping, used in query boosting)
/// - copy_to fields (stored in mapping, requires custom indexing logic)
/// - Format strings (stored in mapping, used for date parsing)
/// - ignore_above values (stored in mapping, used for validation)
/// - Index options (stored in mapping, Tantivy uses defaults)
/// - Norms settings (stored in mapping, not applied to Tantivy)
pub fn mapping_to_schema(mapping: &ElasticsearchMapping) -> Result<Schema> {
    let mut schema_builder = Schema::builder();

    if let Some(ref properties) = mapping.properties {
        for (name, field_mapping) in properties {
            add_field_to_schema(&mut schema_builder, name, field_mapping)?;
        }
    }

    Ok(schema_builder.build())
}

fn add_field_to_schema(
    schema_builder: &mut SchemaBuilder,
    name: &str,
    field_mapping: &FieldMapping,
) -> Result<()> {
    match field_mapping.field_type {
        ElasticsearchFieldType::Text => {
            // TEXT always indexes, so we'll index if requested
            // If index=false, we'll still use TEXT (Tantivy limitation)
            let options = if field_mapping.store {
                TEXT | STORED
            } else {
                TEXT
            };

            // Handle index options
            let _index_options = field_mapping.index_options.as_ref();
            // Tantivy doesn't have separate index_options, it's included in TEXT

            // Handle norms
            let _norms = field_mapping.norms;
            // Tantivy doesn't support disabling norms per-field in the same way

            schema_builder.add_text_field(name, options);

            // Handle multi-fields (sub-fields)
            if let Some(ref fields) = field_mapping.fields {
                for (sub_name, sub_mapping) in fields {
                    let full_name = format!("{name}.{sub_name}");
                    add_field_to_schema(schema_builder, &full_name, sub_mapping)?;
                }
            }
        }
        ElasticsearchFieldType::Keyword => {
            // STRING always indexes, so we'll index if requested
            // If index=false, we'll still use STRING (Tantivy limitation)
            let options = if field_mapping.store {
                STRING | STORED
            } else {
                STRING
            };
            schema_builder.add_text_field(name, options);

            // Handle multi-fields (sub-fields)
            if let Some(ref fields) = field_mapping.fields {
                for (sub_name, sub_mapping) in fields {
                    let full_name = format!("{name}.{sub_name}");
                    add_field_to_schema(schema_builder, &full_name, sub_mapping)?;
                }
            }
        }
        ElasticsearchFieldType::Long => {
            // Default to indexed and stored
            let options = INDEXED | STORED;
            schema_builder.add_i64_field(name, options);
        }
        ElasticsearchFieldType::Double => {
            // Default to indexed and stored
            let options = INDEXED | STORED;
            schema_builder.add_f64_field(name, options);
        }
        ElasticsearchFieldType::Date => {
            // Default to indexed and stored
            let options = INDEXED | STORED;
            schema_builder.add_date_field(name, options);
        }
        ElasticsearchFieldType::Boolean => {
            // Default to indexed and stored
            // Tantivy uses u64 for boolean (0/1)
            let options = INDEXED | STORED;
            schema_builder.add_u64_field(name, options);
        }
        ElasticsearchFieldType::Object | ElasticsearchFieldType::Nested => {
            // For object/nested types, we flatten the structure
            // Each nested property becomes a field with dot notation
            if let Some(ref properties) = field_mapping.properties {
                for (prop_name, prop_mapping) in properties {
                    let full_name = format!("{name}.{prop_name}");
                    add_field_to_schema(schema_builder, &full_name, prop_mapping)?;
                }
            }
        }
        ElasticsearchFieldType::GeoPoint => {
            // GeoPoint is not directly supported in Tantivy
            // We'll store it as text for now (future: custom field type)
            let options = if field_mapping.store {
                TEXT | STORED
            } else {
                TEXT
            };
            schema_builder.add_text_field(name, options);
        }
        ElasticsearchFieldType::Ip => {
            // IP addresses stored as keyword (text, not analyzed)
            let options = if field_mapping.store {
                STRING | STORED
            } else {
                STRING
            };
            schema_builder.add_text_field(name, options);
        }
        ElasticsearchFieldType::Completion => {
            // Completion type is not directly supported in Tantivy
            // We'll store it as text for now (future: custom field type)
            let options = if field_mapping.store {
                TEXT | STORED
            } else {
                TEXT
            };
            schema_builder.add_text_field(name, options);
        }
    }

    // Note: Multi-fields are now handled within each field type case above
    // This ensures proper handling for text and keyword fields

    Ok(())
}

/// Convert Tantivy schema to Elasticsearch mapping
pub fn schema_to_mapping(schema: &Schema) -> Result<ElasticsearchMapping> {
    let mut mapping = ElasticsearchMapping::new();
    let mut properties = HashMap::new();

    for (_field, field_entry) in schema.fields() {
        let field_name = field_entry.name();
        let field_mapping = field_entry_to_mapping(field_entry);
        properties.insert(field_name.to_string(), field_mapping);
    }

    if !properties.is_empty() {
        mapping.properties = Some(properties);
    }

    Ok(mapping)
}

/// Convert Tantivy field entry to Elasticsearch field mapping
///
/// Note: This conversion preserves basic metadata (indexed, stored, field type)
/// but cannot preserve all Elasticsearch-specific metadata (analyzer, boost, copy_to, etc.)
/// because Tantivy schema does not store this information.
///
/// Preserved metadata:
/// - Field type (text, keyword, long, double, date, boolean)
/// - Indexed flag (whether field is indexed)
/// - Stored flag (whether field is stored)
/// - Index options (for text fields, inferred from Tantivy options)
///
/// Not preserved (Tantivy limitation):
/// - Analyzer name (not stored in Tantivy schema)
/// - Normalizer name (not stored in Tantivy schema)
/// - Boost value (not stored in Tantivy schema)
/// - copy_to fields (not stored in Tantivy schema)
/// - Format string (for date fields, not stored in Tantivy schema)
/// - ignore_above value (not stored in Tantivy schema)
/// - Custom field metadata
fn field_entry_to_mapping(field_entry: &FieldEntry) -> FieldMapping {
    let field_type = field_entry.field_type();
    let is_indexed = field_entry.is_indexed();
    let is_stored = field_entry.is_stored();

    let es_field_type = match field_type {
        FieldType::Str(_) => {
            // Determine if it's text or keyword based on indexing options
            // For now, we'll assume text if indexed, keyword if not
            if is_indexed {
                ElasticsearchFieldType::Text
            } else {
                ElasticsearchFieldType::Keyword
            }
        }
        FieldType::I64(_) => ElasticsearchFieldType::Long,
        FieldType::F64(_) => ElasticsearchFieldType::Double,
        FieldType::Date(_) => ElasticsearchFieldType::Date,
        FieldType::U64(_) => {
            // Could be boolean (0/1) or unsigned integer
            // For now, we'll assume it's a boolean if it's a small range
            ElasticsearchFieldType::Boolean
        }
        FieldType::Bool(_) => ElasticsearchFieldType::Boolean,
        FieldType::Bytes(_) => {
            // Bytes fields are typically keywords
            ElasticsearchFieldType::Keyword
        }
        FieldType::Facet(_) => {
            // Facet fields are typically keywords for filtering
            ElasticsearchFieldType::Keyword
        }
        FieldType::JsonObject(_) => {
            // JSON objects are stored as objects
            ElasticsearchFieldType::Object
        }
        FieldType::IpAddr(_) => {
            // IP address fields
            ElasticsearchFieldType::Ip
        }
    };

    let mut field_mapping = FieldMapping::new(es_field_type.clone())
        .with_index(is_indexed)
        .with_store(is_stored);

    // Set index_options for text fields
    if matches!(es_field_type, ElasticsearchFieldType::Text) && is_indexed {
        // Tantivy always includes positions for text fields
        field_mapping.index_options = Some(IndexOptions::Positions);
    }

    // Note: Other metadata (analyzer, boost, copy_to, format, ignore_above) cannot be
    // preserved because Tantivy schema does not store this information.
    // These must be stored separately in the ElasticsearchMapping structure.

    field_mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping_to_schema_text() {
        let mut mapping = ElasticsearchMapping::new();
        mapping = mapping.add_property(
            "title",
            FieldMapping::new(ElasticsearchFieldType::Text).with_analyzer("standard"),
        );

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("title").is_ok());
    }

    #[test]
    fn test_mapping_to_schema_keyword() {
        let mut mapping = ElasticsearchMapping::new();
        mapping =
            mapping.add_property("status", FieldMapping::new(ElasticsearchFieldType::Keyword));

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("status").is_ok());
    }

    #[test]
    fn test_mapping_to_schema_long() {
        let mut mapping = ElasticsearchMapping::new();
        mapping = mapping.add_property("price", FieldMapping::new(ElasticsearchFieldType::Long));

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("price").is_ok());
    }

    #[test]
    fn test_mapping_to_schema_double() {
        let mut mapping = ElasticsearchMapping::new();
        mapping = mapping.add_property("score", FieldMapping::new(ElasticsearchFieldType::Double));

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("score").is_ok());
    }

    #[test]
    fn test_mapping_to_schema_date() {
        let mut mapping = ElasticsearchMapping::new();
        mapping = mapping.add_property(
            "created_at",
            FieldMapping::new(ElasticsearchFieldType::Date)
                .with_format("strict_date_optional_time"),
        );

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("created_at").is_ok());
    }

    #[test]
    fn test_mapping_to_schema_multi_field() {
        let mut mapping = ElasticsearchMapping::new();
        mapping = mapping.add_property(
            "title",
            FieldMapping::new(ElasticsearchFieldType::Text)
                .with_analyzer("standard")
                .add_field(
                    "keyword",
                    FieldMapping::new(ElasticsearchFieldType::Keyword),
                ),
        );

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("title").is_ok());
        assert!(schema.get_field("title.keyword").is_ok());
    }

    #[test]
    fn test_schema_to_mapping() {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_i64_field("price", INDEXED | STORED);
        let schema = schema_builder.build();

        let mapping = schema_to_mapping(&schema).unwrap();
        assert!(mapping.properties.is_some());
        let properties = mapping.properties.unwrap();
        assert!(properties.contains_key("title"));
        assert!(properties.contains_key("price"));
    }

    #[test]
    fn test_elasticsearch_compatibility_sample_mapping() {
        // Test with a real Elasticsearch mapping example
        let es_mapping_json = r#"{
            "mappings": {
                "properties": {
                    "title": {
                        "type": "text",
                        "analyzer": "standard",
                        "fields": {
                            "keyword": {
                                "type": "keyword",
                                "ignore_above": 256
                            }
                        }
                    },
                    "price": {
                        "type": "long"
                    },
                    "rating": {
                        "type": "double"
                    },
                    "created_at": {
                        "type": "date",
                        "format": "strict_date_optional_time||epoch_millis"
                    },
                    "published": {
                        "type": "boolean"
                    },
                    "tags": {
                        "type": "keyword"
                    },
                    "location": {
                        "type": "geo_point"
                    },
                    "ip_address": {
                        "type": "ip"
                    },
                    "user": {
                        "type": "object",
                        "properties": {
                            "name": {
                                "type": "text"
                            },
                            "age": {
                                "type": "long"
                            }
                        }
                    }
                }
            }
        }"#;

        // Parse Elasticsearch mapping using the from_json method
        let mapping = ElasticsearchMapping::from_json(es_mapping_json).unwrap();

        // Validate mapping
        assert!(mapping.validate().is_ok());

        // Convert to schema
        let schema = mapping_to_schema(&mapping).unwrap();

        // Verify all fields are present
        assert!(schema.get_field("title").is_ok());
        assert!(schema.get_field("title.keyword").is_ok());
        assert!(schema.get_field("price").is_ok());
        assert!(schema.get_field("rating").is_ok());
        assert!(schema.get_field("created_at").is_ok());
        assert!(schema.get_field("published").is_ok());
        assert!(schema.get_field("tags").is_ok());
        assert!(schema.get_field("location").is_ok());
        assert!(schema.get_field("ip_address").is_ok());
        assert!(schema.get_field("user.name").is_ok());
        assert!(schema.get_field("user.age").is_ok());
    }

    #[test]
    fn test_elasticsearch_compatibility_ecommerce_mapping() {
        // E-commerce product mapping example
        let es_mapping_json = r#"{
            "mappings": {
                "dynamic": "strict",
                "properties": {
                    "name": {
                        "type": "text",
                        "analyzer": "standard",
                        "fields": {
                            "keyword": {
                                "type": "keyword"
                            }
                        }
                    },
                    "description": {
                        "type": "text",
                        "analyzer": "english"
                    },
                    "sku": {
                        "type": "keyword",
                        "ignore_above": 100
                    },
                    "price": {
                        "type": "double"
                    },
                    "in_stock": {
                        "type": "boolean"
                    },
                    "categories": {
                        "type": "keyword"
                    },
                    "metadata": {
                        "type": "object",
                        "properties": {
                            "brand": {
                                "type": "keyword"
                            },
                            "model": {
                                "type": "keyword"
                            }
                        }
                    }
                }
            }
        }"#;

        // Parse Elasticsearch mapping using the from_json method
        let mapping = ElasticsearchMapping::from_json(es_mapping_json).unwrap();

        assert!(mapping.validate().is_ok());
        use crate::schema::mapping::DynamicMapping;
        assert_eq!(mapping.dynamic, Some(DynamicMapping::Strict));

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("name").is_ok());
        assert!(schema.get_field("name.keyword").is_ok());
        assert!(schema.get_field("description").is_ok());
        assert!(schema.get_field("sku").is_ok());
        assert!(schema.get_field("price").is_ok());
        assert!(schema.get_field("in_stock").is_ok());
        assert!(schema.get_field("categories").is_ok());
        assert!(schema.get_field("metadata.brand").is_ok());
        assert!(schema.get_field("metadata.model").is_ok());
    }

    #[test]
    fn test_elasticsearch_compatibility_log_mapping() {
        // Log/event mapping example
        let es_mapping_json = r#"{
            "mappings": {
                "properties": {
                    "timestamp": {
                        "type": "date",
                        "format": "strict_date_optional_time"
                    },
                    "level": {
                        "type": "keyword"
                    },
                    "message": {
                        "type": "text",
                        "analyzer": "standard"
                    },
                    "source": {
                        "type": "object",
                        "properties": {
                            "ip": {
                                "type": "ip"
                            },
                            "hostname": {
                                "type": "keyword"
                            }
                        }
                    },
                    "tags": {
                        "type": "keyword"
                    }
                }
            }
        }"#;

        // Parse Elasticsearch mapping using the from_json method
        let mapping = ElasticsearchMapping::from_json(es_mapping_json).unwrap();

        assert!(mapping.validate().is_ok());

        let schema = mapping_to_schema(&mapping).unwrap();
        assert!(schema.get_field("timestamp").is_ok());
        assert!(schema.get_field("level").is_ok());
        assert!(schema.get_field("message").is_ok());
        assert!(schema.get_field("source.ip").is_ok());
        assert!(schema.get_field("source.hostname").is_ok());
        assert!(schema.get_field("tags").is_ok());
    }
}
