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

/// Snapshot repository settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotRepositorySettings {
    /// Location for filesystem repositories
    #[serde(default = "default_location")]
    pub location: String,

    /// Compress snapshots
    #[serde(default = "default_compress")]
    pub compress: bool,

    /// Chunk size for snapshots
    #[serde(default = "default_chunk_size")]
    pub chunk_size: String,

    /// Maximum number of snapshots per repository
    #[serde(default = "default_max_restore_bytes_per_sec")]
    pub max_restore_bytes_per_sec: String,

    /// Maximum number of snapshots per repository
    #[serde(default = "default_max_snapshot_bytes_per_sec")]
    pub max_snapshot_bytes_per_sec: String,

    /// Readonly repository
    #[serde(default = "default_readonly")]
    pub readonly: bool,
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

impl Default for SnapshotRepositorySettings {
    fn default() -> Self {
        Self {
            location: default_location(),
            compress: default_compress(),
            chunk_size: default_chunk_size(),
            max_restore_bytes_per_sec: default_max_restore_bytes_per_sec(),
            max_snapshot_bytes_per_sec: default_max_snapshot_bytes_per_sec(),
            readonly: default_readonly(),
        }
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
    ///     while let Some(new_config) = rx.recv().await {
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

    #[tokio::test]
    async fn test_serialize_deserialize() {
        let config = Config::default();
        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: Config = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(config.network.http_port, parsed.network.http_port);
    }

    #[tokio::test]
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

    #[tokio::test]
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

    #[tokio::test]
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
                panic!("Failed to receive configuration change: {}", e);
            }
            Err(_) => {
                panic!("Timeout waiting for configuration change");
            }
        }
    }
}
