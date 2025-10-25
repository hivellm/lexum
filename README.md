# Lexum

[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Status](https://img.shields.io/badge/status-planning-yellow.svg)](docs/STATUS.md)

**Lexum** is a planned high-performance, distributed full-text search engine to be written in Rust, inspired by ElasticSearch but designed from the ground up for modern cloud-native architectures.

> **⚠️ Project Status**: Currently in **planning/documentation phase**. No implementation code has been written yet. See [STATUS.md](docs/STATUS.md) for details.

## Planned Features

- 🚀 **High Performance**: To be built with Rust and Tokio for maximum throughput and minimal latency
- 🔍 **Full-Text Search**: Advanced indexing and search powered by Tantivy
- 🌐 **Distributed**: Native support for sharding and replication
- 💬 **LQL**: Powerful SQL-like query language (Lexum Query Language)
- 🔌 **Multiple Protocols**: StreamableHTTP, MCP, and UMICP support
- 🖥️ **Modern GUI**: Electron-based interface similar to Kibana
- 📊 **Observability**: Comprehensive telemetry, metrics, and distributed tracing
- 🐳 **Cloud Native**: Docker and Kubernetes ready
- 🔒 **Secure**: TLS, authentication, and role-based access control

## Current State

**What exists now:**
- ✅ Comprehensive technical documentation
- ✅ Architecture design
- ✅ API specifications
- ✅ LQL query language design
- ✅ Deployment strategies
- ✅ Development guidelines

**What doesn't exist yet:**
- ❌ No working code/implementation
- ❌ No binaries or packages
- ❌ No running server
- ❌ No actual search functionality

See the [ROADMAP](docs/ROADMAP.md) for the planned implementation timeline.

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

