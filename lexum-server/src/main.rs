//! Lexum server binary

use lexum_server::{Server, server::ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create and run server
    let config = ServerConfig::default();
    let server = Server::new(config);

    server.run().await
}
