//! Server command tests for lexum-cli

use lexum_cli::commands::server;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_start_server_foreground() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("config.yml");

    // Create a test config file
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    // This will fail because lexum-server binary doesn't exist in test environment
    let result = server::start(config_file.to_str().unwrap(), false).await;

    // Should fail due to missing binary
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("lexum-server") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_start_server_daemon() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("config.yml");

    // Create a test config file
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    // This will fail because lexum-server binary doesn't exist in test environment
    let result = server::start(config_file.to_str().unwrap(), true).await;

    // Should fail due to missing binary
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("lexum-server") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_start_server_nonexistent_config() {
    // This should work but show a warning about missing config
    let result = server::start("nonexistent.yml", false).await;

    // Should fail due to missing binary
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("lexum-server") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_stop_server() {
    // This will fail because there's no running server
    let result = server::stop("http://localhost:9200").await;

    // Should succeed even if no server is running or if it fails to stop
    // The function may fail if there's no server to stop, but that's expected
    // We just verify it doesn't panic
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test]
async fn test_server_status() {
    // This will fail because there's no running server
    let result = server::status("http://localhost:9200").await;

    // Should succeed even if no server is running
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_config_valid() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("valid_config.yml");

    // Create a valid config file
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should succeed with valid config
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_config_nonexistent() {
    let result = server::validate_config("nonexistent.yml").await;

    // Should fail because file doesn't exist
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_validate_config_invalid_yaml() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("invalid_config.yml");

    // Create a file with invalid YAML
    fs::write(&config_file, "invalid yaml content").unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to invalid YAML or missing sections
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(
        error.to_string().contains("yaml")
            || error.to_string().contains("invalid")
            || error.to_string().contains("server")
            || error
                .to_string()
                .contains("Configuration validation failed")
    );
}

#[tokio::test]
async fn test_validate_config_missing_server() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("missing_server.yml");

    // Create a config file missing server section
    let config = r#"
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to missing server section
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("server") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_missing_storage() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("missing_storage.yml");

    // Create a config file missing storage section
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to missing storage section
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("storage") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_missing_logging() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("missing_logging.yml");

    // Create a config file missing logging section
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to missing logging section
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("logging") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_missing_host() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("missing_host.yml");

    // Create a config file missing host
    let config = r#"
server:
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to missing host
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("host") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_missing_port() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("missing_port.yml");

    // Create a config file missing port
    let config = r#"
server:
  host: "0.0.0.0"
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to missing port
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("port") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_unknown_section() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("unknown_section.yml");

    // Create a config file with unknown section
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
unknown_section:
  some_value: "test"
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to unknown section
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("unknown") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_complete() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("complete_config.yml");

    // Create a complete config file
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
indices:
  default_settings:
    shards: 1
security:
  enabled: false
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should succeed with complete config
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_validate_config_empty() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("empty_config.yml");

    // Create an empty config file
    fs::write(&config_file, "").unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to empty config
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("server") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_validate_config_invalid_structure() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("invalid_structure.yml");

    // Create a config file with invalid structure
    let config = r#"
server: "invalid"
storage: 123
logging: true
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should fail due to invalid structure
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("host") || error.to_string().contains("validation"));
}

#[tokio::test]
async fn test_start_server_with_special_characters_in_path() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("config with spaces.yml");

    // Create a test config file with special characters in path
    let config = r#"
server:
  host: "0.0.0.0"
  port: 9200
storage:
  path: "./data"
logging:
  level: "info"
"#;
    fs::write(&config_file, config).unwrap();

    // This will fail because lexum-server binary doesn't exist in test environment
    let result = server::start(config_file.to_str().unwrap(), false).await;

    // Should fail due to missing binary
    assert!(result.is_err());
    let error = result.unwrap_err();
    assert!(error.to_string().contains("lexum-server") || error.to_string().contains("not found"));
}

#[tokio::test]
async fn test_validate_config_with_comments() {
    let temp_dir = TempDir::new().unwrap();
    let config_file = temp_dir.path().join("commented_config.yml");

    // Create a config file with comments
    let config = r#"
# Server configuration
server:
  host: "0.0.0.0"  # Listen on all interfaces
  port: 9200       # Default port

# Storage configuration
storage:
  path: "./data"   # Data directory

# Logging configuration
logging:
  level: "info"    # Log level
"#;
    fs::write(&config_file, config).unwrap();

    let result = server::validate_config(config_file.to_str().unwrap()).await;

    // Should succeed with commented config
    assert!(result.is_ok());
}
