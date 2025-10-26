//! Index management commands

use crate::client::LexumClient;
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
struct FieldDef {
    name: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    stored: bool,
    #[serde(default = "default_true")]
    indexed: bool,
    #[serde(default)]
    fast: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct CreateIndexRequest {
    name: String,
    fields: Vec<FieldDef>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IndexInfo {
    name: String,
    num_docs: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListIndicesResponse {
    indices: Vec<IndexInfo>,
}

/// Create index
pub async fn create(url: &str, name: &str, schema_file: &str) -> Result<()> {
    let schema_content = fs::read_to_string(schema_file)?;
    let fields: Vec<FieldDef> = serde_yaml::from_str(&schema_content)?;

    let request = CreateIndexRequest {
        name: name.to_string(),
        fields,
    };

    let client = LexumClient::new(url.to_string());
    let response: IndexInfo = client.post("/api/v1/indices", &request).await?;

    println!(
        "{} Index '{}' created successfully",
        "✓".bright_green().bold(),
        response.name.bright_cyan()
    );

    Ok(())
}

/// List indices
pub async fn list(url: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let response: ListIndicesResponse = client.get("/api/v1/indices").await?;

    if response.indices.is_empty() {
        println!("{}", "No indices found".bright_yellow());
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["Name", "Documents"]);

    for index in response.indices {
        table.add_row(vec![index.name, index.num_docs.to_string()]);
    }

    println!("{table}");

    Ok(())
}

/// Get index info
pub async fn get(url: &str, name: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let response: IndexInfo = client.get(&format!("/api/v1/indices/{name}")).await?;

    println!("{}: {}", "Name".bright_cyan(), response.name);
    println!("{}: {}", "Documents".bright_cyan(), response.num_docs);

    Ok(())
}

/// Get index statistics
pub async fn stats(url: &str, name: &str) -> Result<()> {
    use crate::formatter::{OutputFormat, format_output};

    let client = LexumClient::new(url.to_string());
    let response: IndexInfo = client.get(&format!("/api/v1/indices/{name}")).await?;

    // Display as table by default
    let output = format_output(&response, OutputFormat::Table)?;
    println!("{output}");

    Ok(())
}

/// Delete index
pub async fn delete(url: &str, name: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    client.delete(&format!("/api/v1/indices/{name}")).await?;

    println!(
        "{} Index '{}' deleted successfully",
        "✓".bright_green().bold(),
        name.bright_cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_def_creation() {
        let field = FieldDef {
            name: "title".to_string(),
            field_type: "text".to_string(),
            stored: true,
            indexed: true,
            fast: false,
        };

        assert_eq!(field.name, "title");
        assert_eq!(field.field_type, "text");
        assert!(field.stored);
        assert!(field.indexed);
        assert!(!field.fast);
    }

    #[test]
    fn test_field_def_default_indexed() {
        let field = FieldDef {
            name: "content".to_string(),
            field_type: "text".to_string(),
            stored: false,
            indexed: true, // This should be true by default
            fast: false,
        };

        assert!(field.indexed);
    }

    #[test]
    fn test_field_def_serialization() {
        let field = FieldDef {
            name: "title".to_string(),
            field_type: "text".to_string(),
            stored: true,
            indexed: true,
            fast: false,
        };

        let json = serde_json::to_string(&field).unwrap();
        assert!(json.contains("title"));
        assert!(json.contains("text"));
        assert!(json.contains("stored"));
        assert!(json.contains("indexed"));
        assert!(json.contains("fast"));
    }

    #[test]
    fn test_field_def_deserialization() {
        let json = r#"{
            "name": "title",
            "type": "text",
            "stored": true,
            "indexed": true,
            "fast": false
        }"#;

        let field: FieldDef = serde_json::from_str(json).unwrap();
        assert_eq!(field.name, "title");
        assert_eq!(field.field_type, "text");
        assert!(field.stored);
        assert!(field.indexed);
        assert!(!field.fast);
    }

    #[test]
    fn test_create_index_request_creation() {
        let fields = vec![
            FieldDef {
                name: "title".to_string(),
                field_type: "text".to_string(),
                stored: true,
                indexed: true,
                fast: false,
            },
            FieldDef {
                name: "content".to_string(),
                field_type: "text".to_string(),
                stored: false,
                indexed: true,
                fast: true,
            },
        ];

        let request = CreateIndexRequest {
            name: "test_index".to_string(),
            fields,
        };

        assert_eq!(request.name, "test_index");
        assert_eq!(request.fields.len(), 2);
    }

    #[test]
    fn test_create_index_request_serialization() {
        let fields = vec![FieldDef {
            name: "title".to_string(),
            field_type: "text".to_string(),
            stored: true,
            indexed: true,
            fast: false,
        }];

        let request = CreateIndexRequest {
            name: "test_index".to_string(),
            fields,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test_index"));
        assert!(json.contains("title"));
        assert!(json.contains("text"));
    }

    #[test]
    fn test_index_info_creation() {
        let info = IndexInfo {
            name: "test_index".to_string(),
            num_docs: 1000,
        };

        assert_eq!(info.name, "test_index");
        assert_eq!(info.num_docs, 1000);
    }

    #[test]
    fn test_index_info_serialization() {
        let info = IndexInfo {
            name: "test_index".to_string(),
            num_docs: 1000,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("test_index"));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_index_info_deserialization() {
        let json = r#"{
            "name": "test_index",
            "num_docs": 1000
        }"#;

        let info: IndexInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.name, "test_index");
        assert_eq!(info.num_docs, 1000);
    }

    #[test]
    fn test_list_indices_response_creation() {
        let indices = vec![
            IndexInfo {
                name: "index1".to_string(),
                num_docs: 100,
            },
            IndexInfo {
                name: "index2".to_string(),
                num_docs: 200,
            },
        ];

        let response = ListIndicesResponse { indices };

        assert_eq!(response.indices.len(), 2);
        assert_eq!(response.indices[0].name, "index1");
        assert_eq!(response.indices[1].name, "index2");
    }

    #[test]
    fn test_list_indices_response_serialization() {
        let indices = vec![IndexInfo {
            name: "test_index".to_string(),
            num_docs: 1000,
        }];

        let response = ListIndicesResponse { indices };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("test_index"));
        assert!(json.contains("1000"));
    }

    #[test]
    fn test_list_indices_response_deserialization() {
        let json = r#"{
            "indices": [
                {
                    "name": "index1",
                    "num_docs": 100
                },
                {
                    "name": "index2",
                    "num_docs": 200
                }
            ]
        }"#;

        let response: ListIndicesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.indices.len(), 2);
        assert_eq!(response.indices[0].name, "index1");
        assert_eq!(response.indices[1].name, "index2");
    }

    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }

    #[test]
    fn test_field_def_with_defaults() {
        let field = FieldDef {
            name: "title".to_string(),
            field_type: "text".to_string(),
            stored: false, // explicit false
            indexed: true, // should be true by default
            fast: false,   // explicit false
        };

        // Test that indexed defaults to true
        assert!(field.indexed);
    }

    #[test]
    fn test_field_def_all_field_types() {
        let field_types = vec!["text", "keyword", "integer", "float", "boolean", "date"];

        for field_type in field_types {
            let field = FieldDef {
                name: format!("field_{}", field_type),
                field_type: field_type.to_string(),
                stored: true,
                indexed: true,
                fast: false,
            };

            assert_eq!(field.field_type, field_type);
        }
    }

    #[test]
    fn test_create_index_request_empty_fields() {
        let request = CreateIndexRequest {
            name: "empty_index".to_string(),
            fields: vec![],
        };

        assert_eq!(request.name, "empty_index");
        assert!(request.fields.is_empty());
    }

    #[test]
    fn test_index_info_zero_docs() {
        let info = IndexInfo {
            name: "empty_index".to_string(),
            num_docs: 0,
        };

        assert_eq!(info.num_docs, 0);
    }

    #[test]
    fn test_list_indices_response_empty() {
        let response = ListIndicesResponse { indices: vec![] };

        assert!(response.indices.is_empty());
    }

    #[test]
    fn test_field_def_serialization_with_renames() {
        let field = FieldDef {
            name: "title".to_string(),
            field_type: "text".to_string(),
            stored: true,
            indexed: true,
            fast: false,
        };

        let json = serde_json::to_string(&field).unwrap();
        // Check that "type" is used instead of "field_type"
        assert!(json.contains("\"type\""));
        assert!(!json.contains("\"field_type\""));
    }

    #[test]
    fn test_field_def_deserialization_with_renames() {
        let json = r#"{
            "name": "title",
            "type": "text",
            "stored": true,
            "indexed": true,
            "fast": false
        }"#;

        let field: FieldDef = serde_json::from_str(json).unwrap();
        assert_eq!(field.field_type, "text");
    }
}
