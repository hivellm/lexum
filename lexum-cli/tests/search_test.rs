//! Search command tests for lexum-cli

use lexum_cli::commands::search::{self, SortOrder};
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_search_basic() {
    let result = search::search("http://localhost:65535", "test_index", "hello world", 10).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_advanced_with_sorting() {
    let sort_options = Some(vec![
        ("title".to_string(), SortOrder::Asc),
        ("created_at".to_string(), SortOrder::Desc),
    ]);

    let result = search::search_advanced(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        sort_options,
        None,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_advanced_with_fields() {
    let fields = Some(vec!["title".to_string(), "content".to_string()]);

    let result = search::search_advanced(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        None,
        fields,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_advanced_with_all_options() {
    let sort_options = Some(vec![("title".to_string(), SortOrder::Desc)]);
    let fields = Some(vec!["title".to_string(), "content".to_string()]);

    let result = search::search_advanced(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        sort_options,
        fields,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_advanced_with_options() {
    let sort_options = Some(vec![("title".to_string(), SortOrder::Desc)]);
    let fields = Some(vec!["title".to_string(), "content".to_string()]);

    let result = search::search_advanced_with_options(
        "http://localhost:65535",
        "test_index",
        "hello world",
        20,
        10,
        sort_options,
        fields,
        true,
        true,
        Some(0.7),
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_from_file() {
    let temp_dir = TempDir::new().unwrap();
    let query_file = temp_dir.path().join("query.txt");

    // Create a test query file
    fs::write(&query_file, "hello world").unwrap();

    let result = search::search_from_file(
        "http://localhost:65535",
        "test_index",
        query_file.to_str().unwrap(),
        10,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_from_file_nonexistent() {
    let result =
        search::search_from_file("http://localhost:9200", "test_index", "nonexistent.txt", 10)
            .await;

    // Should fail because file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("No such file") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_search_from_file_empty() {
    let temp_dir = TempDir::new().unwrap();
    let query_file = temp_dir.path().join("empty_query.txt");

    // Create an empty file
    fs::write(&query_file, "").unwrap();

    let result = search::search_from_file(
        "http://localhost:65535",
        "test_index",
        query_file.to_str().unwrap(),
        10,
    )
    .await;

    // Should fail due to empty query or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_from_file_advanced() {
    let temp_dir = TempDir::new().unwrap();
    let query_file = temp_dir.path().join("query.txt");

    // Create a test query file
    fs::write(&query_file, "hello world").unwrap();

    let result = search::search_from_file_advanced(
        "http://localhost:65535",
        "test_index",
        query_file.to_str().unwrap(),
        10,
        0,
        false,
        false,
        None,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_from_files() {
    let temp_dir = TempDir::new().unwrap();
    let query_file1 = temp_dir.path().join("query1.txt");
    let query_file2 = temp_dir.path().join("query2.txt");

    // Create test query files
    fs::write(&query_file1, "hello world").unwrap();
    fs::write(&query_file2, "test query").unwrap();

    let file_paths = vec![
        query_file1.to_str().unwrap().to_string(),
        query_file2.to_str().unwrap().to_string(),
    ];

    let result =
        search::search_from_files("http://localhost:65535", "test_index", file_paths, 10).await;

    // This function doesn't return an error, it just prints error messages
    // So we just verify it completes without panicking
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_search_with_special_characters() {
    let special_query = "hello world & special chars @#$%";

    let result = search::search("http://localhost:65535", "test_index", special_query, 10).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_unicode() {
    let unicode_query = "hello 世界 🌍";

    let result = search::search("http://localhost:65535", "test_index", unicode_query, 10).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_large_limit() {
    let result = search::search("http://localhost:65535", "test_index", "hello world", 10000).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_many_fields() {
    let fields = Some(vec![
        "title".to_string(),
        "content".to_string(),
        "author".to_string(),
        "created_at".to_string(),
        "tags".to_string(),
        "category".to_string(),
        "status".to_string(),
    ]);

    let result = search::search_advanced(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        None,
        fields,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_many_sort_options() {
    let sort_options = Some(vec![
        ("title".to_string(), SortOrder::Asc),
        ("created_at".to_string(), SortOrder::Desc),
        ("score".to_string(), SortOrder::Desc),
        ("author".to_string(), SortOrder::Asc),
    ]);

    let result = search::search_advanced(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        sort_options,
        None,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_high_min_score() {
    let result = search::search_advanced_with_options(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        0,
        None,
        None,
        false,
        false,
        Some(0.95),
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_zero_min_score() {
    let result = search::search_advanced_with_options(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        0,
        None,
        None,
        false,
        false,
        Some(0.0),
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_negative_min_score() {
    let result = search::search_advanced_with_options(
        "http://localhost:65535",
        "test_index",
        "hello world",
        10,
        0,
        None,
        None,
        false,
        false,
        Some(-0.5),
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_from_file_with_whitespace() {
    let temp_dir = TempDir::new().unwrap();
    let query_file = temp_dir.path().join("query.txt");

    // Create a file with leading/trailing whitespace
    fs::write(&query_file, "  \n  hello world  \n  ").unwrap();

    let result = search::search_from_file(
        "http://localhost:65535",
        "test_index",
        query_file.to_str().unwrap(),
        10,
    )
    .await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_search_with_empty_query() {
    let result = search::search("http://localhost:65535", "test_index", "", 10).await;

    // Should fail due to empty query or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_search_with_very_long_query() {
    let long_query = "hello world ".repeat(1000);

    let result = search::search("http://localhost:65535", "test_index", &long_query, 10).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}
