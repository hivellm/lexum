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
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

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

    /// Snapshot repository configuration
    #[serde(default)]
    pub snapshots: SnapshotConfig,
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

/// Snapshot configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    /// Default snapshot repository settings
    #[serde(default)]
    pub repositories: Vec<SnapshotRepositoryConfig>,

    /// Snapshot storage path
    #[serde(default = "default_snapshot_path")]
    pub path: String,

    /// Maximum number of snapshots to keep
    #[serde(default = "default_max_snapshots")]
    pub max_snapshots: usize,

    /// Snapshot compression enabled
    #[serde(default = "default_compression_enabled")]
    pub compression_enabled: bool,
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

fn default_snapshot_path() -> String {
    "./snapshots".to_string()
}

fn default_max_snapshots() -> usize {
    100
}

fn default_compression_enabled() -> bool {
    true
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

/// Snapshot repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRepositoryConfig {
    /// Repository name
    pub name: String,

    /// Repository type (fs, s3, gcs, azure)
    #[serde(default = "default_repository_type")]
    pub repository_type: String,

    /// Repository settings
    #[serde(default)]
    pub settings: SnapshotRepositorySettings,
}

fn default_repository_type() -> String {
    "fs".to_string()
}

impl Default for SnapshotRepositoryConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            repository_type: default_repository_type(),
            settings: SnapshotRepositorySettings::default(),
        }
    }
}

impl SnapshotRepositoryConfig {
    /// Validate snapshot repository configuration
    pub fn validate(&self) -> Result<()> {
        // Validate repository name
        if self.name.is_empty() {
            return Err(Error::Validation(
                "Repository name cannot be empty".to_string(),
            ));
        }

        // Validate repository name format (alphanumeric, hyphens, underscores)
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(Error::Validation(format!(
                "Repository name '{}' contains invalid characters. Only alphanumeric characters, hyphens, and underscores are allowed",
                self.name
            )));
        }

        // Validate repository type
        let valid_types = ["fs", "s3", "gcs", "azure"];
        if !valid_types.contains(&self.repository_type.as_str()) {
            return Err(Error::Validation(format!(
                "Invalid repository type: {}. Must be one of: {}",
                self.repository_type,
                valid_types.join(", ")
            )));
        }

        // Validate settings
        self.settings.validate(&self.repository_type)?;

        Ok(())
    }
}

/// Snapshot repository settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRepositorySettings {
    /// Location for filesystem repositories (path) or cloud repositories (bucket/container)
    #[serde(default = "default_location")]
    pub location: String,

    /// Compress snapshots
    #[serde(default = "default_compress")]
    pub compress: bool,

    /// Chunk size for snapshots
    #[serde(default = "default_chunk_size")]
    pub chunk_size: String,

    /// Maximum restore bytes per second
    #[serde(default = "default_max_restore_bytes_per_sec")]
    pub max_restore_bytes_per_sec: String,

    /// Maximum snapshot bytes per second
    #[serde(default = "default_max_snapshot_bytes_per_sec")]
    pub max_snapshot_bytes_per_sec: String,

    /// Readonly repository
    #[serde(default = "default_readonly")]
    pub readonly: bool,

    /// AWS S3 specific settings
    #[serde(default)]
    pub s3_settings: Option<S3Settings>,

    /// Google Cloud Storage specific settings
    #[serde(default)]
    pub gcs_settings: Option<GcsSettings>,

    /// Azure Blob Storage specific settings
    #[serde(default)]
    pub azure_settings: Option<AzureSettings>,

    /// Maximum number of snapshots to keep in this repository
    #[serde(default = "default_max_snapshots_per_repo")]
    pub max_snapshots: usize,

    /// Snapshot retention policy
    #[serde(default)]
    pub retention_policy: RetentionPolicy,
}

fn default_location() -> String {
    "./snapshots".to_string()
}

fn default_compress() -> bool {
    true
}

fn default_chunk_size() -> String {
    "1gb".to_string()
}

