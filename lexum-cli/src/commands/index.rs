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

#[derive(Debug, Deserialize)]
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
    println!("{}", output);

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
