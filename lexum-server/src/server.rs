//! Server configuration and startup

use crate::handlers::index::AppState;
use crate::router::build_router;
use lexum_core::{IndexManager, SnapshotManager, config::Config};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind address
    pub bind_addr: SocketAddr,
    /// Data directory
    pub data_dir: String,
    /// Configuration
    pub config: Config,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9200".parse().unwrap(),
            data_dir: "./data".to_string(),
            config: Config::default(),
        }
    }
}

/// Lexum server
pub struct Server {
    config: ServerConfig,
    index_manager: Arc<IndexManager>,
    snapshot_manager: Arc<SnapshotManager>,
}

impl Server {
    /// Create new server
    pub fn new(config: ServerConfig) -> anyhow::Result<Self> {
        let index_manager = Arc::new(IndexManager::new(&config.data_dir));
        let snapshot_manager = Arc::new(SnapshotManager::new(&config.config)?);

        Ok(Self {
            config,
            index_manager,
            snapshot_manager,
        })
    }

    /// Run server
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Starting Lexum server on {}", self.config.bind_addr);

        let state = AppState {
            index_manager: self.index_manager,
            snapshot_manager: self.snapshot_manager,
        };

        let app = build_router(state);

        let listener = TcpListener::bind(&self.config.bind_addr).await?;

        tracing::info!("Lexum server listening on {}", self.config.bind_addr);

        // Serve with graceful shutdown
        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await?;

        tracing::info!("Lexum server shutdown complete");

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
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM signal");
        },
    }

    tracing::info!("Starting graceful shutdown...");
}
