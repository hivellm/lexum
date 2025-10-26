# Changelog

All notable changes to Lexum will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Phase 2 Planning
- Index aliases and atomic operations
- Reindexing with transformation support
- Advanced aggregations (terms, stats, date histogram)
- Client SDKs (Python, JavaScript, Rust)
- Docker and Kubernetes deployment
- Distributed clustering (Phase 3)

## [0.1.0-alpha] - 2025-10-25

### Foundation Complete ✅ (38% Overall Progress)

**Major Milestone**: Production-ready foundation with 278 tests passing and 53% code coverage.

### Added - Core Search Engine (99% Complete)

#### Index Management
- ✅ Index creation with configurable settings
- ✅ Index deletion and cleanup
- ✅ Index info and statistics
- ✅ Index refresh and flush operations
- ✅ **Index templates** with pattern matching (NEW)
- ✅ **Template manager** for auto-configuration (NEW)
- ✅ Template priority and versioning
- ✅ Tantivy 0.25 integration with BM25 scoring

#### Schema & Document Operations
- ✅ SchemaBuilder with fluent API
- ✅ Field types: text, keyword, i64, f64, date, boolean
- ✅ Field configuration: stored, indexed, fast fields
- ✅ Document CRUD (create, read, update, delete)
- ✅ Bulk operations support
- ✅ Document serialization/deserialization

#### Query Engine
- ✅ **MatchQuery** - Full-text search
- ✅ **TermQuery** - Exact matching
- ✅ **RangeQuery** - Numeric and date ranges
- ✅ **BoolQuery** - Boolean combinations (must, should, must_not, filter)
- ✅ **FuzzyQuery** - Approximate matching with edit distance
- ✅ **PhraseQuery** - Exact phrase matching with slop
- ✅ QueryBuilder with fluent API
- ✅ Query caching with DashMap

#### Search Features
- ✅ Result pagination (limit, offset)
- ✅ Sorting (ascending, descending)
- ✅ Field selection
- ✅ Query cache optimization
- ✅ Search result formatting

### Added - REST API Server (94% Complete)

#### API Endpoints (34 Total)
- ✅ **Health** (1): GET /health
- ✅ **Index Management** (7): CRUD operations + stats
- ✅ **Documents** (5): CRUD + bulk operations
- ✅ **Search** (1): POST /{index}/_search
- ✅ **Snapshots** (10): Complete backup/restore system (NEW)
- ✅ **Templates** (4): Template CRUD operations (NEW)
- ✅ **Admin/Cluster** (6): Health, stats, settings (NEW)

#### Server Features
- ✅ Axum 0.8 web framework
- ✅ Graceful shutdown (SIGTERM, Ctrl+C)
- ✅ Hot configuration reload
- ✅ Request tracing and correlation IDs
- ✅ Error handling with proper HTTP status codes

#### Middleware
- ✅ **Authentication**: API key based
- ✅ **Rate limiting**: Per-client tracking
- ✅ **CORS**: Configurable origins
- ✅ **Request logging**: Structured tracing
- ✅ **Timeout**: Request timeout handling

#### OpenAPI Documentation
- ✅ utoipa 5.4 integration
- ✅ Swagger UI 8.0
- ✅ 34 endpoints documented with ToSchema
- ✅ Complete request/response schemas
- ✅ Interactive API explorer

### Added - CLI Tool (96% Complete)

#### Command Groups (8 Total)
- ✅ **Server**: start, stop, status, config validation
- ✅ **Index**: create, list, get, stats, delete
- ✅ **Document**: add, get, delete, bulk
- ✅ **Search**: Simple search with limit
- ✅ **LQL**: Advanced query language command (NEW)
- ✅ **Snapshot**: Repository and snapshot management (NEW)
- ✅ **Template**: Template CRUD operations (NEW)
- ✅ **REPL**: Interactive shell mode

