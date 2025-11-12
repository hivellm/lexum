//! Document operation commands

use crate::client::LexumClient;
use anyhow::Result;
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;

#[derive(Debug, Serialize)]
struct AddDocumentRequest {
    document: JsonValue,
}

#[derive(Debug, Deserialize)]
struct AddDocumentResponse {
    id: String,
}

/// Add document
pub async fn add(url: &str, index: &str, file: &str) -> Result<()> {
    let content = fs::read_to_string(file)?;
    let document: JsonValue = serde_json::from_str(&content)?;

    let request = AddDocumentRequest { document };

    let client = LexumClient::new(url.to_string());
    let response: AddDocumentResponse = client
        .post(&format!("/api/v1/indices/{index}/documents"), &request)
        .await?;

    println!(
        "{} Document added with ID: {}",
        "✓".bright_green().bold(),
        response.id.bright_cyan()
    );

    Ok(())
}

/// Get document
pub async fn get(url: &str, index: &str, id: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    let response: JsonValue = client
        .get(&format!("/api/v1/indices/{index}/documents/{id}"))
        .await?;

    println!("{}", serde_json::to_string_pretty(&response)?);

    Ok(())
}

/// Delete document
pub async fn delete(url: &str, index: &str, id: &str) -> Result<()> {
    let client = LexumClient::new(url.to_string());
    client
        .delete(&format!("/api/v1/indices/{index}/documents/{id}"))
        .await?;

    println!(
        "{} Document '{}' deleted from index '{}'",
        "✓".bright_green().bold(),
        id.bright_cyan(),
        index.bright_yellow()
    );

    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BulkOperation {
    action: String,
    #[serde(rename = "_index")]
    index: String,
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    document: Option<JsonValue>,
}

#[derive(Debug, Serialize)]
struct BulkRequest {
    operations: Vec<BulkOperation>,
}

#[derive(Debug, Deserialize)]
struct BulkResponse {
    errors: bool,
    took_ms: u64,
    items: Vec<JsonValue>,
}

/// Bulk operations from file
pub async fn bulk(url: &str, index: &str, file: &str) -> Result<()> {
    let content = fs::read_to_string(file)?;
    let documents: Vec<JsonValue> = serde_json::from_str(&content)?;

    let mut operations = Vec::new();
    for doc in documents {
        operations.push(BulkOperation {
            action: "index".to_string(),
            index: index.to_string(),
            id: None,
            document: Some(doc),
        });
    }

    let request = BulkRequest { operations };

    let client = LexumClient::new(url.to_string());
    let response: BulkResponse = client.post("/api/v1/bulk", &request).await?;

    if response.errors {
        println!(
            "{} Bulk operation completed with some errors",
            "⚠".bright_yellow().bold()
        );
    } else {
        println!("{} Bulk operation successful", "✓".bright_green().bold());
    }

    println!(
        "  {} documents",
        response.items.len().to_string().bright_cyan()
    );
    println!("  Took: {}ms", response.took_ms.to_string().bright_yellow());

    Ok(())
}
