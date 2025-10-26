//! LQL (Lexum Query Language) command implementations

use crate::commands::search::SortOrder;
use crate::{client::LexumClient, formatter::OutputFormat, lql::LqlParser};
use anyhow::Result;
use colored::Colorize;
use serde_json::Value;

/// Execute LQL query from file
pub async fn lql_from_file(url: &str, index: &str, file_path: &str, limit: usize) -> Result<()> {
    let content = std::fs::read_to_string(file_path)?;
    let lql_query = content.trim();

    println!(
        "{}",
        format!("Executing LQL from file: {file_path}")
            .bright_cyan()
            .bold()
    );

    execute_lql_query(url, index, lql_query, limit, None, None).await
}

/// Execute advanced LQL query with sorting and field selection
pub async fn lql_advanced(
    url: &str,
    index: &str,
    lql_query: &str,
    limit: usize,
    sort_options: Option<Vec<(String, SortOrder)>>,
    fields: Option<Vec<String>>,
) -> Result<()> {
    println!(
        "{}",
        format!("Executing LQL query: {lql_query}")
            .bright_cyan()
            .bold()
    );

    execute_lql_query(url, index, lql_query, limit, sort_options, fields).await
}

/// Execute LQL query with all options
async fn execute_lql_query(
    url: &str,
    index: &str,
    lql_query: &str,
    limit: usize,
    sort_options: Option<Vec<(String, SortOrder)>>,
    fields: Option<Vec<String>>,
) -> Result<()> {
    // Parse LQL query
    let query = LqlParser::parse(lql_query)?;

    // Create client
    let client = LexumClient::new(url.to_string());

    // Build search request
    let mut search_request = serde_json::json!({
        "query": query,
        "limit": limit
    });

    // Add sorting if specified
    if let Some(sort_fields) = sort_options {
        let sort_array: Vec<Value> = sort_fields
            .into_iter()
            .map(|(field, order)| {
                let order_str = match order {
                    SortOrder::Asc => "asc",
                    SortOrder::Desc => "desc",
                };
                serde_json::json!({
                    field: order_str
                })
            })
            .collect();
        search_request["sort"] = Value::Array(sort_array);
    }

    // Add field selection if specified
    if let Some(selected_fields) = fields {
        search_request["fields"] =
            Value::Array(selected_fields.into_iter().map(Value::String).collect());
    }

    // Execute search
    let search_result: Value = client
        .post(
            &format!("{url}/api/v1/indices/{index}/search"),
            &search_request,
        )
        .await?;

    // Format and display results
    let _formatter = OutputFormat::Table;
    // For now, just print the raw JSON result
    println!("{}", serde_json::to_string_pretty(&search_result)?);

    Ok(())
}

/// Execute LQL query in REPL mode
pub async fn lql_repl(url: &str, index: &str, lql_query: &str, limit: usize) -> Result<()> {
    execute_lql_query(url, index, lql_query, limit, None, None).await
}

/// Show LQL help and examples
pub fn show_lql_help() {
    println!("{}", "LQL (Lexum Query Language) Help".bright_cyan().bold());
    println!();

    println!("{}", "Basic Syntax:".bright_yellow().bold());
    println!("  FROM <index> [WHERE <conditions>]");
    println!("  SELECT * FROM <index> [WHERE <conditions>]");
    println!("  MATCH <field>:<value>");
    println!("  COUNT FROM <index> [WHERE <conditions>]");
    println!("  GROUP BY <field> FROM <index> [WHERE <conditions>]");
    println!("  AGGREGATE <function>(<field>) FROM <index> [WHERE <conditions>]");
    println!();

    println!("{}", "Query Types:".bright_yellow().bold());
    println!("  {} - Match all documents", "FROM my_index".bright_green());
    println!(
        "  {} - Match with conditions",
        "FROM my_index WHERE title:hello".bright_green()
    );
    println!(
        "  {} - Field-specific match",
        "MATCH title:hello".bright_green()
    );
    println!();

    println!("{}", "Condition Operators:".bright_yellow().bold());
    println!("  {} - Exact match", "field:value".bright_green());
    println!("  {} - Range match", "field:[min,max]".bright_green());
    println!("  {} - Fuzzy match", "field:~value".bright_green());
    println!(
        "  {} - Phrase match",
        "field:\"exact phrase\"".bright_green()
    );
    println!("  {} - Boolean AND", "+field:value".bright_green());
    println!("  {} - Boolean NOT", "-field:value".bright_green());
    println!();

    println!("{}", "Examples:".bright_yellow().bold());
    println!("  {} - All documents", "FROM products".bright_green());
    println!(
        "  {} - Products with specific title",
        "FROM products WHERE title:laptop".bright_green()
    );
    println!(
        "  {} - Price range search",
        "FROM products WHERE price:[100,500]".bright_green()
    );
    println!(
        "  {} - Fuzzy search",
        "FROM products WHERE title:~laptp".bright_green()
    );
    println!(
        "  {} - Phrase search",
        "FROM products WHERE description:\"gaming laptop\"".bright_green()
    );
    println!(
        "  {} - Boolean search",
        "FROM products WHERE +category:electronics -status:discontinued".bright_green()
    );
    println!();

    println!("{}", "Advanced Examples:".bright_yellow().bold());
    println!(
        "  {} - Count documents",
        "COUNT FROM products WHERE category:electronics".bright_green()
    );
    println!(
        "  {} - Group by category",
        "GROUP BY category FROM products".bright_green()
    );
    println!(
        "  {} - Average price",
        "AGGREGATE AVG(price) FROM products".bright_green()
    );
    println!(
        "  {} - Max price",
        "AGGREGATE MAX(price) FROM products WHERE category:electronics".bright_green()
    );
    println!(
        "  {} - Min price",
        "AGGREGATE MIN(price) FROM products".bright_green()
    );
    println!(
        "  {} - Sum prices",
        "AGGREGATE SUM(price) FROM products WHERE category:electronics".bright_green()
    );
    println!();

    println!("{}", "Advanced Features:".bright_yellow().bold());
    println!(
        "  {} - Sort results",
        "--sort field:asc,field2:desc".bright_green()
    );
    println!(
        "  {} - Select fields",
        "--fields title,price,category".bright_green()
    );
    println!("  {} - Limit results", "--limit 20".bright_green());
    println!("  {} - Query from file", "@query.lql".bright_green());
    println!();
}
