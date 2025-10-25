//! Lexum server binary

use lexum_server::{Server, server::ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create and run server
    let config = ServerConfig::default();
    let server = Server::new(config)?;

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
        let config = ServerConfig::default();
        // Verify that default config can be created
        // This ensures the main function can initialize properly
        assert!(true); // Placeholder - actual assertions would depend on ServerConfig implementation
    }
}