fn default_max_restore_bytes_per_sec() -> String {
    "40mb".to_string()
}

fn default_max_snapshot_bytes_per_sec() -> String {
    "40mb".to_string()
}

fn default_readonly() -> bool {
    false
}

fn default_max_snapshots_per_repo() -> usize {
    1000
}

/// AWS S3 specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Settings {
    /// AWS region
    #[serde(default = "default_aws_region")]
    pub region: String,

    /// AWS access key ID
    #[serde(default)]
    pub access_key_id: Option<String>,

    /// AWS secret access key
    #[serde(default)]
    pub secret_access_key: Option<String>,

    /// S3 endpoint URL (for S3-compatible services)
    #[serde(default)]
    pub endpoint: Option<String>,

    /// S3 path style access
    #[serde(default = "default_path_style")]
    pub path_style: bool,

    /// S3 server-side encryption
    #[serde(default)]
    pub server_side_encryption: Option<String>,
}

fn default_aws_region() -> String {
    "us-east-1".to_string()
}

fn default_path_style() -> bool {
    false
}

/// Google Cloud Storage specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GcsSettings {
    /// GCS project ID
    #[serde(default)]
    pub project_id: Option<String>,

    /// GCS service account key file path
    #[serde(default)]
    pub service_account_key_file: Option<String>,

    /// GCS application credentials JSON
    #[serde(default)]
    pub application_credentials: Option<String>,
}

/// Azure Blob Storage specific settings
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AzureSettings {
    /// Azure storage account name
    #[serde(default)]
    pub account_name: Option<String>,

    /// Azure storage account key
    #[serde(default)]
    pub account_key: Option<String>,

    /// Azure storage connection string
    #[serde(default)]
    pub connection_string: Option<String>,

    /// Azure storage container name
    #[serde(default)]
    pub container_name: Option<String>,
}

/// Snapshot retention policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Keep snapshots for this many days
    #[serde(default = "default_retention_days")]
    pub keep_for_days: Option<u32>,

    /// Keep this many snapshots regardless of age
    #[serde(default = "default_keep_count")]
    pub keep_count: Option<usize>,

    /// Delete snapshots older than this many days
    #[serde(default = "default_delete_after_days")]
    pub delete_after_days: Option<u32>,
}

#[allow(clippy::unnecessary_wraps)]
fn default_retention_days() -> Option<u32> {
    Some(30)
}

#[allow(clippy::unnecessary_wraps)]
fn default_keep_count() -> Option<usize> {
    Some(10)
}

#[allow(clippy::unnecessary_wraps)]
fn default_delete_after_days() -> Option<u32> {
    Some(90)
}

impl Default for SnapshotRepositorySettings {
    fn default() -> Self {
        Self {
            location: default_location(),
            compress: default_compress(),
            chunk_size: default_chunk_size(),
            max_restore_bytes_per_sec: default_max_restore_bytes_per_sec(),
            max_snapshot_bytes_per_sec: default_max_snapshot_bytes_per_sec(),
            readonly: default_readonly(),
            s3_settings: None,
            gcs_settings: None,
            azure_settings: None,
            max_snapshots: default_max_snapshots_per_repo(),
            retention_policy: RetentionPolicy::default(),
        }
    }
}

impl Default for S3Settings {
    fn default() -> Self {
        Self {
            region: default_aws_region(),
            access_key_id: None,
            secret_access_key: None,
            endpoint: None,
            path_style: default_path_style(),
            server_side_encryption: None,
        }
    }
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            keep_for_days: default_retention_days(),
            keep_count: default_keep_count(),
            delete_after_days: default_delete_after_days(),
        }
    }
}

