//! Configuration management for Lexum
//!
//! Supports loading configuration from YAML files with environment variable overrides.
//!
//! # Examples
//!
//! ```rust,no_run
//! use lexum_core::config::Config;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load from file
//!     let config = Config::from_file("config.yml").await?;
//!     
//!     // Or use defaults
//!     let config = Config::default();
//!     
//!     println!("HTTP port: {}", config.network.http_port);
//!     Ok(())
//! }
//! ```

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Cluster configuration
    #[serde(default)]
    pub cluster: ClusterConfig,

    /// Node configuration
    #[serde(default)]
    pub node: NodeConfig,

    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,

    /// Storage paths
    #[serde(default)]
    pub path: PathConfig,

    /// Logging configuration
    #[serde(default)]
    pub logging: LoggingConfig,
}

/// Cluster configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Cluster name
    pub name: String,

    /// Initial master nodes for bootstrap
    #[serde(default)]
    pub initial_master_nodes: Vec<String>,
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            name: "lexum-cluster".to_string(),
            initial_master_nodes: vec!["node-1".to_string()],
        }
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node name
    pub name: String,

    /// Node roles (master, data, ingest, coordinator)
    #[serde(default = "default_node_roles")]
    pub roles: Vec<String>,
}

fn default_node_roles() -> Vec<String> {
    vec![
        "master".to_string(),
        "data".to_string(),
        "ingest".to_string(),
    ]
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            name: hostname::get()
                .ok()
                .and_then(|h| h.into_string().ok())
                .unwrap_or_else(|| "lexum-node-1".to_string()),
            roles: default_node_roles(),
        }
    }
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Host to bind HTTP server
    #[serde(default = "default_host")]
    pub host: String,

    /// HTTP port
    #[serde(default = "default_http_port")]
    pub http_port: u16,

    /// Transport port for inter-node communication
    #[serde(default = "default_transport_port")]
    pub transport_port: u16,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    9200
}

fn default_transport_port() -> u16 {
    9300
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            host: default_host(),
            http_port: default_http_port(),
            transport_port: default_transport_port(),
        }
    }
}

/// Storage path configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathConfig {
    /// Data directory
    #[serde(default = "default_data_path")]
    pub data: String,

    /// Logs directory
    #[serde(default = "default_logs_path")]
    pub logs: String,
}

fn default_data_path() -> String {
    "./data".to_string()
}

fn default_logs_path() -> String {
    "./logs".to_string()
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            data: default_data_path(),
            logs: default_logs_path(),
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (json or pretty)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Output targets
    #[serde(default = "default_log_outputs")]
    pub outputs: Vec<String>,
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

fn default_log_outputs() -> Vec<String> {
    vec!["stdout".to_string()]
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            outputs: default_log_outputs(),
        }
    }
}

impl Config {
    /// Load configuration from YAML file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::config::Config;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config = Config::from_file("config.yml").await?;
    ///     println!("Loaded config: {:?}", config);
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or YAML is invalid
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path).await?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Apply environment variable overrides
    fn apply_env_overrides(&mut self) {
        // Cluster
        if let Ok(name) = std::env::var("LEXUM_CLUSTER_NAME") {
            self.cluster.name = name;
        }

        // Node
        if let Ok(name) = std::env::var("LEXUM_NODE_NAME") {
            self.node.name = name;
        }
        if let Ok(roles) = std::env::var("LEXUM_NODE_ROLES") {
            self.node.roles = roles.split(',').map(|s| s.trim().to_string()).collect();
        }

        // Network
        if let Ok(host) = std::env::var("LEXUM_NETWORK_HOST") {
            self.network.host = host;
        }
        if let Ok(port) = std::env::var("LEXUM_HTTP_PORT") {
            if let Ok(port) = port.parse() {
                self.network.http_port = port;
            }
        }
        if let Ok(port) = std::env::var("LEXUM_TRANSPORT_PORT") {
            if let Ok(port) = port.parse() {
                self.network.transport_port = port;
            }
        }

        // Paths
        if let Ok(data) = std::env::var("LEXUM_DATA_PATH") {
            self.path.data = data;
        }
        if let Ok(logs) = std::env::var("LEXUM_LOGS_PATH") {
            self.path.logs = logs;
        }

        // Logging
        if let Ok(level) = std::env::var("LEXUM_LOG_LEVEL") {
            self.logging.level = level;
        }
        if let Ok(format) = std::env::var("LEXUM_LOG_FORMAT") {
            self.logging.format = format;
        }
    }

    /// Validate configuration
    fn validate(&self) -> Result<()> {
        // Validate HTTP port
        if self.network.http_port == 0 {
            return Err(Error::Validation("HTTP port cannot be 0".to_string()));
        }

        // Validate transport port
        if self.network.transport_port == 0 {
            return Err(Error::Validation("Transport port cannot be 0".to_string()));
        }

        // Validate ports are different
        if self.network.http_port == self.network.transport_port {
            return Err(Error::Validation(
                "HTTP and transport ports must be different".to_string(),
            ));
        }

        // Validate cluster name not empty
        if self.cluster.name.is_empty() {
            return Err(Error::Validation(
                "Cluster name cannot be empty".to_string(),
            ));
        }

        // Validate node name not empty
        if self.node.name.is_empty() {
            return Err(Error::Validation("Node name cannot be empty".to_string()));
        }

        // Validate node roles not empty
        if self.node.roles.is_empty() {
            return Err(Error::Validation(
                "Node must have at least one role".to_string(),
            ));
        }

        // Validate log level
        let valid_levels = ["trace", "debug", "info", "warn", "error"];
        if !valid_levels.contains(&self.logging.level.as_str()) {
            return Err(Error::Validation(format!(
                "Invalid log level: {}. Must be one of: trace, debug, info, warn, error",
                self.logging.level
            )));
        }

        // Validate log format
        let valid_formats = ["json", "pretty"];
        if !valid_formats.contains(&self.logging.format.as_str()) {
            return Err(Error::Validation(format!(
                "Invalid log format: {}. Must be one of: json, pretty",
                self.logging.format
            )));
        }

        Ok(())
    }

    /// Save configuration to YAML file
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::config::Config;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config = Config::default();
    ///     config.to_file("config.yml").await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let yaml = serde_yaml::to_string(self)
            .map_err(|e| Error::Config(format!("Failed to serialize config: {e}")))?;
        fs::write(path, yaml).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.network.http_port, 9200);
        assert_eq!(config.network.transport_port, 9300);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_validation_invalid_port() {
        let mut config = Config::default();
        config.network.http_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_same_ports() {
        let mut config = Config::default();
        config.network.http_port = 9200;
        config.network.transport_port = 9200;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_log_level() {
        let mut config = Config::default();
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[tokio::test]
    async fn test_serialize_deserialize() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.network.http_port, parsed.network.http_port);
    }
}
