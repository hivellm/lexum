# Lexum

[![CI](https://github.com/your-org/lexum/workflows/CI/badge.svg)](https://github.com/your-org/lexum/actions/workflows/ci.yml)
[![Security](https://github.com/your-org/lexum/workflows/Security/badge.svg)](https://github.com/your-org/lexum/actions/workflows/security.yml)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)

**Lexum** is a high-performance, distributed full-text search engine written in Rust, inspired by ElasticSearch but designed from the ground up for modern cloud-native architectures.

## Features

- 🚀 **High Performance**: Built with Rust and Tokio for maximum throughput and minimal latency
- 🔍 **Full-Text Search**: Advanced indexing and search powered by Tantivy
- 🌐 **Distributed**: Native support for sharding and replication
- 💬 **LQL**: Powerful SQL-like query language (Lexum Query Language)
- 🔌 **Multiple Protocols**: StreamableHTTP, MCP, and UMICP support
- 🖥️ **Modern GUI**: Electron-based interface similar to Kibana
- 📊 **Observability**: Comprehensive telemetry, metrics, and distributed tracing
- 🐳 **Cloud Native**: Docker and Kubernetes ready
- 🔒 **Secure**: TLS, authentication, and role-based access control

## Quick Start

### Installation

```bash
# Install from crates.io
cargo install lexum

# Or build from source
git clone https://github.com/your-org/lexum
cd lexum
cargo build --release
```

### Running

```bash
# Start single-node instance
lexum serve --config config.yml

# With Docker
docker run -d -p 9200:9200 lexum/lexum:latest

# With Docker Compose
docker-compose up -d
```

### Basic Usage

```bash
# Create an index
curl -X PUT http://localhost:9200/my_index \
  -H 'Content-Type: application/json' \
  -d '{
    "settings": {
      "number_of_shards": 3,
      "number_of_replicas": 1
    }
  }'

# Index a document
curl -X POST http://localhost:9200/my_index/_doc \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Getting Started",
    "content": "Lexum is a powerful search engine"
  }'

# Search
curl -X POST http://localhost:9200/my_index/_search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "match": { "content": "search engine" }
    }
  }'

# Using LQL
curl -X POST http://localhost:9200/_lql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM my_index | WHERE content MATCH \"search engine\" | LIMIT 10"
  }'
```

## Documentation

- **[Overview](docs/README.md)** - Introduction and features
- **[Architecture](docs/ARCHITECTURE.md)** - System design and components
- **[Query Language](docs/QUERY_LANGUAGE.md)** - LQL specification
- **[API Reference](docs/API_REFERENCE.md)** - Complete API documentation
- **[Deployment](docs/DEPLOYMENT.md)** - Docker and Kubernetes deployment
- **[Telemetry](docs/TELEMETRY.md)** - Monitoring and observability
- **[GUI](docs/GUI.md)** - Electron-based interface
- **[Development](docs/DEVELOPMENT.md)** - Development guide
- **[CI/CD](docs/CI_CD.md)** - Build and deployment pipelines

## Architecture

```
┌─────────────┐
│   Clients   │
│ GUI/API/CLI │
└──────┬──────┘
       │
┌──────┴──────────────────┐
│    Protocol Layer       │
│ HTTP│MCP│UMICP│WebSocket│
└──────┬──────────────────┘
       │
┌──────┴──────────────────┐
│     API Gateway         │
│ Auth│Routing│Rate Limit │
└──────┬──────────────────┘
       │
┌──────┴──────────────────┐
│   Distributed Cluster   │
│  Master│Data│Coordinator│
└──────┬──────────────────┘
       │
┌──────┴──────────────────┐
│     Search Engine       │
│   Tantivy-powered       │
└─────────────────────────┘
```

## Technology Stack

- **Language**: Rust 2024 Edition
- **Runtime**: Tokio (async/await)
- **Web Framework**: Axum
- **Search**: Tantivy
- **Consensus**: Raft
- **Observability**: OpenTelemetry
- **GUI**: Electron + React + TypeScript

## Performance

- **Indexing**: ~50K-100K docs/sec per node
- **Search Latency**: < 10ms p95
- **Throughput**: 10K+ queries/sec per node
- **Scalability**: Horizontally scalable

## Use Cases

- **Application Search**: Add search to your application
- **Log Analytics**: Analyze and search logs in real-time
- **E-commerce**: Product search and recommendations
- **Content Management**: Full-text search for CMS
- **Monitoring**: Metrics and log aggregation
- **Security**: SIEM and threat detection

## Comparison with ElasticSearch

| Feature | Lexum | ElasticSearch |
|---------|-------|---------------|
| Language | Rust | Java |
| Memory Safety | ✅ | ❌ |
| Performance | Higher | High |
| Resource Usage | Lower | Higher |
| Query Language | LQL (SQL-like) | Query DSL (JSON) |
| License | Apache 2.0 | Elastic License |
| Native Protocols | HTTP, MCP, UMICP | HTTP |

## Roadmap

- [x] Core search engine
- [x] Distributed clustering
- [x] LQL query language
- [x] REST API
- [x] Telemetry and monitoring
- [x] Electron GUI
- [ ] Vector search (v0.2)
- [ ] Machine learning integration (v0.3)
- [ ] Geo-spatial queries (v0.3)
- [ ] Time-series optimization (v0.4)
- [ ] Graph queries (v0.5)

## Contributing

We welcome contributions! Please see:

- [Development Guide](docs/DEVELOPMENT.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [Code of Conduct](CODE_OF_CONDUCT.md)

### Development

```bash
# Clone repository
git clone https://github.com/your-org/lexum
cd lexum

# Install Rust nightly
rustup install nightly
rustup default nightly

# Build
cargo build

# Run tests
cargo test

# Run locally
cargo run -- serve --config config.example.yml
```

## Community

- **GitHub**: [Issues](https://github.com/your-org/lexum/issues) | [Discussions](https://github.com/your-org/lexum/discussions)
- **Discord**: [Join our Discord](https://discord.gg/lexum)
- **Twitter**: [@LexumSearch](https://twitter.com/lexumsearch)
- **Blog**: [blog.lexum.io](https://blog.lexum.io)

## License

Lexum is licensed under the [Apache License 2.0](LICENSE).

## Acknowledgments

- [Tantivy](https://github.com/quickwit-oss/tantivy) - Rust full-text search library
- [Tokio](https://tokio.rs) - Async runtime for Rust
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- Inspired by [ElasticSearch](https://www.elastic.co/elasticsearch/)

## Security

Please report security vulnerabilities to security@lexum.io. See [SECURITY.md](SECURITY.md) for details.

---

Built with ❤️ in Rust

