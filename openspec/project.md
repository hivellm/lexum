# Lexum Project

## Overview

Lexum is a high-performance, distributed full-text search engine written in Rust, designed as a modern alternative to ElasticSearch with enhanced capabilities for AI/LLM integration.

## Project Conventions

### Language & Tooling
- **Language**: Rust 2024 Edition
- **Toolchain**: Nightly 1.85+
- **Testing**: >95% coverage required
- **Linting**: clippy with -D warnings
- **Formatting**: rustfmt (nightly)

### Architecture Principles
1. **Simplicity First**: Default to <100 lines of new code per feature
2. **Async by Default**: Use Tokio for all I/O operations
3. **Strong Typing**: Leverage Rust's type system
4. **Error Handling**: Use thiserror for errors, anyhow for applications
5. **Documentation**: All public APIs must have doc comments with examples

### Performance Targets
- **v0.1** (Single Node): 10K docs/sec indexing, <50ms p95 search
- **v0.2** (3-Node Cluster): 30K docs/sec indexing, <30ms p95 search
- **v0.3** (Optimized): 50K docs/sec indexing, <20ms p95 search
- **v1.0** (Production): 100K docs/sec indexing, <10ms p95 search

### Code Organization

```
lexum/
├── lexum-core/          # Core search engine library
│   ├── config/          # Configuration management
│   ├── error/           # Error types
│   ├── types/           # Common types
│   ├── storage/         # Storage layer
│   ├── index/           # Index management
│   ├── document/        # Document operations
│   ├── query/           # Query types and LQL
│   ├── search/          # Search execution
│   ├── cluster/         # Clustering (Phase 2)
│   └── aggregation/     # Aggregations
├── lexum-server/        # HTTP server
│   ├── api/             # API endpoints
│   │   ├── rest/        # REST API
│   │   ├── mcp/         # MCP handler
│   │   └── umicp/       # UMICP handler
│   ├── gateway/         # API gateway
│   └── telemetry/       # Observability
├── lexum-cli/           # Command-line tool
└── lexum-gui/           # Electron GUI (Phase 5)
```

### Testing Strategy
- **Unit Tests**: In-file with #[cfg(test)]
- **Integration Tests**: In /tests directory
- **Benchmarks**: Using criterion in /benchmark
- **Coverage**: >95% required for all code

### Documentation Standards
- **Root Level**: Only README, CHANGELOG, CONTRIBUTING, LICENSE, AGENTS.md
- **Technical Docs**: All in /docs directory
- **API Docs**: Generated from code comments
- **OpenSpec**: All proposals in /openspec/changes

### Change Management
- All significant changes require OpenSpec proposal
- Bug fixes and typos can be done directly
- Breaking changes MUST be clearly marked
- Migration paths required for breaking changes

### Dependencies
- Verify latest versions with Context7 before adding
- Document why each dependency is chosen
- Prefer well-maintained crates
- Minimize dependency count

## Current Phase

**Status**: Phase 1 - Planning and Documentation (Complete)  
**Next**: Phase 2 - Core Implementation

## Active Changes

See `openspec/changes/` directory for active proposals:
1. `add-core-search-engine` - Foundation search functionality
2. `add-rest-api` - HTTP REST API
3. `add-distributed-clustering` - Multi-node clustering
4. `add-lql-query-language` - Query language implementation

## Project Timeline

| Phase | Version | Target | Focus |
|-------|---------|--------|-------|
| 1 | v0.1.0-alpha | Month 3 | Core search engine |
| 2 | v0.2.0-alpha | Month 6 | Distributed clustering |
| 3 | v0.3.0-beta | Month 9 | Advanced features (LQL, protocols) |
| 4 | v0.4.0-rc | Month 12 | Observability & ops |
| 5 | v0.5.0 | Month 15 | GUI & tooling |
| 6 | v0.9.0 | Month 18 | Production hardening |
| 7 | v1.0.0 | Month 21 | Stable release |

## Quality Gates

### Before Any Commit
- [ ] Code formatted: `cargo +nightly fmt --all`
- [ ] Clippy clean: `cargo clippy --workspace -- -D warnings`
- [ ] All tests pass: `cargo test --workspace`
- [ ] Coverage >95%: `cargo llvm-cov --all`

### Before Creating Tag
- [ ] All commit checks pass
- [ ] No typos: `codespell`
- [ ] Security clean: `cargo audit`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Version bumped correctly

## Capability Naming

Use verb-noun pattern:
- `core-search` - Core search functionality
- `index-management` - Index operations
- `document-operations` - Document CRUD
- `query-execution` - Query processing
- `cluster-management` - Clustering
- `shard-management` - Sharding
- `lql-language` - LQL implementation

## Communication

- **Issues**: Feature requests and bugs
- **Discussions**: Design discussions
- **Pull Requests**: Code changes with OpenSpec reference
- **Discord**: Real-time collaboration

## References

- [ROADMAP.md](/docs/ROADMAP.md) - Project roadmap
- [ARCHITECTURE.md](/docs/ARCHITECTURE.md) - System architecture
- [DAG.md](/docs/DAG.md) - Component dependencies
- [DEVELOPMENT.md](/docs/DEVELOPMENT.md) - Development guide
- [AGENTS.md](/AGENTS.md) - AI assistant rules
- [OpenSpec AGENTS.md](./AGENTS.md) - OpenSpec instructions
