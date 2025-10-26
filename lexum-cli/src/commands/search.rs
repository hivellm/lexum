//! Search command

use crate::client::LexumClient;
use anyhow::Result;
use colored::Colorize;
use comfy_table::{Table, presets::UTF8_FULL};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fs;

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: Vec<SearchHit>,
    total_hits: u64,
    took_ms: f64,
    max_score: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct SearchHit {
    id: String,
    score: f64,
    source: JsonValue,
}

#[derive(Debug, Serialize)]
struct SearchRequest {
    query: Query,
    limit: usize,
    offset: usize,
    sort: Option<Vec<SortOption>>,
    fields: Option<Vec<String>>,
    highlight: Option<bool>,
    explain: Option<bool>,
    min_score: Option<f32>,
}

#[derive(Debug, Serialize)]
struct SortOption {
    field: String,
    order: SortOrder,
}

/// Sort order for search results
#[derive(Debug, Serialize)]
pub enum SortOrder {
    /// Sort in ascending order
    #[serde(rename = "asc")]
    Asc,
    /// Sort in descending order
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "match_all", rename_all = "snake_case")]
enum Query {
    #[serde(rename = "match")]
    Match {
        field: String,
        query: String,
    },
    #[serde(rename = "term")]
    Term {
        field: String,
        value: String,
    },
    #[serde(rename = "range")]
    Range {
        field: String,
        gte: Option<f64>,
        lte: Option<f64>,
        gt: Option<f64>,
        lt: Option<f64>,
    },
    #[serde(rename = "bool")]
    Bool {
        must: Option<Vec<Query>>,
        should: Option<Vec<Query>>,
        must_not: Option<Vec<Query>>,
        filter: Option<Vec<Query>>,
    },
    #[serde(rename = "fuzzy")]
    Fuzzy {
        field: String,
        value: String,
        fuzziness: Option<u8>,
    },
    #[serde(rename = "phrase")]
    Phrase {
        field: String,
        query: String,
        slop: Option<u32>,
    },
    MatchAll,
}

/// Search documents
pub async fn search(url: &str, index: &str, query: &str, limit: usize) -> Result<()> {
    let search_query = parse_query(query);
    execute_search(url, index, search_query, limit, None, None).await
}

/// Search documents with advanced options
pub async fn search_advanced(
    url: &str,
    index: &str,
    query: &str,
    limit: usize,
    sort: Option<Vec<(String, SortOrder)>>,
    fields: Option<Vec<String>>,
) -> Result<()> {
    let search_query = parse_query(query);
    let sort_options = sort.map(|s| {
        s.into_iter()
            .map(|(field, order)| SortOption { field, order })
            .collect()
    });
    execute_search(url, index, search_query, limit, sort_options, fields).await
}

/// Search documents from file
pub async fn search_from_file(url: &str, index: &str, file_path: &str, limit: usize) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let search_query = parse_query(&content);
    execute_search(url, index, search_query, limit, None, None).await
}

/// Parse query string or JSON into Query struct
fn parse_query(query: &str) -> Query {
    // Try to parse as JSON first
    if let Ok(json_query) = serde_json::from_str::<Query>(query) {
        return json_query;
    }

    // Parse advanced query syntax
    if let Ok(advanced_query) = parse_advanced_query(query) {
        return advanced_query;
    }

    // Fallback to simple text query
    if query == "*" {
        Query::MatchAll
    } else {
        // Simple match query on "content" field for now
        Query::Match {
            field: "content".to_string(),
            query: query.to_string(),
        }
    }
}

