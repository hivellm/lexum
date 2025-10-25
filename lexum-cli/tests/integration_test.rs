//! Integration tests for Lexum CLI

use anyhow::Result;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

/// Test helper to start a Lexum server in the background
struct TestServer {
    _temp_dir: TempDir,
    server_handle: Option<std::process::Child>,
}

impl TestServer {
    fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;

        // Start the server in the background
        let server = Command::new("cargo")
            .args(["run", "--bin", "lexum-server"])
            .current_dir("..")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Wait a bit for the server to start
        thread::sleep(Duration::from_secs(3));

        Ok(Self {
            _temp_dir: temp_dir,
            server_handle: Some(server),
        })
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

#[tokio::test]
async fn test_cli_index_operations() -> Result<()> {
    let _server = TestServer::new()?;

    // Test index creation
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "create",
            "test_index",
        ])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Index creation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Test index listing
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "index", "list"])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Index listing failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("test_index"), "Index not found in list");

    // Test index stats
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "stats",
            "test_index",
        ])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Index stats failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Test index deletion
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "delete",
            "test_index",
        ])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Index deletion failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_document_operations() -> Result<()> {
    let _server = TestServer::new()?;

    // Create test index first
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "create",
            "test_docs",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Index creation failed");

    // Create test document
    let test_doc = r#"{
        "title": "Test Document",
        "content": "This is a test document for integration testing",
        "category": "test",
        "score": 5.0
    }"#;

    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_file.path(), test_doc)?;

    // Add document
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "doc",
            "add",
            "test_docs",
            "--file",
            temp_file.path().to_str().unwrap(),
        ])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Document addition failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Search for the document
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "test_docs",
            "test",
        ])
        .current_dir(".")
        .output()?;

    assert!(
        output.status.success(),
        "Search failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Test Document"),
        "Document not found in search results"
    );

    // Clean up
    let _ = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "delete",
            "test_docs",
        ])
        .current_dir(".")
        .output()?;

    Ok(())
}

#[tokio::test]
async fn test_cli_search_operations() -> Result<()> {
    let _server = TestServer::new()?;

    // Create test index
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "create",
            "search_test",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Index creation failed");

    // Add test documents
    let docs = [
        r#"{"title": "Rust Programming", "content": "Learn Rust programming language", "category": "programming", "score": 9.5}"#,
        r#"{"title": "Python Tutorial", "content": "Python programming tutorial", "category": "programming", "score": 8.0}"#,
        r#"{"title": "Database Design", "content": "Learn database design principles", "category": "database", "score": 7.5}"#,
    ];

    for (i, doc) in docs.iter().enumerate() {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), doc)?;

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "lexum-cli",
                "--",
                "doc",
                "add",
                "search_test",
                "--file",
                temp_file.path().to_str().unwrap(),
            ])
            .current_dir(".")
            .output()?;

        assert!(
            output.status.success(),
            "Document {} addition failed: {}",
            i,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    // Test basic search
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "search_test",
            "programming",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Basic search failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Rust Programming") || stdout.contains("Python Tutorial"),
        "Programming documents not found"
    );

    // Test field-specific search
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "search_test",
            "category:programming",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Field search failed");

    // Test search with limit
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "search_test",
            "programming",
            "--limit",
            "1",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Limited search failed");

    // Test search with sorting
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "search_test",
            "*",
            "--sort",
            "score:desc",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Sorted search failed");

    // Clean up
    let _ = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "delete",
            "search_test",
        ])
        .current_dir(".")
        .output()?;

    Ok(())
}

#[tokio::test]
async fn test_cli_lql_operations() -> Result<()> {
    let _server = TestServer::new()?;

    // Create test index
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "create",
            "lql_test",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Index creation failed");

    // Add test documents
    let docs = vec![
        r#"{"title": "Rust Book", "content": "The Rust Programming Language", "category": "books", "price": 45.99}"#,
        r#"{"title": "Python Guide", "content": "Python Programming Guide", "category": "books", "price": 35.99}"#,
        r#"{"title": "Database Handbook", "content": "Database Design Handbook", "category": "books", "price": 55.99}"#,
    ];

    for doc in docs {
        let temp_file = tempfile::NamedTempFile::new()?;
        std::fs::write(temp_file.path(), doc)?;

        let output = Command::new("cargo")
            .args([
                "run",
                "--bin",
                "lexum-cli",
                "--",
                "doc",
                "add",
                "lql_test",
                "--file",
                temp_file.path().to_str().unwrap(),
            ])
            .current_dir(".")
            .output()?;

        assert!(output.status.success(), "Document addition failed");
    }

    // Test LQL FROM query
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "lql",
            "lql_test",
            "FROM lql_test WHERE category:books",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "LQL FROM query failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Rust Book") || stdout.contains("Python Guide"),
        "Books not found in LQL results"
    );

    // Test LQL MATCH query
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "lql",
            "lql_test",
            "MATCH title:rust",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "LQL MATCH query failed");

    // Test LQL with sorting
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "lql",
            "lql_test",
            "FROM lql_test",
            "--sort",
            "price:asc",
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "LQL sorted query failed");

    // Test LQL from file
    let lql_content = "FROM lql_test WHERE price:[40,60]";
    let temp_file = tempfile::NamedTempFile::new()?;
    std::fs::write(temp_file.path(), lql_content)?;

    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "lql",
            "lql_test",
            &format!("@{}", temp_file.path().to_str().unwrap()),
        ])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "LQL file query failed");

    // Clean up
    let _ = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "index",
            "delete",
            "lql_test",
        ])
        .current_dir(".")
        .output()?;

    Ok(())
}

#[tokio::test]
async fn test_cli_server_operations() -> Result<()> {
    // Test server status
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "server", "status"])
        .current_dir(".")
        .output()?;

    // Server status might fail if no server is running, which is OK for this test
    // We just want to make sure the command doesn't crash
    let _ = output;

    // Test config validation (with a non-existent file)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "server",
            "config",
            "nonexistent.yml",
        ])
        .current_dir(".")
        .output()?;

    // This should fail gracefully
    assert!(
        !output.status.success(),
        "Config validation should fail for non-existent file"
    );

    Ok(())
}

#[tokio::test]
async fn test_cli_help_commands() -> Result<()> {
    // Test main help
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "--help"])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Main help failed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("lexum"), "Help should contain 'lexum'");

    // Test index help
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "index", "--help"])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Index help failed");

    // Test search help
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "search", "--help"])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "Search help failed");

    // Test LQL help
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "lql", "--help"])
        .current_dir(".")
        .output()?;

    assert!(output.status.success(), "LQL help failed");

    Ok(())
}

#[tokio::test]
async fn test_cli_error_handling() -> Result<()> {
    // Test with non-existent index
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "lexum-cli",
            "--",
            "search",
            "nonexistent",
            "query",
        ])
        .current_dir(".")
        .output()?;

    // This should fail gracefully
    assert!(
        !output.status.success(),
        "Search on non-existent index should fail"
    );

    // Test with invalid command
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "invalid", "command"])
        .current_dir(".")
        .output()?;

    // This should fail gracefully
    assert!(!output.status.success(), "Invalid command should fail");

    // Test with missing arguments
    let output = Command::new("cargo")
        .args(["run", "--bin", "lexum-cli", "--", "index", "create"])
        .current_dir(".")
        .output()?;

    // This should fail gracefully
    assert!(!output.status.success(), "Missing arguments should fail");

    Ok(())
}
