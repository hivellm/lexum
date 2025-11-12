# Lexum Core

Core library for Lexum search engine providing configuration management, logging, and foundational types.

## Features

- **Configuration**: YAML configuration with environment variable overrides
- **Logging**: Structured logging with tracing
- **Type Safety**: Strong typing for core types (DocumentId, IndexName, Score)
- **Error Handling**: Comprehensive error types with thiserror

## Usage

```rust
use lexum_core::{Config, logging};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    logging::init()?;
    
    // Load configuration
    let config = Config::from_file("config.yml").await?;
    
    tracing::info!("Lexum started");
    Ok(())
}
```

## Configuration

Configuration can be loaded from YAML files or environment variables:

### YAML File

```yaml
cluster:
  name: lexum-prod
node:
  name: node-1
  roles: [master, data]
network:
  http_port: 9200
```

### Environment Variables

- `LEXUM_CLUSTER_NAME` - Cluster name
- `LEXUM_NODE_NAME` - Node name
- `LEXUM_HTTP_PORT` - HTTP port
- `LEXUM_LOG_LEVEL` - Log level (trace, debug, info, warn, error)

## License

Apache License 2.0

