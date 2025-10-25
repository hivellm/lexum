//! Server configuration and startup

use crate::handlers::index::AppState;
use crate::router::build_router;
use lexum_core::IndexManager;
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
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:9200".parse().unwrap(),
            data_dir: "./data".to_string(),
        }
    }
}

/// Lexum server
pub struct Server {
    config: ServerConfig,
    index_manager: Arc<IndexManager>,
}

impl Server {
    /// Create new server
    pub fn new(config: ServerConfig) -> Self {
        let index_manager = Arc::new(IndexManager::new(&config.data_dir));

        Self {
            config,
            index_manager,
        }
    }

    /// Run server
    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Starting Lexum server on {}", self.config.bind_addr);

        let state = AppState {
            index_manager: self.index_manager,
        };

        let app = build_router(state);

        let listener = TcpListener::bind(&self.config.bind_addr).await?;

        tracing::info!("Lexum server listening on {}", self.config.bind_addr);

        axum::serve(listener, app).await?;

        Ok(())
    }
}
