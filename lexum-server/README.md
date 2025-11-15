# Lexum Server

REST API server for the Lexum search engine.

## Features

- **Index Management**: Create, delete, and list indices
- **Document Operations**: Add, get, update, and delete documents
- **Search**: Execute search queries with filtering and pagination
- **Reindexing**: Copy documents between indices with transformation support
- **Task Management**: Track and monitor long-running operations
- **Health Check**: Service health monitoring

## API Endpoints

### Health Check

```
GET /health
```

### Index Management

```
POST   /api/v1/indices           - Create index
GET    /api/v1/indices           - List indices
GET    /api/v1/indices/:name     - Get index info
DELETE /api/v1/indices/:name     - Delete index
```

### Document Operations

```
POST   /api/v1/indices/:index/documents     - Add document
GET    /api/v1/indices/:index/documents/:id - Get document
PUT    /api/v1/indices/:index/documents/:id - Update document
DELETE /api/v1/indices/:index/documents/:id - Delete document
```

### Search

```
POST   /api/v1/indices/:index/search - Search documents
```

### Reindexing Operations

```
POST   /_reindex                    - Start reindex operation
GET    /_tasks                      - List all tasks
GET    /_tasks/:task_id             - Get task information
POST   /_tasks/:task_id/_    - Cancel task
```
```

## Running

```bash
cargo run --bin lexum-server
```

Server will start on `http://127.0.0.1:9200` by default.

## Configuration

Configuration is done through `ServerConfig`:

```rust
use lexum_server::{Server, server::ServerConfig};

let config = ServerConfig {
    bind_addr: "0.0.0.0:9200".parse().unwrap(),
    data_dir: "./data".to_string(),
};

let server = Server::new(config);
server.run().await?;
```

## License

Apache-2.0