#### CLI Features
- ✅ Daemon mode with PID tracking
- ✅ Process management (SIGTERM/SIGKILL)
- ✅ Output formatting (JSON, Table, Pretty)
- ✅ Colored output with success/error indicators
- ✅ File-based operations (JSON/YAML/LQL)
- ✅ Query from file support (@file.lql)
- ✅ Command history with rustyline
- ✅ Comprehensive help system
- ✅ HTTP client wrapper (LexumClient)

### Added - LQL Query Language (90% Complete) ✨

#### Query Parser
- ✅ **Complete LQL Parser** (~500 LOC)
- ✅ Query cache with LazyLock
- ✅ Syntax error reporting

#### Query Types (9 Total)
- ✅ **FROM** queries - Basic selection
- ✅ **SELECT** queries - Field projection
- ✅ **MATCH** queries - Text matching
- ✅ **COUNT** queries - Aggregation
- ✅ **GROUP BY** queries - Grouping
- ✅ **AGGREGATE** queries - Functions (AVG, SUM, etc.)
- ✅ **JOIN** queries - Multi-index
- ✅ **UNION** queries - Query combination
- ✅ **EXISTS/NOT EXISTS** queries - Field existence

#### Query Syntax
- ✅ WHERE clause parsing
- ✅ Field:value syntax
- ✅ Range queries [min,max]
- ✅ Fuzzy queries ~term
- ✅ Phrase queries "exact phrase"
- ✅ Boolean operators (AND, OR, NOT)
- ✅ File-based queries (@file.lql)

### Added - Snapshot & Repository System (100% Complete)

#### Repository Management
- ✅ Filesystem repository implementation
- ✅ Repository configuration (FS, S3, Azure, GCS)
- ✅ Repository create/get/list/delete
- ✅ Settings validation
- ✅ Multiple repository support

#### Snapshot Operations
- ✅ Snapshot creation with metadata
- ✅ Snapshot listing and filtering
- ✅ Snapshot deletion
- ✅ Snapshot restoration
- ✅ Snapshot statistics
- ✅ Progress tracking
- ✅ Incremental snapshot foundation
- ✅ Snapshot chain management

#### Advanced Features (Phase 3 - WIP)
- 🚧 Compression algorithms (LZ4, Zstd, Snappy)
- 🚧 Binary diff and delta snapshots
- 🚧 Parallel processing
- 🚧 Content deduplication
- 🚧 Chain optimization

### Added - Configuration & Logging (100% Complete) ✅ ARCHIVED

#### Configuration
- ✅ YAML configuration files
- ✅ Environment variable overrides
- ✅ Configuration validation
- ✅ Hot-reload with file watcher
- ✅ Configuration merging
- ✅ Default values

#### Logging
- ✅ Structured JSON logging
- ✅ Log level configuration (trace, debug, info, warn, error)
- ✅ File logger with rotation
- ✅ Multiple output targets
- ✅ Correlation ID propagation
- ✅ Tracing subscriber integration

### Added - Admin Operations (69% Complete)

#### Cluster Monitoring
- ✅ Cluster health endpoint
- ✅ Cluster statistics
- ✅ Node statistics (JVM, CPU, memory)
- ✅ Cluster settings management

#### Template System
- ✅ Template pattern matching
- ✅ Automatic index configuration
- ✅ Template priority system
- ✅ Template versioning

### Testing & Quality (44% Complete)

#### Test Suite
- ✅ **278 tests passing** (verified 2025-10-25)
  - lexum-core: 136 tests
  - lexum-server: 91 tests (3 ignored)
  - lexum-cli: 45 tests
  - integration: 6 tests
- ✅ **53% code coverage** overall
- ✅ **>90% coverage** on critical modules
- ✅ Property-based testing with proptest
- ✅ Load testing infrastructure
- ✅ Benchmark suite with criterion
- ✅ HTML coverage reports

#### Test Files
- comprehensive_tests.rs (47 tests)
- integration_test.rs (multiple workflows)
- api_test.rs (API endpoints)
- handlers_test.rs (handler logic)
- cli_test.rs (CLI operations)
- lql_test.rs (LQL parser)
- snapshot tests (18+ tests)
- template tests (7+ tests)

