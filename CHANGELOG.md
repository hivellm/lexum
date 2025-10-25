# Changelog

All notable changes to Lexum will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Vector search support
- Machine learning-based ranking
- Geo-spatial queries
- Graph traversal queries
- Advanced time-series features

## [0.1.0] - TBD

### Added
- Core search engine based on Tantivy
- Distributed clustering with Raft consensus
- LQL (Lexum Query Language) - SQL-inspired query language
- REST API with full-text search capabilities
- StreamableHTTP protocol support
- MCP (Model Context Protocol) integration
- UMICP (Universal Model Interchange Communication Protocol) support
- Sharding and replication
- Real-time indexing
- Aggregations framework (terms, stats, histograms)
- OpenTelemetry integration for observability
- Prometheus-compatible metrics
- Distributed tracing with Jaeger
- Structured JSON logging
- Electron-based GUI (Lexum UI)
- Docker support
- Kubernetes deployment with Helm charts
- TLS/mTLS support
- Role-based access control (RBAC)
- API key authentication
- Comprehensive documentation
- CI/CD pipelines with GitHub Actions

### Features

#### Search
- BM25 scoring
- Fuzzy matching
- Phrase queries
- Boolean queries
- Range queries
- Wildcard queries
- Query caching
- Filter caching

#### Index Management
- Dynamic and strict schemas
- Multiple field types (text, keyword, integer, float, date, etc.)
- Custom analyzers
- Index templates
- Index aliases
- Reindexing

#### Cluster Management
- Automatic node discovery
- Shard allocation
- Replica management
- Cluster health monitoring
- Rolling upgrades
- Snapshot and restore

#### Query Language (LQL)
- SQL-like syntax
- Pipe-based operation chaining
- Full-text search operators
- Aggregations
- Window functions
- Subqueries

#### Protocols
- HTTP/1.1 and HTTP/2
- WebSocket for real-time updates
- Server-Sent Events (SSE) for streaming
- MCP for AI/LLM integration
- UMICP for high-performance binary protocol

#### Observability
- Request/response metrics
- Search performance metrics
- Indexing metrics
- Cluster metrics
- System metrics (CPU, memory, disk)
- Distributed tracing
- Slow query logging
- Health checks

#### GUI Features
- Search and discover interface
- Dashboard builder
- Index management
- User and role management
- Real-time monitoring
- Log viewer
- LQL query console

### Security
- TLS 1.3 support
- mTLS for inter-node communication
- API key authentication
- OAuth 2.0 integration
- Role-based access control
- Document-level security
- Field-level security
- Audit logging

### Performance
- Concurrent query execution
- Parallel shard search
- Memory-mapped index files
- Compression (stored fields, network)
- Connection pooling
- Request batching

### Deployment
- Docker images for multiple architectures
- Kubernetes StatefulSets
- Helm charts
- Health and readiness probes
- Horizontal Pod Autoscaling support
- PersistentVolume support

### Documentation
- Architecture documentation
- API reference
- Query language specification
- Deployment guides (Docker, Kubernetes, bare metal)
- Telemetry and monitoring guide
- GUI documentation
- Development guide
- CI/CD documentation

### Testing
- Unit tests
- Integration tests
- Performance benchmarks
- End-to-end tests
- Load testing

### CI/CD
- Automated testing on multiple platforms
- Code coverage reporting
- Security scanning
- Dependency auditing
- Multi-platform builds
- Automated releases
- Docker image publishing

## Version History

### [0.1.0] - Initial Release
First public release of Lexum search engine.

---

## Guidelines

### Types of Changes
- `Added` for new features
- `Changed` for changes in existing functionality
- `Deprecated` for soon-to-be removed features
- `Removed` for now removed features
- `Fixed` for any bug fixes
- `Security` for vulnerability fixes

### Version Format
- MAJOR.MINOR.PATCH
- MAJOR: Breaking changes
- MINOR: New features, backwards compatible
- PATCH: Bug fixes, backwards compatible

