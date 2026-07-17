# Lexum Documentation

**Lexum** is a high-performance, distributed full-text search engine written in Rust, inspired by ElasticSearch but designed from the ground up for modern cloud-native architectures.

## Overview

Lexum provides enterprise-grade full-text search capabilities with:

- **High Performance**: Built with Rust and Tokio for maximum throughput and minimal latency
- **Distributed Architecture**: Native support for sharding and replication
- **Flexible Query Language**: LQL (Lexum Query Language) - inspired by ESQL with enhanced capabilities
- **Multiple Protocols**: StreamableHTTP, MCP, and UMICP support
- **Rich GUI**: Electron-based observability interface similar to Kibana
- **Production Ready**: Comprehensive telemetry, monitoring, and deployment tools

## Key Features

### Core Engine
- **Full-Text Search**: Advanced indexing and search powered by Tantivy
- **Sharding**: Automatic data distribution across nodes
- **Replication**: Configurable replication factor for high availability
- **Real-time Indexing**: Near real-time document updates
- **Schema Flexibility**: Dynamic and strict schema modes

### Query Capabilities
- **LQL (Lexum Query Language)**: Powerful SQL-like query syntax
- **Aggregations**: Rich aggregation framework (terms, stats, histograms, etc.)
- **Full-Text Search**: BM25 scoring, fuzzy matching, phrase queries
- **Filtering**: Complex boolean logic with nested queries
- **Sorting & Pagination**: Efficient result ordering and pagination

### Protocols
- **StreamableHTTP**: Streaming HTTP responses for large result sets
- **MCP**: Model Context Protocol for AI/LLM integration
- **UMICP**: Universal Model Interchange Communication Protocol
- **REST API**: Standard HTTP/JSON API

### Operations
- **Telemetry**: OpenTelemetry integration for traces, metrics, and logs
- **Monitoring**: Built-in health checks and metrics endpoints
- **Administration**: Cluster management, index operations, user management
- **Security**: TLS, authentication, role-based access control

## Documentation Structure

### Core

- [Architecture](./ARCHITECTURE.md) - System design and components
- [Roadmap](./ROADMAP.md) - Development phases and future plans
- [Status](./STATUS.md) - Current project status
- [DAG](./DAG.md) - Module dependency graph
- [Performance](./PERFORMANCE.md) - Performance characteristics
- [Implementation Summary](./IMPLEMENTATION_SUMMARY.md) - What is built and tested

### API

- [API Reference](./api/API_REFERENCE.md) - Complete API documentation
- [Query Language](./api/QUERY_LANGUAGE.md) - LQL specification and examples

### Development

- [Development Guide](./development/DEVELOPMENT.md) - Development setup and workflow
- [CI/CD](./development/CI_CD.md) - Build and deployment pipelines
- [Windows Native](./development/WINDOWS_NATIVE.md) - Windows development setup
- [WSL/Tantivy Conflict](./development/WSL_TANTIVY_CONFLICT.md) - Why builds must run on native Windows

### Deployment

- [Deployment](./deployment/DEPLOYMENT.md) - Docker and Kubernetes deployment guides
- [Telemetry](./deployment/TELEMETRY.md) - Observability and monitoring setup
- [Snapshot Configuration](./deployment/SNAPSHOT_CONFIGURATION.md) - Backup/restore configuration

### Testing

- [Test Coverage Report](./testing/TEST_COVERAGE_REPORT.md) - Coverage details
- [Test Results](./testing/TEST_RESULTS.md) - Latest test run results

### Guides

- [Troubleshooting](./guides/TROUBLESHOOTING.md) - Common issues and solutions
- [GUI](./guides/GUI.md) - Electron-based GUI documentation

### Specifications

- [Specs Index](./specs/README.md) - The implementation contract: SPEC-001..SPEC-016, normative (RFC 2119) with stable requirement IDs, mapped to roadmap phases

### Analysis

- [Meilisearch Analysis](./analysis/meilisearch/README.md) - Architecture and parity study (F-001..F-039)
- [Elasticsearch Analysis](./analysis/elastic/README.md) - Architecture and parity study (F-001..F-055)
- [Tantivy Alternatives](./analysis/TANTIVY_ALTERNATIVES.md) - Search library evaluation

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

