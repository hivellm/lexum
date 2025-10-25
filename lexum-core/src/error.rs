//! Error types for Lexum

use thiserror::Error;
use utoipa::ToSchema;

/// Result type alias for Lexum operations
pub type Result<T> = std::result::Result<T, Error>;

/// Lexum error types
#[derive(Error, Debug, ToSchema)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O error
    #[error("I/O error: {0}")]
    #[schema(value_type = String)]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParse(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    #[schema(value_type = String)]
    EnvVar(#[from] std::env::VarError),

    /// Not found error
    #[error("Not found: {0}")]
    NotFound(String),

    /// JSON parsing error
    #[error("JSON parsing error: {0}")]
    JsonParse(String),

    /// File watching error
    #[error("File watching error: {0}")]
    FileWatch(String),
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::YamlParse(err.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonParse(err.to_string())
    }
}

impl From<notify::Error> for Error {
    fn from(err: notify::Error) -> Self {
        Error::FileWatch(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_error_display() {
        let config_error = Error::Config("Invalid configuration".to_string());
        assert_eq!(config_error.to_string(), "Configuration error: Invalid configuration");

        let io_error = Error::Io(io::Error::new(io::ErrorKind::NotFound, "File not found"));
        assert!(io_error.to_string().contains("I/O error"));

        let yaml_error = Error::YamlParse("Invalid YAML".to_string());
        assert_eq!(yaml_error.to_string(), "YAML parsing error: Invalid YAML");

        let validation_error = Error::Validation("Invalid input".to_string());
        assert_eq!(validation_error.to_string(), "Validation error: Invalid input");

        let env_error = Error::EnvVar(std::env::VarError::NotPresent);
        assert!(env_error.to_string().contains("Environment variable error"));

        let not_found_error = Error::NotFound("Resource not found".to_string());
        assert_eq!(not_found_error.to_string(), "Not found: Resource not found");

        let json_error = Error::JsonParse("Invalid JSON".to_string());
        assert_eq!(json_error.to_string(), "JSON parsing error: Invalid JSON");

        let file_watch_error = Error::FileWatch("Watch failed".to_string());
        assert_eq!(file_watch_error.to_string(), "File watching error: Watch failed");
    }

    #[test]
    fn test_error_from_io() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "Permission denied");
        let lexum_error: Error = io_error.into();
        
        match lexum_error {
            Error::Io(_) => (),
            _ => panic!("Expected Io error variant"),
        }
    }

    #[test]
    fn test_error_from_env_var() {
        let env_error = std::env::VarError::NotPresent;
        let lexum_error: Error = env_error.into();
        
        match lexum_error {
            Error::EnvVar(_) => (),
            _ => panic!("Expected EnvVar error variant"),
        }
    }

    #[test]
    fn test_error_types_coverage() {
        // Test all error variants are accessible
        let _config = Error::Config("test".to_string());
        let _io = Error::Io(io::Error::new(io::ErrorKind::Other, "test"));
        let _yaml = Error::YamlParse("test".to_string());
        let _validation = Error::Validation("test".to_string());
        let _env = Error::EnvVar(std::env::VarError::NotPresent);
        let _not_found = Error::NotFound("test".to_string());
        let _json = Error::JsonParse("test".to_string());
        let _file_watch = Error::FileWatch("test".to_string());
    }

    #[test]
    fn test_result_type_alias() {
        fn test_function() -> Result<String> {
            Ok("success".to_string())
        }

        fn test_error_function() -> Result<String> {
            Err(Error::Config("test error".to_string()))
        }

        assert_eq!(test_function().unwrap(), "success");
        assert!(test_error_function().is_err());
    }
}
