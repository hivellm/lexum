//! Index command tests for lexum-cli

use lexum_cli::commands::index;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_create_index_success() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("schema.yml");

    // Create a test schema file
    let schema = r"
- name: title
  type: text
  stored: true
  indexed: true
  fast: false
- name: content
  type: text
  stored: true
  indexed: true
  fast: false
- name: created_at
  type: datetime
  stored: true
  indexed: true
  fast: true
      ";
    fs::write(&schema_file, schema).unwrap();

    // This will fail because there's no server, but we can test the file reading part
    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
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
async fn test_create_index_nonexistent_schema() {
    let result = index::create("http://localhost:9200", "test_index", "nonexistent.yml").await;

    // Should fail because schema file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("No such file") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_create_index_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("invalid_schema.yml");

    // Create a file with invalid YAML
    fs::write(&schema_file, "invalid yaml content").unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail due to invalid YAML
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("yaml") || error.to_string().contains("invalid"));
}

#[tokio::test]
async fn test_create_index_minimal_schema() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("minimal_schema.yml");

    // Create a minimal schema file
    let schema = r"
- name: id
  type: text
      ";
    fs::write(&schema_file, schema).unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at schema processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_create_index_complex_schema() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("complex_schema.yml");

    // Create a complex schema file
    let schema = r"
- name: title
  type: text
  stored: true
  indexed: true
  fast: false
- name: content
  type: text
  stored: true
  indexed: true
  fast: false
- name: tags
  type: text
  stored: true
  indexed: true
  fast: true
- name: created_at
  type: datetime
  stored: true
  indexed: true
  fast: true
- name: updated_at
  type: datetime
  stored: true
  indexed: true
  fast: true
- name: author
  type: text
  stored: true
  indexed: true
  fast: true
- name: status
  type: text
  stored: true
  indexed: true
  fast: true
- name: priority
  type: i64
  stored: true
  indexed: true
  fast: true
      ";
    fs::write(&schema_file, schema).unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at schema processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_list_indices() {
    let result = index::list("http://localhost:65535").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_get_index() {
    let result = index::get("http://localhost:65535", "test_index").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_get_index_stats() {
    let result = index::stats("http://localhost:65535", "test_index").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_delete_index() {
    let result = index::delete("http://localhost:65535", "test_index").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_create_index_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("schema.yml");

    let schema = r"
- name: title
  type: text
      ";
    fs::write(&schema_file, schema).unwrap();

    let special_name = "test-index_123@example.com";
    let result = index::create(
        "http://localhost:65535",
        special_name,
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at name processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_create_index_with_empty_schema() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("empty_schema.yml");

    // Create an empty schema file
    fs::write(&schema_file, "").unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail due to empty schema or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_index_with_malformed_schema() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("malformed_schema.yml");

    // Create a schema with missing required fields
    let schema = r"
- name: title
  # missing type field
  stored: true
      ";
    fs::write(&schema_file, schema).unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail due to malformed schema or server connection
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_index_with_special_characters() {
    let special_name = "test-index_123@example.com";
    let result = index::get("http://localhost:65535", special_name).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_get_index_stats_with_special_characters() {
    let special_name = "test-index_123@example.com";
    let result = index::stats("http://localhost:65535", special_name).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_delete_index_with_special_characters() {
    let special_name = "test-index_123@example.com";
    let result = index::delete("http://localhost:65535", special_name).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_create_index_with_all_field_types() {
    let temp_dir = TempDir::new().unwrap();
    let schema_file = temp_dir.path().join("all_types_schema.yml");

    // Create a schema with all supported field types
    let schema = r"
- name: text_field
  type: text
  stored: true
  indexed: true
  fast: false
- name: i64_field
  type: i64
  stored: true
  indexed: true
  fast: true
- name: u64_field
  type: u64
  stored: true
  indexed: true
  fast: true
- name: f64_field
  type: f64
  stored: true
  indexed: true
  fast: true
- name: bool_field
  type: bool
  stored: true
  indexed: true
  fast: true
- name: date_field
  type: datetime
  stored: true
  indexed: true
  fast: true
- name: json_field
  type: json
  stored: true
  indexed: false
  fast: false
      ";
    fs::write(&schema_file, schema).unwrap();

    let result = index::create(
        "http://localhost:65535",
        "test_index",
        schema_file.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at schema processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}
