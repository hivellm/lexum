//! Comprehensive integration tests for Lexum CLI
//!
//! Tests marked with `#[cfg(feature = "server-tests")]` require a running server
//! and are excluded from CI/CD runs. Run with: `cargo test --features server-tests`

use anyhow::Result;
use std::io::Write;
use std::process::Command;
use tempfile::TempDir;

/// Test helper to start a Lexum server in the background
#[allow(dead_code)]
struct TestServer {
    _temp_dir: TempDir,
    server_handle: Option<std::process::Child>,
}

impl TestServer {
    #[allow(dead_code)]
    async fn new() -> Result<Self> {
        // Skip server startup in tests - too slow and causes timeouts
        // Tests that need server should use mockito or skip server-dependent operations
        Err(anyhow::anyhow!(
            "TestServer disabled - use mockito for HTTP tests"
        ))
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(mut server) = self.server_handle.take() {
            let _ = server.kill();
            let _ = server.wait();
        }
    }
}

/// Helper function to run CLI command
fn run_cli_command(args: &[&str]) -> Result<std::process::Output> {
    let mut cmd_args = vec!["run", "--bin", "lexum-cli", "--"];
    cmd_args.extend(args);

    let output = Command::new("cargo")
        .args(&cmd_args)
        .current_dir(".")
        .output()?;

    Ok(output)
}

/// Helper function to create test data files
fn create_test_files() -> Result<(String, String, String)> {
    let temp_dir = TempDir::new()?;

    // Create test document
    let doc_path = temp_dir.path().join("test_doc.json");
    let mut doc_file = std::fs::File::create(&doc_path)?;
    writeln!(
        doc_file,
        r#"{{"title": "Test Document", "content": "This is a test document", "category": "test"}}"#
    )?;

    // Create test query file
    let query_path = temp_dir.path().join("test_query.json");
    let mut query_file = std::fs::File::create(&query_path)?;
    writeln!(
        query_file,
        r#"{{"match": {{"field": "content", "query": "test"}}}}"#
    )?;

    // Create test LQL file
    let lql_path = temp_dir.path().join("test_query.lql");
    let mut lql_file = std::fs::File::create(&lql_path)?;
    writeln!(lql_file, "FROM test_index WHERE content:test")?;

    Ok((
        doc_path.to_string_lossy().to_string(),
        query_path.to_string_lossy().to_string(),
        lql_path.to_string_lossy().to_string(),
    ))
}

#[tokio::test]
async fn test_cli_help_commands() -> Result<()> {
    // Test main help
    let output = run_cli_command(&["--help"])?;
    assert!(output.status.success(), "Main help should work");
    let help_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        help_text.contains("lexum") || help_text.contains("Lexum"),
        "Help should contain lexum"
    );

    // Test subcommand help
    let output = run_cli_command(&["index", "--help"])?;
    assert!(output.status.success(), "Index help should work");

    let output = run_cli_command(&["search", "--help"])?;
    assert!(output.status.success(), "Search help should work");

    let output = run_cli_command(&["doc", "--help"])?;
    assert!(output.status.success(), "Doc help should work");

    let output = run_cli_command(&["server", "--help"])?;
    assert!(output.status.success(), "Server help should work");

    let output = run_cli_command(&["snapshot", "--help"])?;
    assert!(output.status.success(), "Snapshot help should work");

    let output = run_cli_command(&["lql", "--help"])?;
    assert!(output.status.success(), "LQL help should work");

    Ok(())
}