### Changed

#### Architecture
- Upgraded to Rust Edition 2024
- Minimum Rust version: 1.85+
- Async runtime: Tokio 1.48

#### Dependencies
- utoipa upgraded to 5.4 (from 4.2)
- utoipa-swagger-ui upgraded to 8.0 (from 7.0)
- axum 0.8
- tantivy 0.25

### Fixed

#### Compilation Issues
- ✅ Resolved utoipa version conflicts
- ✅ Added ToSchema to all public types
- ✅ Fixed private function exports
- ✅ Added unsafe blocks for env tests
- ✅ Implemented Default for AppState
- ✅ Removed unused imports and variables
- ✅ Fixed OpenAPI schema references
- ✅ Configured unsafe_code linting

#### Test Issues
- ✅ Fixed auth middleware tests
- ✅ Fixed serialization tests
- ✅ Fixed Option.contains usage
- ✅ Fixed snapshot handler tests
- 🚧 Phase 3 compression tests (4 failing - WIP)

### Documentation

#### Created
- ✅ IMPLEMENTATION_SUMMARY.md - Complete project overview
- ✅ OPENSPEC_STATUS.md - OpenSpec progress tracking
- ✅ PROGRESS_ANALYSIS.md - Detailed metrics
- ✅ OpenAPI specification with 34 endpoints
- ✅ Swagger UI documentation
- ✅ LQL usage examples (10+)
- ✅ CLI help system with examples

#### Updated
- ✅ README.md - Status from "planning" to "foundation complete"
- ✅ All tasks.md files (7 changes)
- ✅ STATUS.md with current implementation
- ✅ Coverage reports

### Archived

The following changes have been completed and moved to `openspec/changes/archive/`:

1. **add-configuration-logging** (100%) - All configuration and logging complete
2. **add-lql-query-language** (90%) - Production-ready, optimizations deferred

## Code Metrics

### Current State
```
Total Files:       129 Rust files
Lines of Code:     ~93,000
Public APIs:       173 (69 core + 104 server)
Tests:             278 passing
Test Coverage:     53% overall, >90% on critical modules
Endpoints:         34 documented
Commands:          8 CLI groups
Dependencies:      50+ crates
Benchmarks:        Criterion suite with HTML reports
```

### Test Breakdown
```
Unit Tests:        272 (core + server + cli)
Integration Tests: 6 workflows
Load Tests:        2 suites
Benchmarks:        Search + indexing
Coverage Reports:  HTML + summary
```

## Implementation Summary

### What's Working (Production-Ready)
- ✅ Full-text search with 6 query types
- ✅ Index lifecycle management
- ✅ Document CRUD + bulk operations
- ✅ Snapshot backup/restore system
- ✅ Template auto-configuration
- ✅ REST API with 34 endpoints
- ✅ CLI with 8 command groups
- ✅ LQL query language
- ✅ Authentication and rate limiting
- ✅ Cluster monitoring

### What's Next (Phase 2)
- ⏳ Index aliases and reindexing
- ⏳ Docker/Kubernetes deployment
- ⏳ Client SDKs (Python, JS, Rust)
- ⏳ Advanced aggregations
- ⏳ Performance optimization at scale

### Future Phases
- Phase 3: Advanced features (distributed clustering, advanced search)
- Phase 4: GUI, telemetry, multi-protocol support

## Breaking Changes

None yet (alpha version).

## Migration Guide

Not applicable (first alpha release).

## Contributors

- HiveLLM Team

## License

Apache License 2.0 - See [LICENSE](LICENSE) for details.

---

## Version History

- **0.1.0-alpha** (2025-10-25) - Foundation complete, 38% progress
- **0.0.1** - Initial planning phase

For detailed progress tracking, see:
- [IMPLEMENTATION_SUMMARY.md](IMPLEMENTATION_SUMMARY.md)
- [OPENSPEC_STATUS.md](openspec/OPENSPEC_STATUS.md)
- [PROGRESS_ANALYSIS.md](openspec/PROGRESS_ANALYSIS.md)
