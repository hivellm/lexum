//! MCP Tools definitions for Lexum

use rmcp::model::{Tool, ToolAnnotations};
use serde_json::json;
use std::borrow::Cow;

/// Get list of available MCP tools for Lexum
pub fn get_mcp_tools() -> Vec<Tool> {
    vec![
        // Search tool
        Tool {
            name: Cow::Borrowed("search"),
            title: Some("Search Index".to_string()),
            description: Some(Cow::Borrowed("Search documents in an index using a query.")),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "string",
                        "description": "Index name"
                    },
                    "query": {
                        "type": "object",
                        "description": "Query object (Elasticsearch-style)"
                    },
                    "q": {
                        "type": "string",
                        "description": "Simple query string"
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of results",
                        "default": 10
                    },
                    "offset": {
                        "type": "integer",
                        "description": "Result offset",
                        "default": 0
                    }
                },
                "required": ["index"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(true).idempotent(false)),
        },
        // Retrieve document tool
        Tool {
            name: Cow::Borrowed("retrieve"),
            title: Some("Retrieve Document".to_string()),
            description: Some(Cow::Borrowed(
                "Retrieve a specific document by ID from an index.",
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "string",
                        "description": "Index name"
                    },
                    "id": {
                        "type": "string",
                        "description": "Document ID"
                    }
                },
                "required": ["index", "id"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(true).idempotent(true)),
        },
        // Aggregate tool
        Tool {
            name: Cow::Borrowed("aggregate"),
            title: Some("Aggregate Data".to_string()),
            description: Some(Cow::Borrowed(
                "Perform aggregations on search results (terms, stats, etc.).",
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "string",
                        "description": "Index name"
                    },
                    "query": {
                        "type": "object",
                        "description": "Query object"
                    },
                    "aggregations": {
                        "type": "object",
                        "description": "Aggregation specifications"
                    }
                },
                "required": ["index"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(true).idempotent(false)),
        },
        // List indices tool
        Tool {
            name: Cow::Borrowed("list_indices"),
            title: Some("List Indices".to_string()),
            description: Some(Cow::Borrowed("List all available indices with metadata.")),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(true).idempotent(true)),
        },
        // Create index tool
        Tool {
            name: Cow::Borrowed("create_index"),
            title: Some("Create Index".to_string()),
            description: Some(Cow::Borrowed(
                "Create a new index with optional mappings and settings.",
            )),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {
                        "type": "string",
                        "description": "Index name"
                    },
                    "mappings": {
                        "type": "object",
                        "description": "Elasticsearch-style mappings (optional)"
                    },
                    "settings": {
                        "type": "object",
                        "description": "Index settings (optional)"
                    },
                    "fields": {
                        "type": "array",
                        "description": "Field definitions (optional, alternative to mappings)"
                    }
                },
                "required": ["name"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(false)),
        },
        // Get mapping tool
        Tool {
            name: Cow::Borrowed("get_mapping"),
            title: Some("Get Index Mapping".to_string()),
            description: Some(Cow::Borrowed("Get the mapping of an index.")),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "string",
                        "description": "Index name"
                    }
                },
                "required": ["index"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(true).idempotent(true)),
        },
        // Update mapping tool
        Tool {
            name: Cow::Borrowed("update_mapping"),
            title: Some("Update Index Mapping".to_string()),
            description: Some(Cow::Borrowed("Update the mapping of an existing index.")),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "index": {
                        "type": "string",
                        "description": "Index name"
                    },
                    "mappings": {
                        "type": "object",
                        "description": "Elasticsearch-style mappings"
                    }
                },
                "required": ["index", "mappings"]
            })
            .as_object()
            .unwrap()
            .clone()
            .into(),
            output_schema: None,
            icons: None,
            annotations: Some(ToolAnnotations::new().read_only(false)),
        },
    ]
}
