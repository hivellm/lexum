//! Search command

use crate::client::LexumClient;
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Serialize)]
struct SearchRequest {
    query: Query,
    limit: usize,
    offset: usize,
}

#[derive(Debug, Serialize)]
#[serde(tag = "match_all", rename_all = "snake_case")]
enum Query {
    #[serde(rename = "match")]
    Match {
        field: String,
        query: String,
    },
    MatchAll,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    hits: Vec<SearchHit>,
    total: usize,
    took_ms: u64,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    id: String,
    score: f32,
    source: JsonValue,
}

/// Search documents
pub async fn search(url: &str, index: &str, query: &str, limit: usize) -> Result<()> {
    let search_query = if query == "*" {
        Query::MatchAll
    } else {
        // Simple match query on "content" field for now
        Query::Match {
            field: "content".to_string(),
            query: query.to_string(),
        }
    };

    let request = SearchRequest {
        query: search_query,
        limit,
        offset: 0,
    };

    let client = LexumClient::new(url.to_string());
    let response: SearchResult = client
        .post(&format!("/api/v1/indices/{index}/search"), &request)
        .await?;

    println!(
        "{} {} results in {}ms",
        "Found".bright_cyan(),
        response.total.to_string().bright_yellow(),
        response.took_ms.to_string().bright_green()
    );

    if response.hits.is_empty() {
        return Ok(());
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec!["ID", "Score", "Document"]);

    for hit in response.hits {
        let doc_str = serde_json::to_string(&hit.source)?;
        let truncated = if doc_str.len() > 60 {
            format!("{}...", &doc_str[..60])
        } else {
            doc_str
        };

        table.add_row(vec![hit.id, format!("{:.4}", hit.score), truncated]);
    }

    println!("{table}");

    Ok(())
}
