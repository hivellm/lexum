# Lexum

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-alpha-brightgreen.svg)](docs/STATUS.md)
[![Tests](https://img.shields.io/badge/tests-278%20passing-success.svg)](docs/IMPLEMENTATION_SUMMARY.md)
[![Coverage](https://img.shields.io/badge/coverage-%3E95%25-success.svg)](docs/IMPLEMENTATION_SUMMARY.md)
[![Roadmap](https://img.shields.io/badge/roadmap-14%20phases-blue.svg)](docs/ROADMAP.md)

**Lexum** is a high-performance, distributed full-text search engine written in Rust, inspired by ElasticSearch but designed from the ground up for modern cloud-native architectures.

> **✅ Project Status**: **Foundation Complete** (38% overall). Core search engine, REST API, CLI tool, and LQL query language are production-ready with 278 tests passing and >95% coverage. See [docs/IMPLEMENTATION_SUMMARY.md](docs/IMPLEMENTATION_SUMMARY.md) for details.

## Features

### ✅ Implemented (Production-Ready)

- 🚀 **High Performance**: Built with Rust Edition 2024 and Tokio for maximum throughput
- 🔍 **Full-Text Search**: Advanced indexing and search powered by Tantivy 0.25
- 💬 **LQL**: Fully functional SQL-like query language with 9 query types
- 📡 **REST API**: 39 working endpoints (100% success rate) with OpenAPI/Swagger UI
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

#### One-Liner Installation (Recommended)

**Linux / macOS:**

```bash
curl -fsSL https://raw.githubusercontent.com/hivellm/lexum/main/install.sh | bash
```

**Windows:**

```powershell
powershell -c "irm https://raw.githubusercontent.com/hivellm/lexum/main/install.ps1 | iex"
```

The installation script will:

- ✅ Install dependencies (Git, Rust, build tools)
- ✅ Clone and compile Lexum from source
- ✅ Set up Lexum as a system service (auto-starts on boot)
- ✅ Make CLI available in your PATH
- ✅ Configure default settings

After installation:

- **Linux**: Service runs as `lexum` user, CLI available as `lexum`
- **Windows**: Service runs as Windows Service, CLI available as `lexum-server`

See [scripts/INSTALL.md](scripts/INSTALL.md) for detailed instructions and troubleshooting.

#### Manual Installation

```bash
# Clone repository
git clone https://github.com/hivellm/lexum.git
cd lexum

# Build all crates
cargo build --release

# Run server
./target/release/lexum-server

# Use CLI
./target/release/lexum-server --help
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
curl http://localhost:17000/health

# Create index
curl -X PUT http://localhost:17000/api/v1/indices/products \
  -H "Content-Type: application/json" \
  -d '{"fields": [{"name": "title", "type": "text"}], "settings": {}}'

# Search
curl -X POST http://localhost:17000/api/v1/indices/products/search \
  -H "Content-Type: application/json" \
  -d '{"query": {"match": {"field": "title", "query": "laptop"}}, "limit": 10}'

# Swagger UI
open http://localhost:17000/swagger-ui
```

## Current State

**What exists now (38% complete):**

- ✅ **93,000 lines** of production-ready Rust code
- ✅ **39 REST API endpoints** fully working (100% success rate)
- ✅ **Complete CLI** with 8 command groups
- ✅ **LQL query language** with 9 query types
- ✅ **278 tests passing** with >95% coverage
- ✅ **Snapshot system** for backup/restore
- ✅ **Template system** for auto-configuration
- ✅ **Load testing** infrastructure
- ✅ **Benchmark suite** with criterion
- ✅ **API stability** - All routes tested and validated

**What's next (Phase 2):**

- ⏳ Index aliases and reindexing
- ⏳ Docker/Kubernetes deployment
- ⏳ Client SDKs
- ⏳ Advanced aggregations
- ⏳ Performance tuning at scale

See [docs/IMPLEMENTATION_SUMMARY.md](docs/IMPLEMENTATION_SUMMARY.md) for complete details.

## Documentation

- **[Overview](docs/README.md)** - Introduction and features
- **[Architecture](docs/ARCHITECTURE.md)** - System design and components
- **[Query Language](docs/api/QUERY_LANGUAGE.md)** - LQL specification
- **[API Reference](docs/api/API_REFERENCE.md)** - Complete API documentation
- **[Deployment](docs/deployment/DEPLOYMENT.md)** - Docker and Kubernetes deployment
- **[Telemetry](docs/deployment/TELEMETRY.md)** - Monitoring and observability
- **[GUI](docs/guides/GUI.md)** - Electron-based interface
- **[Development](docs/development/DEVELOPMENT.md)** - Development guide
- **[CI/CD](docs/development/CI_CD.md)** - Build and deployment pipelines

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

✅ **Current capabilities:**

- **Application Search**: Embed full-text search in applications
- **Document Management**: Index and search documents
- **Data Exploration**: Use LQL for complex queries
- **Development/Testing**: Rapid prototyping with CLI

📋 **Planned:**

- Log Analytics
- E-commerce product search
- Content Management Systems
- Security/SIEM
- Monitoring and metrics

## Comparison with ElasticSearch

| Feature          | Lexum (v0.1.0-alpha)   | ElasticSearch       |
| ---------------- | ---------------------- | ------------------- |
| Language         | Rust Edition 2024      | Java                |
| Memory Safety    | ✅ Guaranteed          | ❌                  |
| Performance      | Good (optimizing)      | Excellent           |
| Resource Usage   | Lower                  | Higher              |
| Query Language   | **LQL** (SQL-like) ✅  | Query DSL (JSON)    |
| License          | Apache 2.0             | Elastic License 2.0 |
| Native Protocols | HTTP/REST ✅           | HTTP/REST           |
| Snapshot/Backup  | ✅ Implemented         | ✅                  |
| Templates        | ✅ Implemented         | ✅                  |
| CLI Tool         | ✅ Full-featured       | Basic               |
| API Stability    | ✅ 100% routes working | ✅                  |
| **Status**       | **Alpha (38%)**        | **Production**      |

## Roadmap

### ✅ Phase 1: Foundation (COMPLETE - 38%)

**Completed Components:**

- [x] Core search engine with Tantivy
- [x] Index management + templates
- [x] Query engine (6 types)
- [x] REST API (39 endpoints, 100% working)
- [x] CLI tool (8 command groups)
- [x] LQL query language
- [x] Configuration & logging
- [x] Snapshot/restore system
- [x] Test suite (278 tests)
- [x] OpenAPI documentation
- [x] API route stability (all bugs fixed)

### 🚧 Phase 2: Advanced Features (Started - 10%)

**In Progress:**

- [x] Admin operations (69% - snapshots, templates, monitoring)
- [x] Performance optimization (30% - infrastructure ready)
- [ ] Index aliases
- [ ] Reindexing operations
- [ ] Docker/Kubernetes deployment
- [ ] Client SDKs (Python, JavaScript, Rust)

### 📋 Phase 3: Distributed & Scale (Planned)

- [ ] Distributed clustering with Raft
- [ ] Sharding and replication
- [ ] Advanced aggregations
- [ ] Advanced search features
- [ ] Security enhancements
- [ ] Telemetry and observability

### 📋 Phase 4: Production & GUI (Planned)

- [ ] Electron-based GUI
- [ ] Multi-protocol support (MCP, UMICP)
- [ ] Production deployment tools
- [ ] Performance tuning at scale

### 📋 Future (Post v1.0)

- [ ] Vector search
- [ ] Machine learning ranking
- [ ] Geo-spatial queries
- [ ] Time-series optimization
- [ ] Graph traversal

See [ROADMAP.md](docs/ROADMAP.md) and the task tracker in [.rulebook/tasks/](.rulebook/tasks/README.md) for details.

## Project Structure

```
lexum/
├── crates/
│   ├── lexum-core/      # Core search engine (23 modules, 40K LOC)
│   │   ├── config/      # Configuration management
│   │   ├── document/    # Document operations
│   │   ├── index/       # Index and template management
│   │   ├── query/       # Query types and builder
│   │   ├── schema/      # Schema builder
│   │   ├── search/      # Search executor
│   │   └── snapshot/    # Snapshot and repository
│   ├── lexum-server/    # REST API server (15 modules, 30K LOC)
│   │   ├── handlers/    # API endpoint handlers
│   │   ├── middleware/  # Auth, rate limiting, CORS
│   │   └── openapi/     # OpenAPI specification
│   └── lexum-macros/    # Procedural macros
├── tests/               # Integration, e2e, and stress tests
├── benchmark/           # Performance benchmarks
└── docs/                # Documentation

Total: 129 Rust files, ~93,000 LOC
```

## Development

### Prerequisites

- Rust 1.85+ (Edition 2024)
- Cargo
- Optional: cargo-llvm-cov for coverage

### Platform Requirements

⚠️ **Important**: Due to Tantivy/WSL compatibility limitations, all builds, tests, and development operations **MUST** be performed in **PowerShell (Windows native)**, **NOT** in WSL.

**Why?**

- Tantivy has filesystem compatibility issues with WSL's `9p` protocol when accessing Windows-mounted drives
- This causes `Invalid argument (os error 22)` errors during index creation
- See [WSL_TANTIVY_CONFLICT.md](docs/development/WSL_TANTIVY_CONFLICT.md) for technical details

**Solutions:**

1. **Use Windows Native (Recommended)**: Run all commands in PowerShell with Windows paths (`C:\Users\...`)
2. **Use Linux Native Paths in WSL**: Store data in WSL's native filesystem (`~/.lexum/data`) instead of `/mnt/f/...`
3. **Use Docker**: Run Lexum in a Docker container for better filesystem isolation

See [WINDOWS_NATIVE.md](docs/development/WINDOWS_NATIVE.md) for Windows setup instructions.

### Building

**⚠️ Run these commands in PowerShell (Windows native), not WSL:**

```powershell
# Clone repository
git clone https://github.com/hivellm/lexum.git
cd lexum

# Build all crates
cargo build --release

# Run tests
cargo test --all-features

# Run with coverage
cargo llvm-cov --html

# Run benchmarks
cargo bench
```

**Note**: If you're using WSL, ensure data directories use Linux native paths (e.g., `~/.lexum/data`) instead of Windows-mounted drives (`/mnt/f/...`).

### Running

**⚠️ Run these commands in PowerShell (Windows native), not WSL:**

```powershell
# Set data directory to Windows native path (if needed)
$env:LEXUM_DATA_DIR = "C:\Users\YourUser\lexum-data"

# Start server
cargo run --bin lexum-server

# Or use CLI
cargo run --bin lexum-cli -- server start

# Interactive mode
cargo run --bin lexum-cli repl
```

**Troubleshooting**: If you encounter `Invalid argument (os error 22)` errors, see [TROUBLESHOOTING.md](docs/guides/TROUBLESHOOTING.md) for solutions.

## Contributing

We welcome contributions! Please see:

- [Development Guide](docs/development/DEVELOPMENT.md)
- [Contributing Guidelines](CONTRIBUTING.md)
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)

### Development Status

✅ **Ready for contributions** in:

- Bug fixes and improvements
- Test coverage expansion
- Documentation updates
- Performance optimization
- New features (see Phase 2 roadmap)

## License

Lexum is licensed under the [Apache License 2.0](LICENSE).

## Acknowledgments

- [Tantivy](https://github.com/quickwit-oss/tantivy) - Full-text search library (0.25)
- [Tokio](https://tokio.rs) - Async runtime (1.48)
- [Axum](https://github.com/tokio-rs/axum) - Web framework (0.8)
- [utoipa](https://github.com/juhaku/utoipa) - OpenAPI generation (5.4)
- [clap](https://github.com/clap-rs/clap) - CLI parsing (4.5)
- Inspired by [ElasticSearch](https://www.elastic.co/elasticsearch/)

## Community

- **Issues**: [GitHub Issues](https://github.com/hivellm/lexum/issues)
- **Discussions**: [GitHub Discussions](https://github.com/hivellm/lexum/discussions)
- **Documentation**: [docs/](docs/)

## Security

For security issues, please see [SECURITY.md](SECURITY.md).

## Links

- 📊 [Implementation Summary](docs/IMPLEMENTATION_SUMMARY.md) - Complete overview
- 📈 [Project Status](docs/STATUS.md) - Current state
- 🎯 [Task Tracker](.rulebook/tasks/README.md) - Phased task board
- 🔬 [Analyses](docs/analysis/) - Elasticsearch and Meilisearch studies
- 📝 [Changelog](CHANGELOG.md) - Version history
- 🗺️ [Roadmap](docs/ROADMAP.md) - Future plans

---

**Built with ❤️ in Rust Edition 2024**

**Status**: Foundation Complete — re-planned 2026-07-17 | **Roadmap**: [14 phases](docs/ROADMAP.md) | **Tests**: 278 passing