/// Parse advanced query syntax
fn parse_advanced_query(query: &str) -> Result<Query> {
    let query = query.trim();

    // Match queries: field:value
    if let Some(colon_pos) = query.find(':') {
        let field = query[..colon_pos].trim();
        let value = query[colon_pos + 1..].trim();

        if field.is_empty() || value.is_empty() {
            return Err(anyhow::anyhow!("Invalid field:value syntax"));
        }

        // Check for special query types
        if value.starts_with('"') && value.ends_with('"') {
            // Phrase query
            let phrase = value[1..value.len() - 1].to_string();
            return Ok(Query::Phrase {
                field: field.to_string(),
                query: phrase,
                slop: None,
            });
        } else if let Some(stripped) = value.strip_prefix('~') {
            // Fuzzy query
            let fuzzy_value = stripped.to_string();
            return Ok(Query::Fuzzy {
                field: field.to_string(),
                value: fuzzy_value,
                fuzziness: None,
            });
        } else if value.starts_with('[') && value.ends_with(']') {
            // Range query
            let range_str = value[1..value.len() - 1].to_string();
            let parts: Vec<&str> = range_str.split(',').collect();
            if parts.len() == 2 {
                let gte = parts[0].trim().parse().ok();
                let lte = parts[1].trim().parse().ok();
                return Ok(Query::Range {
                    field: field.to_string(),
                    gte,
                    lte,
                    gt: None,
                    lt: None,
                });
            }
        } else {
            // Term query
            return Ok(Query::Term {
                field: field.to_string(),
                value: value.to_string(),
            });
        }
    }

    // Boolean queries: +field:value -field:value field:value
    let mut must_queries = Vec::new();
    let mut must_not_queries = Vec::new();
    let mut should_queries = Vec::new();

    for part in query.split_whitespace() {
        if let Some(stripped) = part.strip_prefix('+') {
            // Must query - avoid recursion by parsing directly
            let sub_query = parse_simple_query(stripped)?;
            must_queries.push(sub_query);
        } else if let Some(stripped) = part.strip_prefix('-') {
            // Must not query - avoid recursion by parsing directly
            let sub_query = parse_simple_query(stripped)?;
            must_not_queries.push(sub_query);
        } else {
            // Should query - avoid recursion by parsing directly
            let sub_query = parse_simple_query(part)?;
            should_queries.push(sub_query);
        }
    }

    if !must_queries.is_empty() || !must_not_queries.is_empty() || !should_queries.is_empty() {
        return Ok(Query::Bool {
            must: if must_queries.is_empty() {
                None
            } else {
                Some(must_queries)
            },
            must_not: if must_not_queries.is_empty() {
                None
            } else {
                Some(must_not_queries)
            },
            should: if should_queries.is_empty() {
                None
            } else {
                Some(should_queries)
            },
            filter: None,
        });
    }

    Err(anyhow::anyhow!("Unable to parse query"))
}

/// Parse simple query without recursion
fn parse_simple_query(query: &str) -> Result<Query> {
    let query = query.trim();

    // Match queries: field:value
    if let Some(colon_pos) = query.find(':') {
        let field = query[..colon_pos].trim();
        let value = query[colon_pos + 1..].trim();

        if field.is_empty() || value.is_empty() {
            return Err(anyhow::anyhow!("Invalid field:value syntax"));
        }

        // Check for special query types
        if value.starts_with('"') && value.ends_with('"') {
            // Phrase query
            let phrase = value[1..value.len() - 1].to_string();
            return Ok(Query::Phrase {
                field: field.to_string(),
                query: phrase,
                slop: None,
            });
        } else if let Some(stripped) = value.strip_prefix('~') {
            // Fuzzy query
            let fuzzy_value = stripped.to_string();
            return Ok(Query::Fuzzy {
                field: field.to_string(),
                value: fuzzy_value,
                fuzziness: None,
            });
        } else if value.starts_with('[') && value.ends_with(']') {
            // Range query
            let range_str = value[1..value.len() - 1].to_string();
            let parts: Vec<&str> = range_str.split(',').collect();
            if parts.len() == 2 {
                let gte = parts[0].trim().parse().ok();
                let lte = parts[1].trim().parse().ok();
                return Ok(Query::Range {
                    field: field.to_string(),
                    gte,
                    lte,
                    gt: None,
                    lt: None,
                });
            }
        } else {
            // Term query
            return Ok(Query::Term {
                field: field.to_string(),
                value: value.to_string(),
            });
        }
    }

    // Simple text query
    Ok(Query::Match {
        field: "content".to_string(),
        query: query.to_string(),
    })
}