### Running Locally

```bash
# Start single-node instance
lexum serve --config config.yml

# Start with custom data directory
lexum serve --data-dir ./data --http-port 9200
```

### Create an Index

```bash
# Using HTTP API
curl -X PUT http://localhost:9200/my_index \
  -H 'Content-Type: application/json' \
  -d '{
    "settings": {
      "number_of_shards": 3,
      "number_of_replicas": 1
    },
    "mappings": {
      "properties": {
        "title": { "type": "text" },
        "content": { "type": "text" },
        "created_at": { "type": "date" }
      }
    }
  }'
```

### Index a Document

```bash
curl -X POST http://localhost:9200/my_index/_doc \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Getting Started with Lexum",
    "content": "Lexum is a powerful search engine...",
    "created_at": "2024-10-25T00:00:00Z"
  }'
```

### Search

```bash
# Simple search
curl -X POST http://localhost:9200/my_index/_search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "match": {
        "content": "search engine"
      }
    }
  }'

# Using LQL
curl -X POST http://localhost:9200/_lql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM my_index | WHERE content MATCH \"search engine\" | SORT created_at DESC | LIMIT 10"
  }'
```

## Architecture Highlights

```
┌─────────────────┐
│   GUI (Electron)│
│    Lexum UI     │
└────────┬────────┘
         │
    ┌────┴────┐
    │  HTTP   │
    │  MCP    │
    │  UMICP  │
    └────┬────┘
         │
┌────────┴─────────┐
│   API Gateway    │
│  (Load Balancer) │
└────────┬─────────┘
         │
    ┌────┴─────┐
    │          │
┌───┴──┐   ┌──┴───┐
│Node 1│   │Node 2│  ... Lexum Cluster
└───┬──┘   └──┬───┘
    │         │
┌───┴─────────┴───┐
│  Distributed     │
│  Storage Layer   │
│  (Shards)        │
└──────────────────┘
```

## Technology Stack

### Core
- **Language**: Rust 2024 Edition
- **Runtime**: Tokio (async/await)
- **Web Framework**: Axum
- **Search Engine**: Tantivy
- **Serialization**: Serde, bincode

### Storage
- **Index**: Tantivy inverted index
- **Metadata**: RocksDB
- **Replication**: Raft consensus

### Networking
- **HTTP**: Hyper, Axum
- **Streaming**: Server-Sent Events (SSE), WebSockets
- **RPC**: gRPC, UMICP

### Observability
- **Telemetry**: OpenTelemetry
- **Metrics**: Prometheus format
- **Tracing**: Distributed tracing with Jaeger
- **Logging**: Structured logging with tracing

### GUI
- **Framework**: Electron
- **Frontend**: React + TypeScript
- **Visualization**: D3.js, Recharts
- **Editor**: Monaco Editor (for LQL)

## Performance Characteristics

- **Indexing**: ~50K-100K docs/sec per node (varies by doc size)
- **Search Latency**: < 10ms p95 for most queries
- **Throughput**: 10K+ queries/sec per node
- **Storage Overhead**: ~1.5x original data size
- **Memory**: ~2GB base + index cache

## Compatibility

### Supported Platforms
- Linux (x86_64, aarch64)
- macOS (Intel, Apple Silicon)
- Windows (x86_64)

### Kubernetes
- Kubernetes 1.25+
- Helm 3.10+

### Docker
- Docker 20.10+
- Docker Compose 2.0+

## License

Lexum is open-source software licensed under the [Apache License 2.0](../LICENSE).

## Community

- **Issues**: [GitHub Issues](https://github.com/your-org/lexum/issues)
- **Discussions**: [GitHub Discussions](https://github.com/your-org/lexum/discussions)
- **Discord**: [Join our Discord](https://discord.gg/lexum)
- **Twitter**: [@LexumSearch](https://twitter.com/lexumsearch)

## Contributing

See [DEVELOPMENT.md](./development/DEVELOPMENT.md) for development setup and [CONTRIBUTING.md](../CONTRIBUTING.md) for contribution guidelines.

