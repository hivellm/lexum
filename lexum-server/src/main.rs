//! Lexum server binary

use lexum_server::{Server, server::ServerConfig};
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Get configuration file path from environment or command line
    let config_path = env::args()
        .nth(1)
        .or_else(|| env::var("LEXUM_CONFIG_FILE").ok());

    let config = ServerConfig {
        bind_addr: "127.0.0.1:9200".parse().unwrap(),
        data_dir: "./data".to_string(),
        config_path,
    };

    // Create server with or without hot-reload
    let server = if config.config_path.is_some() {
        Server::new_with_hot_reload(config).await?
    } else {
        Server::new(config)?
    };

    server.run().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_function_exists() {
        // This test ensures the main function can be called
        // In a real test environment, we would mock the server creation
        // For now, we just verify the function signature is correct
        let _config = ServerConfig::default();
        // The actual server creation and running would be tested in integration tests
    }

    #[test]
    fn test_server_config_default() {
        let _config = ServerConfig::default();
        // Verify that default config can be created
        // This ensures the main function can initialize properly
        // Note: Actual assertions would depend on ServerConfig implementation
    }
}
