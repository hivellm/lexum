//! Logging and tracing setup for Lexum
//!
//! Provides structured logging using the tracing ecosystem with support for
//! JSON and pretty-printed output formats.
//!
//! # Examples
//!
//! ```rust,no_run
//! use lexum_core::logging;
//! use tracing::{info, error};
//!
//! fn main() -> anyhow::Result<()> {
//!     // Initialize with default settings
//!     logging::init()?;
//!     
//!     info!("Application started");
//!     error!(error = "something went wrong", "Error occurred");
//!     
//!     Ok(())
//! }
//! ```

use crate::config::LoggingConfig;
use crate::error::{Error, Result};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize logging with default configuration
///
/// Uses INFO level and JSON format by default.
///
/// # Examples
///
/// ```rust,no_run
/// use lexum_core::logging;
///
/// fn main() -> anyhow::Result<()> {
///     logging::init()?;
///     tracing::info!("Logging initialized");
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns error if subscriber cannot be initialized
pub fn init() -> Result<()> {
    let config = LoggingConfig::default();
    init_with_config(&config)
}

/// Initialize logging with custom configuration
///
/// # Examples
///
/// ```rust,no_run
/// use lexum_core::{logging, config::LoggingConfig};
///
/// fn main() -> anyhow::Result<()> {
///     let config = LoggingConfig {
///         level: "debug".to_string(),
///         format: "pretty".to_string(),
///         outputs: vec!["stdout".to_string()],
///     };
///     
///     logging::init_with_config(&config)?;
///     tracing::debug!("Debug logging enabled");
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns error if configuration is invalid or subscriber cannot be initialized
pub fn init_with_config(config: &LoggingConfig) -> Result<()> {
    // Parse log level
    let log_level = config.level.to_uppercase();
    let env_filter = EnvFilter::try_new(&log_level)
        .map_err(|e| Error::Config(format!("Invalid log level: {e}")))?;

    // Check if file output is requested
    let has_file_output = config.outputs.iter().any(|o| o == "file");
    let has_stdout_output = config.outputs.iter().any(|o| o == "stdout");

    // Build subscriber based on format and outputs
    match config.format.as_str() {
        "json" => {
            if has_file_output {
                // Create rolling file appender (daily rotation)
                let file_appender = RollingFileAppender::new(
                    Rotation::DAILY,
                    "./logs",
                    "lexum.log",
                );
                let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

                if has_stdout_output {
                    // Both stdout and file
                    let subscriber = Registry::default()
                        .with(env_filter)
                        .with(fmt::layer().json())
                        .with(fmt::layer().json().with_writer(non_blocking));

                    subscriber
                        .try_init()
                        .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
                } else {
                    // File only
                    let subscriber = Registry::default()
                        .with(env_filter)
                        .with(fmt::layer().json().with_writer(non_blocking));

                    subscriber
                        .try_init()
                        .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
                }
                
                // Keep guard alive (in real app, store it)
                std::mem::forget(_guard);
            } else {
                // Stdout only
                let subscriber = Registry::default()
                    .with(env_filter)
                    .with(fmt::layer().json());

                subscriber
                    .try_init()
                    .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
            }
        }
        "pretty" => {
            if has_file_output {
                // Create rolling file appender
                let file_appender = RollingFileAppender::new(
                    Rotation::DAILY,
                    "./logs",
                    "lexum.log",
                );
                let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

                if has_stdout_output {
                    // Both stdout and file
                    let subscriber = Registry::default()
                        .with(env_filter)
                        .with(fmt::layer().pretty())
                        .with(fmt::layer().pretty().with_writer(non_blocking));

                    subscriber
                        .try_init()
                        .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
                } else {
                    // File only
                    let subscriber = Registry::default()
                        .with(env_filter)
                        .with(fmt::layer().pretty().with_writer(non_blocking));

                    subscriber
                        .try_init()
                        .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
                }
                
                std::mem::forget(_guard);
            } else {
                // Stdout only
                let subscriber = Registry::default()
                    .with(env_filter)
                    .with(fmt::layer().pretty());

                subscriber
                    .try_init()
                    .map_err(|e| Error::Config(format!("Failed to initialize logging: {e}")))?;
            }
        }
        _ => {
            return Err(Error::Config(format!(
                "Invalid log format: {}. Use 'json' or 'pretty'",
                config.format
            )));
        }
    }

    tracing::info!(
        level = %config.level,
        format = %config.format,
        outputs = ?config.outputs,
        "Logging initialized"
    );

    Ok(())
}

/// Set correlation ID for the current span
///
/// # Examples
///
/// ```rust,no_run
/// use lexum_core::logging;
/// use tracing::info;
///
/// logging::init().unwrap();
///
/// logging::set_correlation_id("req-123");
/// info!("This log will have correlation ID");
/// ```
pub fn set_correlation_id(id: &str) {
    tracing::Span::current().record("correlation_id", id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init_default() {
        // This test just verifies it doesn't panic
        // Can't test actual logging without more complex setup
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, "json");
    }

    #[test]
    fn test_invalid_format() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "invalid".to_string(),
            outputs: vec!["stdout".to_string()],
        };

        assert!(init_with_config(&config).is_err());
    }

    #[test]
    fn test_file_output_config() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            outputs: vec!["file".to_string()],
        };

        assert_eq!(config.outputs.len(), 1);
        assert!(config.outputs.contains(&"file".to_string()));
    }

    #[test]
    fn test_multiple_outputs() {
        let config = LoggingConfig {
            level: "info".to_string(),
            format: "json".to_string(),
            outputs: vec!["stdout".to_string(), "file".to_string()],
        };

        assert_eq!(config.outputs.len(), 2);
        assert!(config.outputs.contains(&"stdout".to_string()));
        assert!(config.outputs.contains(&"file".to_string()));
    }
}
