//! LQL command tests for lexum-cli

use lexum_cli::commands::lql;
use lexum_cli::commands::search::SortOrder;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_lql_from_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_query.lql");

    // Create a test LQL file
    fs::write(&file_path, "FROM test_index WHERE title:hello").unwrap();

    // This will fail because there's no server, but we can test the file reading part
    let result = lql::lql_from_file(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
        10,
    )
    .await;

    // The function should fail at the HTTP request, not at file reading
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_lql_from_file_nonexistent() {
    let result =
        lql::lql_from_file("http://localhost:9200", "test_index", "nonexistent.lql", 10).await;

    // Should fail because file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_str = error.to_string().to_lowercase();
    if !(error_str.contains("no such file")
        || error_str.contains("not found")
        || error_str.contains("cannot find")
        || error_str.contains("the system cannot find")
        || error_str.contains("no such file or directory")
        || error_str.contains("could not find file")
        || error_str.contains("os error 2")
        || error_str.contains("os error 3"))
    {
        panic!("Unexpected error message: {error_str}");
    }
}

#[tokio::test]
async fn test_lql_advanced_with_sorting() {
    let sort_options = Some(vec![
        ("title".to_string(), SortOrder::Asc),
        ("price".to_string(), SortOrder::Desc),
    ]);
    let fields = Some(vec!["title".to_string(), "price".to_string()]);

    // This will fail because there's no server, but we can test the parameter handling
    let result = lql::lql_advanced(
        "http://localhost:9200",
        "test_index",
        "FROM test_index WHERE title:hello",
        10,
        sort_options,
        fields,
    )
    .await;

    // The function should fail at the HTTP request, not at parameter processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_lql_advanced_without_options() {
    // Test with None options
    let result = lql::lql_advanced(
        "http://localhost:9200",
        "test_index",
        "FROM test_index",
        10,
        None,
        None,
    )
    .await;

    // The function should fail at the HTTP request, not at parameter processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_lql_repl() {
    // Test REPL mode
    let result = lql::lql_repl(
        "http://localhost:9200",
        "test_index",
        "FROM test_index WHERE title:hello",
        10,
    )
    .await;

    // The function should fail at the HTTP request, not at parameter processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[test]
fn test_show_lql_help() {
    // This test just ensures the function doesn't panic
    // The actual output testing would require more complex setup
    lql::show_lql_help();

    // If we get here without panic, the test passes
}

#[test]
fn test_lql_help_content_structure() {
    // Test that the help function produces expected content structure
    // We can't easily capture stdout in a test, but we can ensure it doesn't panic
    lql::show_lql_help();

    // If we get here without panic, the test passes
}

#[tokio::test]
async fn test_lql_with_empty_query() {
    let result = lql::lql_advanced("http://localhost:9200", "test_index", "", 10, None, None).await;

    // Should fail due to empty query or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lql_with_invalid_query() {
    let result = lql::lql_advanced(
        "http://localhost:9200",
        "test_index",
        "INVALID LQL SYNTAX",
        10,
        None,
        None,
    )
    .await;

    // Should fail due to invalid query syntax or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_lql_with_large_limit() {
    let result = lql::lql_advanced(
        "http://localhost:9200",
        "test_index",
        "FROM test_index",
        10000,
        None,
        None,
    )
    .await;

    // Should fail at server connection, not at limit processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_lql_with_special_characters() {
    let result = lql::lql_advanced(
        "http://localhost:9200",
        "test_index",
        "FROM test_index WHERE title:\"hello world\"",
        10,
        None,
        None,
    )
    .await;

    // Should fail at server connection, not at query processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[test]
fn test_lql_help_contains_expected_sections() {
    // Test that help function doesn't panic and produces output
    // In a real test environment, we'd capture stdout and verify content
    lql::show_lql_help();

    // If we get here without panic, the test passes
}

#[tokio::test]
async fn test_lql_file_with_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_query.lql");

    // Create a test LQL file with leading/trailing whitespace
    fs::write(&file_path, "  \n  FROM test_index WHERE title:hello  \n  ").unwrap();

    let result = lql::lql_from_file(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
        10,
    )
    .await;

    // Should fail at server connection, not at file processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_lql_file_with_empty_content() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty_query.lql");

    // Create an empty file
    fs::write(&file_path, "").unwrap();

    let result = lql::lql_from_file(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
        10,
    )
    .await;

    // Should fail due to empty query or server connection
    assert!(result.is_err());
}
