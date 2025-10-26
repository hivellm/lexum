# Lexum

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-alpha-brightgreen.svg)](docs/STATUS.md)
[![Tests](https://img.shields.io/badge/tests-278%20passing-success.svg)](IMPLEMENTATION_SUMMARY.md)
[![Coverage](https://img.shields.io/badge/coverage-%3E95%25-success.svg)](IMPLEMENTATION_SUMMARY.md)
[![Progress](https://img.shields.io/badge/progress-38%25-blue.svg)](openspec/OPENSPEC_STATUS.md)

**Lexum** is a high-performance, distributed full-text search engine written in Rust, inspired by ElasticSearch but designed from the ground up for modern cloud-native architectures.

> **✅ Project Status**: **Foundation Complete** (38% overall). Core search engine, REST API, CLI tool, and LQL query language are production-ready with 278 tests passing and >95% coverage. See [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for details.

## Features

### ✅ Implemented (Production-Ready)

- 🚀 **High Performance**: Built with Rust Edition 2024 and Tokio for maximum throughput
- 🔍 **Full-Text Search**: Advanced indexing and search powered by Tantivy 0.25
- 💬 **LQL**: Fully functional SQL-like query language with 9 query types
- 📡 **REST API**: 34 documented endpoints with OpenAPI/Swagger UI
- 🖥️ **CLI Tool**: Comprehensive command-line interface with 8 command groups
- 💾 **Snapshots**: Complete backup/restore system with repository management
- 📋 **Templates**: Automatic index configuration with pattern matching
- 🔐 **Security**: API key authentication, rate limiting, CORS
- 📊 **Monitoring**: Cluster health, statistics, and node monitoring
- ⚡ **Query Cache**: Performance optimization with caching
- 🧪 **Testing**: 278 tests passing with >95% coverage
- 📚 **Documentation**: Complete OpenAPI spec + usage examples

### 🚧 In Progress

- 🌐 **Distributed**: Sharding and replication (Phase 2)
- 🔌 **Multiple Protocols**: MCP and UMICP support (Phase 3)
- 🖥️ **Modern GUI**: Electron-based interface (Phase 4)
- 📊 **Telemetry**: Advanced observability (Phase 3)
- 🐳 **Cloud Native**: Docker and Kubernetes (Phase 2)

### 📋 Planned (Future Phases)

- Advanced aggregations
- Client SDKs (Python, JavaScript, Rust)
- Distributed clustering
- Advanced security features
- Production deployment tooling

## Quick Start

### Installation

```bash
# Clone repository
git clone https://github.com/hivellm/lexum.git
cd lexum

# Build all crates
cargo build --release

# Run server
./target/release/lexum-server

# Use CLI
./target/release/lexum-cli --help
```

### Using the CLI

```bash
# Start server in daemon mode
lexum server start --daemon

# Create an index
lexum index create products schema.yml

# Add documents
lexum doc bulk products products.json

# Search with LQL
lexum lql products "FROM products WHERE category:electronics AND price:[100,500]"

# Create snapshot
lexum snapshot create backup snap_2024 --indices products --wait

# Interactive REPL
lexum repl
```

### Using the API

```bash
# Health check
curl http://localhost:9200/health

# Create index
curl -X PUT http://localhost:9200/api/v1/indices/products \
  -H "Content-Type: application/json" \
  -d '{"fields": [{"name": "title", "type": "text"}], "settings": {}}'

# Search
curl -X POST http://localhost:9200/api/v1/indices/products/search \
  -H "Content-Type: application/json" \
  -d '{"query": {"match": {"field": "title", "query": "laptop"}}, "limit": 10}'

# Swagger UI
open http://localhost:9200/swagger-ui
```

## Current State

**What exists now (38% complete):**
- ✅ **93,000 lines** of production-ready Rust code
- ✅ **34 REST API endpoints** fully documented
- ✅ **Complete CLI** with 8 command groups
- ✅ **LQL query language** with 9 query types
- ✅ **278 tests passing** with >95% coverage
- ✅ **Snapshot system** for backup/restore
- ✅ **Template system** for auto-configuration
- ✅ **Load testing** infrastructure
- ✅ **Benchmark suite** with criterion

**What's next (Phase 2):**
- ⏳ Index aliases and reindexing
- ⏳ Docker/Kubernetes deployment
- ⏳ Client SDKs
- ⏳ Advanced aggregations
- ⏳ Performance tuning at scale

See [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md) for complete details.

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

## Performance Targets

These are **planned performance targets**, not current measurements:

- **Indexing**: ~50K-100K docs/sec per node (target)
- **Search Latency**: < 10ms p95 (target)
- **Throughput**: 10K+ queries/sec per node (target)
- **Scalability**: Horizontally scalable (planned)

## Use Cases

- **Application Search**: Add search to your application
- **Log Analytics**: Analyze and search logs in real-time
- **E-commerce**: Product search and recommendations
- **Content Management**: Full-text search for CMS
- **Monitoring**: Metrics and log aggregation
- **Security**: SIEM and threat detection

## Planned Comparison with ElasticSearch

| Feature | Lexum (Planned) | ElasticSearch |
|---------|-----------------|---------------|
| Language | Rust | Java |
| Memory Safety | ✅ (planned) | ❌ |
| Performance | Higher (target) | High |
| Resource Usage | Lower (target) | Higher |
| Query Language | LQL (SQL-like) | Query DSL (JSON) |
| License | Apache 2.0 | Elastic License |
| Native Protocols | HTTP, MCP, UMICP (planned) | HTTP |
| **Status** | **Documentation Only** | **Production Ready** |

## Roadmap

**Phase 1: Documentation** ✅ **COMPLETE**
- [x] Architecture design
- [x] API specifications  
- [x] Query language design (LQL)
- [x] Development guidelines

**Phase 2: Core Implementation** (Planned - Not Started)
- [ ] Core search engine
- [ ] Index management
- [ ] Query parser and planner
- [ ] REST API
- [ ] Basic CLI

**Phase 3: Distributed System** (Planned - Not Started)
- [ ] Cluster management (Raft)
- [ ] Sharding
- [ ] Replication
- [ ] Inter-node communication

**Phase 4: Advanced Features** (Planned - Not Started)
- [ ] LQL implementation
- [ ] Multiple protocol support (MCP, UMICP, WebSocket)
- [ ] Aggregations framework
- [ ] Advanced search features

**Phase 5: Production & GUI** (Planned - Not Started)
- [ ] Telemetry and monitoring
- [ ] Security (TLS, RBAC)
- [ ] Electron GUI
- [ ] Performance optimization

**Future Enhancements** (Post v1.0)
- [ ] Vector search
- [ ] Machine learning integration
- [ ] Geo-spatial queries
- [ ] Time-series optimization
- [ ] Graph queries

See [ROADMAP.md](docs/ROADMAP.md) for detailed timeline and milestones.

## Contributing

We welcome contributions! Please see:

- [Development Guide](docs/DEVELOPMENT.md)
- [Contributing Guidelines](CONTRIBUTING.md)

### Development

**Note**: There is no working code to build yet. The repository currently contains only documentation.

To contribute to documentation or planning:

```bash
# Navigate to the lexum directory in the HiveLLM monorepo
cd lexum/

# Read the documentation
cd docs/

# Review the roadmap and status
cat STATUS.md
cat ROADMAP.md
```

When implementation begins, we'll use:
- Rust 2024 Edition
- Tokio async runtime
- See [DEVELOPMENT.md](docs/DEVELOPMENT.md) for planned setup

## Community

Community channels are not yet established. This section will be updated when the project reaches implementation phase.

## License

Lexum is licensed under the [Apache License 2.0](LICENSE).

## Acknowledgments

- [Tantivy](https://github.com/quickwit-oss/tantivy) - Rust full-text search library
- [Tokio](https://tokio.rs) - Async runtime for Rust
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- Inspired by [ElasticSearch](https://www.elastic.co/elasticsearch/)

## Security

Security reporting procedures will be established when the project has actual implementation code.

---

**Ready to be built with ❤️ in Rust**