/// Execute search with parsed query
async fn execute_search(
    url: &str,
    index: &str,
    search_query: Query,
    limit: usize,
    sort: Option<Vec<SortOption>>,
    fields: Option<Vec<String>>,
) -> Result<()> {
    let request = SearchRequest {
        query: search_query,
        limit,
        offset: 0,
        sort,
        fields,
        highlight: None,
        explain: None,
        min_score: None,
    };

    let client = LexumClient::new(url.to_string());
    let response: SearchResponse = client
        .post(&format!("/api/v1/indices/{index}/search"), &request)
        .await?;

    println!(
        "{} {} results in {}ms",
        "Found".bright_cyan(),
        response.total_hits.to_string().bright_yellow(),
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

/// Search documents from file with advanced options
#[allow(clippy::too_many_arguments)]
pub async fn search_from_file_advanced(
    url: &str,
    index: &str,
    file_path: &str,
    limit: usize,
    offset: usize,
    highlight: bool,
    explain: bool,
    min_score: Option<f32>,
) -> Result<()> {
    let content = fs::read_to_string(file_path)?;
    let search_query = parse_query(&content);
    execute_search_with_options(
        url,
        index,
        search_query,
        limit,
        offset,
        None,
        None,
        Some(highlight),
        Some(explain),
        min_score,
    )
    .await
}

/// Search documents with all advanced options
#[allow(clippy::too_many_arguments)]
pub async fn search_advanced_with_options(
    url: &str,
    index: &str,
    query: &str,
    limit: usize,
    offset: usize,
    sort: Option<Vec<(String, SortOrder)>>,
    fields: Option<Vec<String>>,
    highlight: bool,
    explain: bool,
    min_score: Option<f32>,
) -> Result<()> {
    let search_query = parse_query(query);
    let sort_options = sort.map(|s| {
        s.into_iter()
            .map(|(field, order)| SortOption { field, order })
            .collect()
    });
    execute_search_with_options(
        url,
        index,
        search_query,
        limit,
        offset,
        sort_options,
        fields,
        Some(highlight),
        Some(explain),
        min_score,
    )
    .await
}

/// Execute search with all options
#[allow(clippy::too_many_arguments)]
async fn execute_search_with_options(
    url: &str,
    index: &str,
    query: Query,
    limit: usize,
    offset: usize,
    sort: Option<Vec<SortOption>>,
    fields: Option<Vec<String>>,
    highlight: Option<bool>,
    explain: Option<bool>,
    min_score: Option<f32>,
) -> Result<()> {
    let request = SearchRequest {
        query,
        limit,
        offset,
        sort,
        fields,
        highlight,
        explain,
        min_score,
    };

    let client = LexumClient::new(url.to_string());
    let response: SearchResponse = client
        .post(&format!("/api/v1/indices/{index}/search"), &request)
        .await?;

    if response.hits.is_empty() {
        println!("{}", "No results found".bright_yellow());
        return Ok(());
    }

    // Display results with enhanced formatting
    display_search_results(
        &response,
        highlight.unwrap_or(false),
        explain.unwrap_or(false),
    );

    Ok(())
}

/// Display search results with enhanced formatting
fn display_search_results(response: &SearchResponse, highlight: bool, explain: bool) {
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);

    if highlight {
        table.set_header(vec!["ID", "Score", "Document", "Highlights"]);
    } else {
        table.set_header(vec!["ID", "Score", "Document"]);
    }

    for hit in &response.hits {
        // Optimize document display - only serialize if needed
        let doc_str = if hit.source.is_object() {
            // Try to extract key fields first for better performance
            if let Some(title) = hit.source.get("title").and_then(|v| v.as_str()) {
                format!("title: {title}")
            } else if let Some(name) = hit.source.get("name").and_then(|v| v.as_str()) {
                format!("name: {name}")
            } else {
                // Fallback to full JSON serialization
                serde_json::to_string(&hit.source).unwrap_or_else(|_| "{}".to_string())
            }
        } else {
            serde_json::to_string(&hit.source).unwrap_or_else(|_| "{}".to_string())
        };

        let truncated = if doc_str.len() > 60 {
            format!("{}...", &doc_str[..60])
        } else {
            doc_str
        };

        if highlight {
            // Add highlight information (simplified for now)
            let highlights = "**highlighted terms**".to_string();
            table.add_row(vec![
                hit.id.clone(),
                format!("{:.4}", hit.score),
                truncated,
                highlights,
            ]);
        } else {
            table.add_row(vec![hit.id.clone(), format!("{:.4}", hit.score), truncated]);
        }
    }

    println!("{table}");

    if explain {
        println!("\n{}", "Query Explanation:".bright_cyan().bold());
        println!(
            "  {}: {}",
            "Total hits".bright_yellow(),
            response.total_hits
        );
        println!("  {}: {:.2}ms", "Took".bright_yellow(), response.took_ms);
        if let Some(max_score) = response.max_score {
            println!("  {}: {:.4}", "Max score".bright_yellow(), max_score);
        }
    }
}
