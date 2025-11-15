//! Server configuration and startup

use crate::handlers::index::AppState;
use crate::middleware::connection_pool::ConnectionPoolConfig;
use crate::middleware::http2_push::Http2PushConfig;
use crate::router::build_router;
use lexum_core::{
    IndexManager, ProgressTracker, SnapshotManager, TemplateManager,
    config::{Config, ConfigManager},
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address
    pub bind_addr: SocketAddr,
    /// Data directory
    pub data_dir: String,
    /// Configuration file path (optional, for hot-reload)
    pub config_path: Option<String>,
    /// Connection pool configuration
    pub connection_pool: ConnectionPoolConfig,
    /// HTTP/2 push configuration
    pub http2_push: Http2PushConfig,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:17000".parse().unwrap(),
            data_dir: "./data".to_string(),
            config_path: None,
            connection_pool: ConnectionPoolConfig::default(),
            http2_push: Http2PushConfig::default(),
        }
    }
}

/// Lexum server
pub struct Server {
    config: ServerConfig,
    index_manager: Arc<IndexManager>,
    snapshot_manager: Arc<RwLock<SnapshotManager>>,
    config_manager: Option<Arc<ConfigManager>>,
}

impl Server {
    /// Create new server
    pub fn new(config: ServerConfig) -> anyhow::Result<Self> {
        let index_manager = Arc::new(IndexManager::new(&config.data_dir));

        // Use default config if no config file specified
        let default_config = Config::default();
        let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&default_config)?));

        Ok(Self {
            config,
            index_manager,
            snapshot_manager,
            config_manager: None,
        })
    }

    /// Create new server with configuration hot-reload
    pub async fn new_with_hot_reload(config: ServerConfig) -> anyhow::Result<Self> {
        let index_manager = Arc::new(IndexManager::new(&config.data_dir));

        if let Some(config_path) = &config.config_path {
            let manager = Config::from_file_with_hot_reload(config_path).await?;
            let current_config = manager.get_config().await;
            let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&current_config)?));

            Ok(Self {
                config,
                index_manager,
                snapshot_manager,
                config_manager: Some(Arc::new(manager)),
            })
        } else {
            // Fallback to default config
            let default_config = Config::default();
            let snapshot_manager = Arc::new(RwLock::new(SnapshotManager::new(&default_config)?));

            Ok(Self {
                config,
                index_manager,
                snapshot_manager,
                config_manager: None,
            })
        }
    }

    /// Run server
    pub async fn run(self) -> anyhow::Result<()> {
        let (host, port) = {
            let addr = self.config.bind_addr;
            (addr.ip().to_string(), addr.port())
        };

        tracing::info!("Starting Lexum Server on {}:{}", host, port);

        // Create MCP router (main server) using StreamableHTTP transport
        tracing::info!("Creating MCP router with StreamableHTTP transport (rmcp 0.8)...");

        let state = AppState {
            index_manager: self.index_manager,
            snapshot_manager: self.snapshot_manager,
            template_manager: Arc::new(TemplateManager::new()),
            task_manager: Arc::new(crate::handlers::reindex::TaskManager::new()),
            progress_tracker: Arc::new(ProgressTracker::new()),
            auth_state: crate::middleware::auth::AuthState::new(
                crate::middleware::auth::AuthConfig::from_env(),
            ),
            query_complexity_config:
                crate::middleware::query_complexity::QueryComplexityLimitConfig::default(),
            metrics: Arc::new(crate::handlers::metrics::PrometheusMetrics::new()),
        };

        let app = build_router(state, &self.config.http2_push);
        tracing::info!("MCP router created (StreamableHTTP)");

        let listener = TcpListener::bind(&self.config.bind_addr).await?;

        tracing::info!("Lexum Server available at:");
        tracing::info!("   MCP StreamableHTTP: http://{}:{}/mcp", host, port);
        tracing::info!("   REST API: http://{}:{}", host, port);
        tracing::info!("   UMICP: http://{}:{}/umicp", host, port);
        tracing::info!("   Metrics: http://{}:{}/_metrics", host, port);
        tracing::info!("   Health: http://{}:{}/health", host, port);
        tracing::info!(
            "MCP server (StreamableHTTP) with REST API listening on {}:{}",
            host,
            port
        );

        // Start configuration hot-reload task if enabled
        if let Some(config_manager) = &self.config_manager {
            let config_manager = Arc::clone(config_manager);
            tokio::spawn(async move {
                let mut rx = config_manager.subscribe();
                while let Ok(new_config) = rx.recv().await {
                    tracing::info!("Configuration reloaded, updating server settings");
                    // Here we could update server settings like rate limits, auth config, etc.
                    // For now, we just log the event
                    tracing::debug!("New configuration: {:?}", new_config);
                }
            });
        }

        // Configure connection pooling via hyper
        // Note: Axum uses hyper which has built-in connection pooling
        // The ConnectionPoolConfig is available for future use and documentation
        tracing::debug!(
            "Connection pool config: max_idle_per_host={}, max_idle_total={}, idle_timeout={:?}, keep_alive={:?}",
            self.config.connection_pool.max_idle_per_host,
            self.config.connection_pool.max_idle_total,
            self.config.connection_pool.idle_timeout,
            self.config.connection_pool.keep_alive
        );

        // Serve with graceful shutdown
        // Hyper (used by Axum) automatically handles connection pooling and keep-alive
        let server_handle = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

        // Wait for server to complete or shutdown signal
        tokio::select! {
            result = server_handle => {
                match result {
                    Ok(_) => tracing::info!("Server completed normally"),
                    Err(e) => tracing::error!("Server error: {}", e),
                }
            }
            _ = async {
                // This branch is handled by shutdown_signal
                std::future::pending::<()>().await
            } => {
                tracing::info!("Shutdown signal received, stopping server...");
            }
        }

        tracing::info!("Server stopped gracefully");

        Ok(())
    }
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM)
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received shutdown signal (Ctrl+C)");
        },
        _ = terminate => {
            tracing::info!("Received shutdown signal (SIGTERM)");
        },
    }

    tracing::info!("Starting graceful shutdown...");
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_server_creation() {
        let config = ServerConfig::default();
        let server = Server::new(config);
        assert!(server.is_ok());
    }

    #[lexum_macros::tokio_test]
    async fn test_server_with_hot_reload() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yml");

        // Create a test config file
        let test_config = r#"
cluster:
  name: "test-cluster"
node:
  name: "test-node"
  roles: ["data", "master"]
network:
  host: "127.0.0.1"
  http_port: 17000
  transport_port: 17300
path:
  data: "./data"
  logs: "./logs"
logging:
  level: "info"
  format: "pretty"
snapshots:
  repositories:
    - name: "default"
      repository_type: "fs"
      settings:
        location: "./snapshots"
"#;
        std::fs::write(&config_path, test_config).unwrap();

        let config = ServerConfig {
            bind_addr: "127.0.0.1:17001".parse().unwrap(),
            data_dir: temp_dir.path().join("data").to_string_lossy().to_string(),
            config_path: Some(config_path.to_string_lossy().to_string()),
            connection_pool: ConnectionPoolConfig::default(),
            http2_push: Http2PushConfig::default(),
        };

        let server = Server::new_with_hot_reload(config).await;
        assert!(server.is_ok());
    }
}
