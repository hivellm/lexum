//! Error types for Lexum

use thiserror::Error;

/// Result type alias for Lexum operations
pub type Result<T> = std::result::Result<T, Error>;

/// Lexum error types
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// YAML parsing error
    #[error("YAML parsing error: {0}")]
    YamlParse(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Environment variable error
    #[error("Environment variable error: {0}")]
    EnvVar(#[from] std::env::VarError),
}

impl From<serde_yaml::Error> for Error {
    fn from(err: serde_yaml::Error) -> Self {
        Error::YamlParse(err.to_string())
    }
}