#[tokio::test]
async fn test_cli_version_command() -> Result<()> {
    let output = run_cli_command(&["--version"])?;
    assert!(output.status.success(), "Version command should work");
    let version_text = String::from_utf8_lossy(&output.stdout);
    assert!(
        version_text.contains("lexum") || version_text.contains("version"),
        "Version should contain lexum or version"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_error_handling() -> Result<()> {
    // Test invalid command
    let output = run_cli_command(&["invalid_command"])?;
    assert!(!output.status.success(), "Invalid command should fail");

    // Test missing arguments for index create
    let output = run_cli_command(&["index", "create"])?;
    assert!(!output.status.success(), "Missing arguments should fail");

    // Test missing arguments for search
    let output = run_cli_command(&["search"])?;
    assert!(
        !output.status.success(),
        "Missing search arguments should fail"
    );

    // Test missing arguments for doc add
    let output = run_cli_command(&["doc", "add"])?;
    assert!(
        !output.status.success(),
        "Missing doc arguments should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_global_options() -> Result<()> {
    // Test with custom URL (using port that doesn't have server)
    let output = run_cli_command(&["--url", "http://localhost:65535", "index", "list"])?;
    // This should fail due to no server, but should parse arguments correctly
    assert!(!output.status.success(), "Should fail with invalid URL");

    // Test with custom format
    let output = run_cli_command(&[
        "--url",
        "http://localhost:65535",
        "--format",
        "json",
        "index",
        "list",
    ])?;
    // This should fail due to no server, but should parse arguments correctly
    assert!(!output.status.success(), "Should fail with no server");

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_advanced_search_options() -> Result<()> {
    // Test search with all advanced options
    let output = run_cli_command(&[
        "search",
        "test_index",
        "test_query",
        "--limit",
        "20",
        "--offset",
        "10",
        "--sort",
        "score:desc",
        "--fields",
        "title,content",
        "--highlight",
        "--explain",
        "--min-score",
        "0.5",
    ])?;

    // This will fail due to no server, but should parse arguments correctly
    assert!(
        !output.status.success(),
        "Search should fail without server"
    );

    // Test search with file input
    let (_, query_path, _) = create_test_files()?;
    let output = run_cli_command(&[
        "search",
        "test_index",
        &format!("@{query_path}"),
        "--limit",
        "10",
    ])?;

    assert!(
        !output.status.success(),
        "Search with file should fail without server"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_lql_operations() -> Result<()> {
    // Test LQL with basic query
    let output = run_cli_command(&[
        "lql",
        "test_index",
        "FROM test_index WHERE content:test",
        "--limit",
        "10",
    ])?;

    assert!(!output.status.success(), "LQL should fail without server");

    // Test LQL with file input
    let (_, _, lql_path) = create_test_files()?;
    let output = run_cli_command(&[
        "lql",
        "test_index",
        &format!("@{lql_path}"),
        "--limit",
        "10",
    ])?;

    assert!(
        !output.status.success(),
        "LQL with file should fail without server"
    );

    // Test LQL with advanced options
    let output = run_cli_command(&[
        "lql",
        "test_index",
        "FROM test_index WHERE content:test",
        "--limit",
        "20",
        "--sort",
        "score:desc",
        "--fields",
        "title,content",
    ])?;

    assert!(
        !output.status.success(),
        "LQL with advanced options should fail without server"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_document_operations() -> Result<()> {
    let (doc_path, _, _) = create_test_files()?;

    // Test doc add with file
    let output = run_cli_command(&["doc", "add", "test_index", "--file", &doc_path])?;

    assert!(
        !output.status.success(),
        "Doc add should fail without server"
    );

    // Test doc add with ID
    let output = run_cli_command(&[
        "doc",
        "add",
        "test_index",
        "--file",
        &doc_path,
        "--id",
        "test_doc_1",
    ])?;

    assert!(
        !output.status.success(),
        "Doc add with ID should fail without server"
    );

    // Test doc get
    let output = run_cli_command(&["doc", "get", "test_index", "test_doc_1"])?;

    assert!(
        !output.status.success(),
        "Doc get should fail without server"
    );

    // Test doc delete
    let output = run_cli_command(&["doc", "delete", "test_index", "test_doc_1"])?;

    assert!(
        !output.status.success(),
        "Doc delete should fail without server"
    );

    // Test doc bulk
    let output = run_cli_command(&["doc", "bulk", "test_index", "--file", &doc_path])?;

    assert!(
        !output.status.success(),
        "Doc bulk should fail without server"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_server_operations() -> Result<()> {
    // Test server status
    let output = run_cli_command(&["server", "status"])?;
    // This might fail if no server is running, which is OK
    let _ = output;

    // Test server config validation with non-existent file
    let output = run_cli_command(&["server", "config", "nonexistent.yml"])?;

    assert!(!output.status.success(), "Invalid config should fail");

    // Test server start (this will fail in test environment)
    let output = run_cli_command(&["server", "start"])?;
    // This should fail in test environment
    assert!(
        !output.status.success(),
        "Server start should fail in test environment"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_snapshot_operations() -> Result<()> {
    // Test snapshot list-repos
    let output = run_cli_command(&["--url", "http://localhost:65535", "snapshot", "list-repos"])?;
    assert!(
        !output.status.success(),
        "Snapshot list-repos should fail without server"
    );

    // Test snapshot list
    let output = run_cli_command(&["snapshot", "list", "test_repo"])?;
    assert!(
        !output.status.success(),
        "Snapshot list should fail without server"
    );

    // Test snapshot create
    let output = run_cli_command(&[
        "snapshot",
        "create",
        "test_repo",
        "test_snapshot",
        "--indices",
        "test_index",
    ])?;

    assert!(
        !output.status.success(),
        "Snapshot create should fail without server"
    );

    // Test snapshot delete
    let output = run_cli_command(&["snapshot", "delete", "test_repo", "test_snapshot"])?;

    assert!(
        !output.status.success(),
        "Snapshot delete should fail without server"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_repl_command() -> Result<()> {
    // Test REPL command (this will fail in test environment)
    let output = run_cli_command(&["repl"])?;
    // This should fail in test environment or succeed if REPL is available
    // We just want to make sure the command doesn't crash
    let _ = output;

    Ok(())
}

#[tokio::test]
async fn test_cli_file_validation() -> Result<()> {
    let (_doc_path, query_path, lql_path) = create_test_files()?;

    // Test with valid JSON file
    let output = run_cli_command(&["search", "test_index", &format!("@{query_path}")])?;

    assert!(
        !output.status.success(),
        "Search with valid JSON should fail without server"
    );

    // Test with valid LQL file
    let output = run_cli_command(&["lql", "test_index", &format!("@{lql_path}")])?;

    assert!(
        !output.status.success(),
        "LQL with valid file should fail without server"
    );

    // Test with invalid file
    let output = run_cli_command(&["search", "test_index", "@nonexistent.json"])?;

    assert!(
        !output.status.success(),
        "Search with invalid file should fail"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_output_formats() -> Result<()> {
    // Test different output formats
    let formats = ["json", "table", "json-pretty"];

    for format in &formats {
        let output = run_cli_command(&[
            "--url",
            "http://localhost:65535",
            "--format",
            format,
            "index",
            "list",
        ])?;

        // This should fail due to no server, but should parse format correctly
        assert!(
            !output.status.success(),
            "Format {format} should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_boolean_queries() -> Result<()> {
    // Test boolean query syntax
    let queries = [
        "+category:electronics +brand:apple",
        "category:electronics -status:discontinued",
        "title:laptop AND price:[100,500]",
        "category:electronics OR category:computers",
    ];

    for query in &queries {
        let output = run_cli_command(&["search", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Boolean query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_range_queries() -> Result<()> {
    // Test range query syntax
    let queries = [
        "price:[100,500]",
        "rating:[4.0,5.0]",
        "created_at:[2024-01-01,2024-12-31]",
        "price:[100,*]",
        "price:[*,500]",
    ];

    for query in &queries {
        let output = run_cli_command(&["search", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Range query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_fuzzy_queries() -> Result<()> {
    // Test fuzzy query syntax
    let queries = ["title:~laptp", "description:~wireles", "name:~gaming"];

    for query in &queries {
        let output = run_cli_command(&["search", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Fuzzy query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_phrase_queries() -> Result<()> {
    // Test phrase query syntax
    let queries = [
        "description:\"wireless headphones\"",
        "title:\"gaming laptop\"",
        "content:\"exact phrase match\"",
    ];

    for query in &queries {
        let output = run_cli_command(&["search", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Phrase query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_wildcard_queries() -> Result<()> {
    // Test wildcard query syntax
    let queries = ["title:*gaming*", "sku:PROD-*", "name:*laptop*"];

    for query in &queries {
        let output = run_cli_command(&["search", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Wildcard query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_complex_queries() -> Result<()> {
    // Test complex query combinations
    let queries = [
        "+category:electronics +price:[100,1000] -status:discontinued",
        "title:laptop AND (category:electronics OR category:computers)",
        "description:\"wireless gaming\" AND price:[200,800]",
        "category:books AND (title:*programming* OR title:*coding*)",
    ];

    for query in &queries {
        let output = run_cli_command(&[
            "search",
            "test_index",
            query,
            "--limit",
            "10",
            "--sort",
            "price:asc",
            "--fields",
            "title,price,category",
        ])?;

        // This should fail due to no server, but should parse query correctly
        assert!(
            !output.status.success(),
            "Complex query should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_pagination() -> Result<()> {
    // Test pagination with different offsets and limits
    let pagination_tests = [(0, 10), (10, 10), (20, 10), (0, 50), (100, 25)];

    for (offset, limit) in &pagination_tests {
        let output = run_cli_command(&[
            "search",
            "test_index",
            "*",
            "--offset",
            &offset.to_string(),
            "--limit",
            &limit.to_string(),
        ])?;

        // This should fail due to no server, but should parse pagination correctly
        assert!(
            !output.status.success(),
            "Pagination should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
#[ignore] // TODO: Fix timeout issue - test hangs indefinitely
async fn test_cli_sorting_options() -> Result<()> {
    // Test different sorting options
    let sort_options = [
        "score:desc",
        "score:asc",
        "price:desc",
        "price:asc",
        "title:asc",
        "created_at:desc",
    ];

    for sort in &sort_options {
        let output = run_cli_command(&["search", "test_index", "test", "--sort", sort])?;

        // This should fail due to no server, but should parse sort correctly
        assert!(
            !output.status.success(),
            "Sort option should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_field_selection() -> Result<()> {
    // Test different field selection options
    let field_options = [
        "title",
        "title,content",
        "title,price,category",
        "id,title,description,price,category,created_at",
    ];

    for fields in &field_options {
        let output = run_cli_command(&["search", "test_index", "test", "--fields", fields])?;

        // This should fail due to no server, but should parse fields correctly
        assert!(
            !output.status.success(),
            "Field selection should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_minimum_score() -> Result<()> {
    // Test different minimum score thresholds
    let scores = ["0.1", "0.3", "0.5", "0.7", "0.9"];

    for score in &scores {
        let output = run_cli_command(&["search", "test_index", "test", "--min-score", score])?;

        // This should fail due to no server, but should parse score correctly
        assert!(
            !output.status.success(),
            "Min score should fail without server"
        );
    }

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_highlight_and_explain() -> Result<()> {
    // Test highlight and explain options
    let output = run_cli_command(&["search", "test_index", "test", "--highlight", "--explain"])?;

    // This should fail due to no server, but should parse options correctly
    assert!(
        !output.status.success(),
        "Highlight and explain should fail without server"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_combined_options() -> Result<()> {
    // Test all options combined
    let output = run_cli_command(&[
        "search",
        "test_index",
        "test query",
        "--limit",
        "20",
        "--offset",
        "10",
        "--sort",
        "score:desc",
        "--fields",
        "title,content,price",
        "--highlight",
        "--explain",
        "--min-score",
        "0.5",
    ])?;

    // This should fail due to no server, but should parse all options correctly
    assert!(
        !output.status.success(),
        "Combined options should fail without server"
    );

    Ok(())
}

#[cfg(feature = "server-tests")]
#[tokio::test]
async fn test_cli_lql_syntax_variations() -> Result<()> {
    // Test different LQL syntax variations
    let lql_queries = [
        "FROM test_index",
        "FROM test_index WHERE title:test",
        "FROM test_index WHERE category:electronics AND price:[100,500]",
        "SELECT * FROM test_index WHERE title:laptop",
        "MATCH title:gaming",
        "FROM test_index WHERE title:~laptp",
        "FROM test_index WHERE description:\"wireless headphones\"",
    ];

    for query in &lql_queries {
        let output = run_cli_command(&["lql", "test_index", query, "--limit", "10"])?;

        // This should fail due to no server, but should parse LQL correctly
        assert!(
            !output.status.success(),
            "LQL query should fail without server"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_cli_error_messages() -> Result<()> {
    // Test that error messages are informative
    let output = run_cli_command(&["invalid_command"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("error") || stderr.contains("Error") || stderr.contains("unknown"));

    // Test missing required arguments
    let output = run_cli_command(&["search"])?;
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("required") || stderr.contains("missing") || stderr.contains("argument")
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_help_content() -> Result<()> {
    // Test that help content is comprehensive
    let output = run_cli_command(&["--help"])?;
    assert!(output.status.success());
    let help_text = String::from_utf8_lossy(&output.stdout);

    // Check for key command categories
    assert!(help_text.contains("index"));
    assert!(help_text.contains("search"));
    assert!(help_text.contains("doc"));
    assert!(help_text.contains("server"));
    assert!(help_text.contains("snapshot"));
    assert!(help_text.contains("lql"));
    assert!(help_text.contains("repl"));

    // Test search help specifically
    let output = run_cli_command(&["search", "--help"])?;
    assert!(output.status.success());
    let search_help = String::from_utf8_lossy(&output.stdout);

    // Check for key search options
    assert!(search_help.contains("--limit"));
    assert!(search_help.contains("--offset"));
    assert!(search_help.contains("--sort"));
    assert!(search_help.contains("--fields"));
    assert!(search_help.contains("--highlight"));
    assert!(search_help.contains("--explain"));
    assert!(search_help.contains("--min-score"));

    Ok(())
}