impl SnapshotRepositorySettings {
    /// Validate snapshot repository settings
    pub fn validate(&self, repository_type: &str) -> Result<()> {
        // Validate location based on repository type
        match repository_type {
            "fs" => {
                if self.location.is_empty() {
                    return Err(Error::Validation(
                        "Filesystem repository location cannot be empty".to_string(),
                    ));
                }
            }
            "s3" => {
                if self.location.is_empty() {
                    return Err(Error::Validation(
                        "S3 repository bucket name cannot be empty".to_string(),
                    ));
                }
                if let Some(ref s3_settings) = self.s3_settings {
                    s3_settings.validate()?;
                }
            }
            "gcs" => {
                if self.location.is_empty() {
                    return Err(Error::Validation(
                        "GCS repository bucket name cannot be empty".to_string(),
                    ));
                }
                if let Some(ref gcs_settings) = self.gcs_settings {
                    gcs_settings.validate()?;
                }
            }
            "azure" => {
                if self.location.is_empty() {
                    return Err(Error::Validation(
                        "Azure repository container name cannot be empty".to_string(),
                    ));
                }
                if let Some(ref azure_settings) = self.azure_settings {
                    azure_settings.validate()?;
                }
            }
            _ => {
                return Err(Error::Validation(format!(
                    "Unknown repository type: {repository_type}"
                )));
            }
        }

        // Validate chunk size format (e.g., "1gb", "512mb", "1024kb")
        if !Self::is_valid_size_string(&self.chunk_size) {
            return Err(Error::Validation(format!(
                "Invalid chunk size format: {}. Must be in format like '1gb', '512mb', '1024kb'",
                self.chunk_size
            )));
        }

        // Validate max restore bytes per sec format
        if !Self::is_valid_size_string(&self.max_restore_bytes_per_sec) {
            return Err(Error::Validation(format!(
                "Invalid max restore bytes per sec format: {}. Must be in format like '40mb', '1gb'",
                self.max_restore_bytes_per_sec
            )));
        }

        // Validate max snapshot bytes per sec format
        if !Self::is_valid_size_string(&self.max_snapshot_bytes_per_sec) {
            return Err(Error::Validation(format!(
                "Invalid max snapshot bytes per sec format: {}. Must be in format like '40mb', '1gb'",
                self.max_snapshot_bytes_per_sec
            )));
        }

        // Validate max snapshots
        if self.max_snapshots == 0 {
            return Err(Error::Validation(
                "Maximum snapshots per repository must be greater than 0".to_string(),
            ));
        }

        if self.max_snapshots > 100000 {
            return Err(Error::Validation(
                "Maximum snapshots per repository cannot exceed 100000".to_string(),
            ));
        }

        // Validate retention policy
        self.retention_policy.validate()?;

        Ok(())
    }

    /// Check if a string represents a valid size format
    fn is_valid_size_string(size_str: &str) -> bool {
        if size_str.is_empty() {
            return false;
        }

        // Check if it ends with a valid unit
        let valid_units = ["b", "kb", "mb", "gb", "tb"];
        let has_valid_unit = valid_units
            .iter()
            .any(|unit| size_str.to_lowercase().ends_with(unit));

        if !has_valid_unit {
            return false;
        }

        // Extract the numeric part
        let numeric_part = size_str
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>();

        // Must have at least one digit
        !numeric_part.is_empty()
    }
}

impl S3Settings {
    /// Validate S3 settings
    pub fn validate(&self) -> Result<()> {
        if self.region.is_empty() {
            return Err(Error::Validation("AWS region cannot be empty".to_string()));
        }

        // Validate region format (e.g., "us-east-1", "eu-west-1")
        if !self.region.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(Error::Validation(format!(
                "Invalid AWS region format: {}. Must contain only alphanumeric characters and hyphens",
                self.region
            )));
        }

        Ok(())
    }
}

impl GcsSettings {
    /// Validate GCS settings
    pub fn validate(&self) -> Result<()> {
        // At least one authentication method must be provided
        if self.service_account_key_file.is_none() && self.application_credentials.is_none() {
            return Err(Error::Validation(
                "GCS authentication must be configured: either service_account_key_file or application_credentials must be set".to_string(),
            ));
        }

        Ok(())
    }
}

