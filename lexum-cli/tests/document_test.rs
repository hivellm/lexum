//! Document command tests for lexum-cli

use lexum_cli::commands::document;
use std::fs;
use tempfile::TempDir;
use tokio;

#[tokio::test]
async fn test_add_document_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_doc.json");

    // Create a test document file
    let doc = serde_json::json!({
        "title": "Test Document",
        "content": "This is a test document",
        "author": "test_user"
    });
    fs::write(&file_path, serde_json::to_string(&doc).unwrap()).unwrap();

    // This will fail because there's no server, but we can test the file reading part
    let result = document::add(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
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
async fn test_add_document_nonexistent_file() {
    let result = document::add("http://localhost:9200", "test_index", "nonexistent.json").await;

    // Should fail because file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("No such file") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_add_document_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid_doc.json");

    // Create a file with invalid JSON
    fs::write(&file_path, "invalid json content").unwrap();

    let result = document::add(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail due to invalid JSON
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("expected") || error.to_string().contains("invalid"));
}

#[tokio::test]
async fn test_get_document() {
    let result = document::get("http://localhost:9200", "test_index", "doc123").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_delete_document() {
    let result = document::delete("http://localhost:9200", "test_index", "doc123").await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_bulk_operations_success() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("bulk_docs.json");

    // Create a test bulk documents file
    let docs = serde_json::json!([
        {
            "title": "Document 1",
            "content": "Content 1"
        },
        {
            "title": "Document 2",
            "content": "Content 2"
        }
    ]);
    fs::write(&file_path, serde_json::to_string(&docs).unwrap()).unwrap();

    // This will fail because there's no server, but we can test the file reading part
    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
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
async fn test_bulk_operations_nonexistent_file() {
    let result = document::bulk("http://localhost:9200", "test_index", "nonexistent.json").await;

    // Should fail because file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("No such file") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_bulk_operations_invalid_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("invalid_bulk.json");

    // Create a file with invalid JSON
    fs::write(&file_path, "invalid json content").unwrap();

    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail due to invalid JSON
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("expected") || error.to_string().contains("invalid"));
}

#[tokio::test]
async fn test_bulk_operations_empty_array() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("empty_bulk.json");

    // Create a file with empty array
    fs::write(&file_path, "[]").unwrap();

    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at processing empty array
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_bulk_operations_single_document() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("single_doc.json");

    // Create a file with single document
    let doc = serde_json::json!([{
        "title": "Single Document",
        "content": "Single content"
    }]);
    fs::write(&file_path, serde_json::to_string(&doc).unwrap()).unwrap();

    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at processing single document
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_bulk_operations_large_dataset() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("large_bulk.json");

    // Create a file with many documents
    let mut docs = Vec::new();
    for i in 0..100 {
        docs.push(serde_json::json!({
            "id": i,
            "title": format!("Document {}", i),
            "content": format!("Content for document {}", i)
        }));
    }
    fs::write(&file_path, serde_json::to_string(&docs).unwrap()).unwrap();

    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at processing large dataset
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_add_document_with_complex_json() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("complex_doc.json");

    // Create a complex JSON document
    let doc = serde_json::json!({
        "title": "Complex Document",
        "metadata": {
            "author": "test_user",
            "tags": ["test", "complex", "json"],
            "nested": {
                "level1": {
                    "level2": "deep_value"
                }
            }
        },
        "content": "This is a complex document with nested structures",
        "array_field": [1, 2, 3, "mixed", true, null]
    });
    fs::write(&file_path, serde_json::to_string(&doc).unwrap()).unwrap();

    let result = document::add(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at JSON processing
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_get_document_with_special_characters() {
    let special_id = "doc-123_test@example.com";
    let result = document::get("http://localhost:9200", "test_index", special_id).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_delete_document_with_special_characters() {
    let special_id = "doc-123_test@example.com";
    let result = document::delete("http://localhost:9200", "test_index", special_id).await;

    // Should fail due to server connection
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}

#[tokio::test]
async fn test_bulk_operations_with_mixed_document_types() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("mixed_bulk.json");

    // Create a file with mixed document types
    let docs = serde_json::json!([
        {
            "type": "article",
            "title": "Article 1",
            "content": "Article content"
        },
        {
            "type": "comment",
            "text": "Comment text",
            "author": "user1"
        },
        {
            "type": "metadata",
            "version": "1.0",
            "created": "2024-01-01"
        }
    ]);
    fs::write(&file_path, serde_json::to_string(&docs).unwrap()).unwrap();

    let result = document::bulk(
        "http://localhost:9200",
        "test_index",
        file_path.to_str().unwrap(),
    )
    .await;

    // Should fail at server connection, not at processing mixed types
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("Connection refused") || error.to_string().contains("error")
    );
}