impl AzureSettings {
    /// Validate Azure settings
    pub fn validate(&self) -> Result<()> {
        // At least one authentication method must be provided
        if self.account_name.is_none() && self.connection_string.is_none() {
            return Err(Error::Validation(
                "Azure authentication must be configured: either account_name with account_key or connection_string must be set".to_string(),
            ));
        }

        // If account_name is provided, account_key must also be provided
        if self.account_name.is_some() && self.account_key.is_none() {
            return Err(Error::Validation(
                "Azure account_key must be provided when account_name is set".to_string(),
            ));
        }

        Ok(())
    }
}

impl RetentionPolicy {
    /// Validate retention policy
    pub fn validate(&self) -> Result<()> {
        // At least one retention rule must be specified
        if self.keep_for_days.is_none()
            && self.keep_count.is_none()
            && self.delete_after_days.is_none()
        {
            return Err(Error::Validation(
                "At least one retention policy rule must be specified".to_string(),
            ));
        }

        // Validate keep_for_days
        if let Some(days) = self.keep_for_days {
            if days == 0 {
                return Err(Error::Validation(
                    "Keep for days must be greater than 0".to_string(),
                ));
            }
            if days > 3650 {
                // 10 years
                return Err(Error::Validation(
                    "Keep for days cannot exceed 3650 (10 years)".to_string(),
                ));
            }
        }

        // Validate keep_count
        if let Some(count) = self.keep_count {
            if count == 0 {
                return Err(Error::Validation(
                    "Keep count must be greater than 0".to_string(),
                ));
            }
            if count > 10000 {
                return Err(Error::Validation(
                    "Keep count cannot exceed 10000".to_string(),
                ));
            }
        }

        // Validate delete_after_days
        if let Some(days) = self.delete_after_days {
            if days == 0 {
                return Err(Error::Validation(
                    "Delete after days must be greater than 0".to_string(),
                ));
            }
            if days > 3650 {
                // 10 years
                return Err(Error::Validation(
                    "Delete after days cannot exceed 3650 (10 years)".to_string(),
                ));
            }
        }

        // Validate logical consistency
        if let (Some(keep_days), Some(delete_days)) = (self.keep_for_days, self.delete_after_days) {
            if keep_days > delete_days {
                return Err(Error::Validation(
                    "Keep for days cannot be greater than delete after days".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self {
            repositories: vec![SnapshotRepositoryConfig::default()],
            path: default_snapshot_path(),
            max_snapshots: default_max_snapshots(),
            compression_enabled: default_compression_enabled(),
        }
    }
}

impl SnapshotConfig {
    /// Validate snapshot configuration
    pub fn validate(&self) -> Result<()> {
        // Validate snapshot path is not empty
        if self.path.is_empty() {
            return Err(Error::Validation(
                "Snapshot path cannot be empty".to_string(),
            ));
        }

        // Validate max snapshots is reasonable
        if self.max_snapshots == 0 {
            return Err(Error::Validation(
                "Maximum snapshots must be greater than 0".to_string(),
            ));
        }

        if self.max_snapshots > 10000 {
            return Err(Error::Validation(
                "Maximum snapshots cannot exceed 10000".to_string(),
            ));
        }

        // Validate repositories
        if self.repositories.is_empty() {
            return Err(Error::Validation(
                "At least one snapshot repository must be configured".to_string(),
            ));
        }

        // Check for duplicate repository names
        let mut names = std::collections::HashSet::new();
        for repo in &self.repositories {
            if !names.insert(&repo.name) {
                return Err(Error::Validation(format!(
                    "Duplicate repository name: {}",
                    repo.name
                )));
            }
            repo.validate()?;
        }

        Ok(())
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

    /// Load configuration from YAML file with hot-reload support
    ///
    /// This method creates a configuration manager that watches the config file
    /// for changes and automatically reloads the configuration when the file is modified.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use lexum_core::config::Config;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let config_manager = Config::from_file_with_hot_reload("config.yml").await?;
    ///     
    ///     // Get current config
    ///     let config = config_manager.get_config().await;
    ///     println!("Current config: {:?}", config);
    ///     
    ///     // Listen for config changes
    ///     let mut rx = config_manager.subscribe();
    ///     while let Ok(new_config) = rx.recv().await {
    ///         println!("Config reloaded: {:?}", new_config);
    ///     }
    ///     
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read, YAML is invalid, or file watcher cannot be started
    pub async fn from_file_with_hot_reload(path: impl AsRef<Path>) -> Result<ConfigManager> {
        let path = path.as_ref().to_path_buf();

        // Load initial configuration
        let config = Self::from_file(&path).await?;

        // Create configuration manager
        let manager = ConfigManager::new(config, path).await?;

        Ok(manager)
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

        // Snapshots
        if let Ok(path) = std::env::var("LEXUM_SNAPSHOT_PATH") {
            self.snapshots.path = path;
        }
        if let Ok(max) = std::env::var("LEXUM_MAX_SNAPSHOTS") {
            if let Ok(max) = max.parse() {
                self.snapshots.max_snapshots = max;
            }
        }
        if let Ok(compression) = std::env::var("LEXUM_SNAPSHOT_COMPRESSION") {
            self.snapshots.compression_enabled = compression.parse().unwrap_or(true);
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

        // Validate snapshot configuration
        self.snapshots.validate()?;

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

/// Configuration manager with hot-reload support
#[derive(Debug)]
pub struct ConfigManager {
    /// Current configuration
    config: Arc<RwLock<Config>>,

    /// Path to the configuration file
    config_path: PathBuf,

    /// Broadcast channel for configuration changes
    change_tx: broadcast::Sender<Config>,

    /// File watcher task handle
    _watcher_handle: JoinHandle<()>,
}

impl ConfigManager {
    /// Create a new configuration manager
    async fn new(initial_config: Config, config_path: PathBuf) -> Result<Self> {
        let config = Arc::new(RwLock::new(initial_config));
        let (change_tx, _) = broadcast::channel(16);

        // Clone references for the watcher task
        let config_clone = Arc::clone(&config);
        let change_tx_clone = change_tx.clone();
        let path_clone = config_path.clone();

        // Start file watcher task
        let watcher_handle = tokio::spawn(async move {
            if let Err(e) = Self::watch_config_file(path_clone, config_clone, change_tx_clone).await
            {
                error!("Configuration file watcher error: {}", e);
            }
        });

        Ok(Self {
            config,
            config_path,
            change_tx,
            _watcher_handle: watcher_handle,
        })
    }

    /// Get the current configuration
    pub async fn get_config(&self) -> Config {
        self.config.read().await.clone()
    }

    /// Subscribe to configuration changes
    pub fn subscribe(&self) -> broadcast::Receiver<Config> {
        self.change_tx.subscribe()
    }

    /// Manually reload configuration from file
    pub async fn reload(&self) -> Result<()> {
        debug!(
            "Manually reloading configuration from {:?}",
            self.config_path
        );

        match Self::load_config_from_file(&self.config_path).await {
            Ok(new_config) => {
                // Update the stored configuration
                {
                    let mut config_guard = self.config.write().await;
                    *config_guard = new_config.clone();
                }

                // Notify subscribers
                if let Err(e) = self.change_tx.send(new_config) {
                    warn!("Failed to notify configuration subscribers: {}", e);
                }

                info!("Configuration reloaded successfully");
                Ok(())
            }
            Err(e) => {
                error!("Failed to reload configuration: {}", e);
                Err(e)
            }
        }
    }

    /// Load configuration from file (internal helper)
    async fn load_config_from_file(path: &Path) -> Result<Config> {
        let content = fs::read_to_string(path).await?;
        let mut config: Config = serde_yaml::from_str(&content)?;

        // Apply environment variable overrides
        config.apply_env_overrides();

        // Validate configuration
        config.validate()?;

        Ok(config)
    }

    /// Watch configuration file for changes
    async fn watch_config_file(
        config_path: PathBuf,
        config: Arc<RwLock<Config>>,
        change_tx: broadcast::Sender<Config>,
    ) -> Result<()> {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher};
        use std::time::Duration;

        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Create file watcher
        let mut watcher = RecommendedWatcher::new(
            move |res| {
                if let Err(e) = tx.try_send(res) {
                    error!("Failed to send file event: {}", e);
                }
            },
            notify::Config::default(),
        )?;

        // Watch the configuration file
        watcher.watch(&config_path, RecursiveMode::NonRecursive)?;

        info!("Watching configuration file: {:?}", config_path);

        // Process file events
        while let Some(event) = rx.recv().await {
            match event {
                Ok(notify::Event {
                    kind: notify::EventKind::Modify(_),
                    paths,
                    ..
                }) => {
                    if paths.contains(&config_path) {
                        debug!("Configuration file modified: {:?}", config_path);

                        // Debounce rapid file changes
                        tokio::time::sleep(Duration::from_millis(100)).await;

                        // Reload configuration
                        match Self::load_config_from_file(&config_path).await {
                            Ok(new_config) => {
                                // Update the stored configuration
                                {
                                    let mut config_guard = config.write().await;
                                    *config_guard = new_config.clone();
                                }

                                // Notify subscribers
                                if let Err(e) = change_tx.send(new_config) {
                                    warn!("Failed to notify configuration subscribers: {}", e);
                                }

                                info!("Configuration hot-reloaded from {:?}", config_path);
                            }
                            Err(e) => {
                                error!("Failed to hot-reload configuration: {}", e);
                            }
                        }
                    }
                }
                Ok(event) => {
                    debug!("File event: {:?}", event);
                }
                Err(e) => {
                    error!("File watcher error: {}", e);
                }
            }
        }

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

    #[lexum_macros::tokio_test]
    async fn test_serialize_deserialize() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.network.http_port, parsed.network.http_port);
    }

    #[lexum_macros::tokio_test]
    async fn test_config_manager_creation() {
        use std::io::Write;
        use tempfile::NamedTempFile;

        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        // Create config manager
        let manager = Config::from_file_with_hot_reload(temp_file.path())
            .await
            .unwrap();

        // Verify initial config
        let loaded_config = manager.get_config().await;
        assert_eq!(loaded_config.network.http_port, 9200);
    }

    #[lexum_macros::tokio_test]
    async fn test_config_reload() {
        use std::io::{Seek, Write};
        use tempfile::NamedTempFile;

        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let mut config = Config::default();
        config.network.http_port = 9200;
        let yaml = serde_yaml::to_string(&config).unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        // Create config manager
        let manager = Config::from_file_with_hot_reload(temp_file.path())
            .await
            .unwrap();

        // Verify initial config
        let initial_config = manager.get_config().await;
        assert_eq!(initial_config.network.http_port, 9200);

        // Modify config file (clear and write new content)
        temp_file.as_file_mut().set_len(0).unwrap();
        temp_file
            .as_file_mut()
            .seek(std::io::SeekFrom::Start(0))
            .unwrap();
        let mut modified_config = Config::default();
        modified_config.network.http_port = 9300;
        modified_config.network.transport_port = 9400; // Ensure different ports
        let modified_yaml = serde_yaml::to_string(&modified_config).unwrap();
        temp_file.write_all(modified_yaml.as_bytes()).unwrap();

        // Reload configuration
        manager.reload().await.unwrap();

        // Verify reloaded config
        let reloaded_config = manager.get_config().await;
        assert_eq!(reloaded_config.network.http_port, 9300);
    }

    #[lexum_macros::tokio_test]
    async fn test_config_subscription() {
        use std::io::{Seek, Write};
        use tempfile::NamedTempFile;

        // Create a temporary config file
        let mut temp_file = NamedTempFile::new().unwrap();
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        temp_file.write_all(yaml.as_bytes()).unwrap();

        // Create config manager
        let manager = Config::from_file_with_hot_reload(temp_file.path())
            .await
            .unwrap();

        // Subscribe to changes
        let mut rx = manager.subscribe();

        // Modify config file (clear and write new content)
        temp_file.as_file_mut().set_len(0).unwrap();
        temp_file
            .as_file_mut()
            .seek(std::io::SeekFrom::Start(0))
            .unwrap();
        let mut modified_config = Config::default();
        modified_config.network.http_port = 9400;
        let modified_yaml = serde_yaml::to_string(&modified_config).unwrap();
        temp_file.write_all(modified_yaml.as_bytes()).unwrap();

        // Reload configuration
        manager.reload().await.unwrap();

        // Wait for notification (with timeout)
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), rx.recv()).await;

        match result {
            Ok(Ok(notified_config)) => {
                assert_eq!(notified_config.network.http_port, 9400);
            }
            Ok(Err(e)) => {
                panic!("Failed to receive configuration change: {e}");
            }
            Err(_) => {
                panic!("Timeout waiting for configuration change");
            }
        }
    }

    // ============================================================================
    // Snapshot Repository Configuration Tests
    // ============================================================================

    #[test]
    fn test_snapshot_config_default() {
        let config = SnapshotConfig::default();
        assert_eq!(config.path, "./snapshots");
        assert_eq!(config.max_snapshots, 100);
        assert!(config.compression_enabled);
        assert_eq!(config.repositories.len(), 1);
        assert_eq!(config.repositories[0].name, "default");
    }

    #[test]
    fn test_snapshot_config_validation_success() {
        let config = SnapshotConfig {
            path: "/tmp/snapshots".to_string(),
            max_snapshots: 50,
            compression_enabled: true,
            repositories: vec![SnapshotRepositoryConfig {
                name: "test_repo".to_string(),
                repository_type: "fs".to_string(),
                settings: SnapshotRepositorySettings {
                    location: "/tmp/repo".to_string(),
                    ..Default::default()
                },
            }],
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_snapshot_config_validation_empty_path() {
        let config = SnapshotConfig {
            path: "".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_config_validation_zero_max_snapshots() {
        let config = SnapshotConfig {
            max_snapshots: 0,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_config_validation_too_many_snapshots() {
        let config = SnapshotConfig {
            max_snapshots: 10001,
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_config_validation_no_repositories() {
        let config = SnapshotConfig {
            repositories: vec![],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_config_validation_duplicate_repository_names() {
        let config = SnapshotConfig {
            repositories: vec![
                SnapshotRepositoryConfig {
                    name: "repo1".to_string(),
                    ..Default::default()
                },
                SnapshotRepositoryConfig {
                    name: "repo1".to_string(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_repository_config_validation_success() {
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "fs".to_string(),
            settings: SnapshotRepositorySettings {
                location: "/tmp/repo".to_string(),
                ..Default::default()
            },
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_snapshot_repository_config_validation_empty_name() {
        let config = SnapshotRepositoryConfig {
            name: "".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_repository_config_validation_invalid_name() {
        let config = SnapshotRepositoryConfig {
            name: "invalid@name".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_repository_config_validation_invalid_type() {
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "invalid_type".to_string(),
            ..Default::default()
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_fs() {
        let settings = SnapshotRepositorySettings {
            location: "/tmp/repo".to_string(),
            ..Default::default()
        };
        assert!(settings.validate("fs").is_ok());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_fs_empty_location() {
        let settings = SnapshotRepositorySettings {
            location: "".to_string(),
            ..Default::default()
        };
        assert!(settings.validate("fs").is_err());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_s3() {
        let settings = SnapshotRepositorySettings {
            location: "my-bucket".to_string(),
            s3_settings: Some(S3Settings {
                region: "us-east-1".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(settings.validate("s3").is_ok());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_s3_empty_bucket() {
        let settings = SnapshotRepositorySettings {
            location: "".to_string(),
            s3_settings: Some(S3Settings::default()),
            ..Default::default()
        };
        assert!(settings.validate("s3").is_err());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_invalid_chunk_size() {
        let settings = SnapshotRepositorySettings {
            chunk_size: "invalid".to_string(),
            ..Default::default()
        };
        assert!(settings.validate("fs").is_err());
    }

    #[test]
    fn test_snapshot_repository_settings_validation_valid_chunk_sizes() {
        let valid_sizes = ["1b", "1kb", "1mb", "1gb", "1tb", "512mb", "1024kb"];

        for size in &valid_sizes {
            let settings = SnapshotRepositorySettings {
                chunk_size: size.to_string(),
                ..Default::default()
            };
            assert!(settings.validate("fs").is_ok(), "Failed for size: {size}");
        }
    }

    #[test]
    fn test_s3_settings_validation_success() {
        let settings = S3Settings {
            region: "us-east-1".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_s3_settings_validation_empty_region() {
        let settings = S3Settings {
            region: "".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_s3_settings_validation_invalid_region() {
        let settings = S3Settings {
            region: "invalid@region".to_string(),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_gcs_settings_validation_success() {
        let settings = GcsSettings {
            service_account_key_file: Some("/path/to/key.json".to_string()),
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_gcs_settings_validation_no_auth() {
        let settings = GcsSettings::default();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_azure_settings_validation_success() {
        let settings = AzureSettings {
            account_name: Some("myaccount".to_string()),
            account_key: Some("mykey".to_string()),
            ..Default::default()
        };
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn test_azure_settings_validation_no_auth() {
        let settings = AzureSettings::default();
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_azure_settings_validation_account_name_without_key() {
        let settings = AzureSettings {
            account_name: Some("myaccount".to_string()),
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn test_retention_policy_validation_success() {
        let policy = RetentionPolicy {
            keep_for_days: Some(30),
            keep_count: Some(10),
            delete_after_days: Some(90),
        };
        assert!(policy.validate().is_ok());
    }

    #[test]
    fn test_retention_policy_validation_no_rules() {
        let policy = RetentionPolicy {
            keep_for_days: None,
            keep_count: None,
            delete_after_days: None,
        };
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_retention_policy_validation_zero_days() {
        let policy = RetentionPolicy {
            keep_for_days: Some(0),
            ..Default::default()
        };
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_retention_policy_validation_keep_greater_than_delete() {
        let policy = RetentionPolicy {
            keep_for_days: Some(90),
            delete_after_days: Some(30),
            ..Default::default()
        };
        assert!(policy.validate().is_err());
    }

    #[test]
    fn test_snapshot_repository_settings_size_validation() {
        let _settings = SnapshotRepositorySettings::default();

        // Test valid size strings
        assert!(SnapshotRepositorySettings::is_valid_size_string("1b"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("1kb"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("1mb"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("1gb"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("1tb"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("512mb"));
        assert!(SnapshotRepositorySettings::is_valid_size_string("1024kb"));

        // Test invalid size strings
        assert!(!SnapshotRepositorySettings::is_valid_size_string(""));
        assert!(!SnapshotRepositorySettings::is_valid_size_string("invalid"));
        assert!(!SnapshotRepositorySettings::is_valid_size_string("1"));
        assert!(!SnapshotRepositorySettings::is_valid_size_string("1xx"));
        assert!(!SnapshotRepositorySettings::is_valid_size_string("mb"));
    }

    #[test]
    fn test_snapshot_config_serialization() {
        let config = SnapshotConfig::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: SnapshotConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.path, parsed.path);
        assert_eq!(config.max_snapshots, parsed.max_snapshots);
        assert_eq!(config.compression_enabled, parsed.compression_enabled);
    }

    #[test]
    fn test_snapshot_repository_config_serialization() {
        let config = SnapshotRepositoryConfig {
            name: "test_repo".to_string(),
            repository_type: "s3".to_string(),
            settings: SnapshotRepositorySettings {
                location: "my-bucket".to_string(),
                s3_settings: Some(S3Settings {
                    region: "us-west-2".to_string(),
                    ..Default::default()
                }),
                ..Default::default()
            },
        };

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: SnapshotRepositoryConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.name, parsed.name);
        assert_eq!(config.repository_type, parsed.repository_type);
        assert_eq!(config.settings.location, parsed.settings.location);
    }
}
