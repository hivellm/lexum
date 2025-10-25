## Context

Lexum requires a robust search engine foundation that can handle full-text search at scale. This is a greenfield implementation where we're integrating Tantivy as the underlying search library while building a clean abstraction layer that will support future distributed operations.

**Constraints:**
- Must use Rust 2024 edition
- Must achieve >95% test coverage
- Performance: 10K docs/sec indexing, <50ms p95 search
- Must be async-first with Tokio

**Stakeholders:**
- Development team (implementation)
- Future API consumers (clean interface)
- Operations team (performance, observability)

## Goals / Non-Goals

**Goals:**
- Provide production-ready full-text search
- Clean, maintainable abstraction over Tantivy
- High performance indexing and search
- Comprehensive error handling
- Async-first design

**Non-Goals:**
- Distributed clustering (Phase 2)
- Advanced query language (Phase 3)
- REST API (separate component)
- GUI (Phase 5)

## Decisions

### Decision 1: Tantivy as Search Library

**What:** Use Tantivy as the core search engine instead of building from scratch.

**Why:**
- Proven performance (Lucene-inspired)
- Pure Rust implementation (no FFI overhead)
- Active development and maintenance
- Rich feature set (BM25, faceting, etc.)
- Good documentation

**Alternatives Considered:**
- **ElasticSearch Rust client**: Rejected - adds deployment complexity, not embeddable
- **Custom implementation**: Rejected - reinventing the wheel, months of work
- **MeiliSearch**: Rejected - less flexible, focused on different use case

### Decision 2: Async Abstraction Layer

**What:** Build async wrappers around Tantivy (which is sync).

**Why:**
- Enable non-blocking operations in server context
- Better resource utilization
- Consistent with modern Rust ecosystem
- Required for future distributed features

**Implementation:**
```rust
pub async fn search(&self, query: Query) -> Result<SearchResults> {
    let index = self.index.clone();
    tokio::task::spawn_blocking(move || {
        // Tantivy sync operations
    }).await?
}
```

### Decision 3: Error Handling with thiserror

**What:** Use thiserror for error types, anyhow for application errors.

**Why:**
- thiserror: Clean error enum definition
- anyhow: Easy error propagation in applications
- Standard pattern in Rust ecosystem

```rust
#[derive(Error, Debug)]
pub enum LexumError {
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    #[error("Query parsing error: {0}")]
    QueryParseError(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
```

### Decision 4: Field Type System

**What:** Support 6 core field types: text, keyword, i64, f64, date, boolean.

**Why:**
- Covers 95% of use cases
- Maps cleanly to Tantivy types
- Simple to understand and use
- Can extend later if needed

**Mapping:**
```
text    -> Tantivy Text (analyzed)
keyword -> Tantivy Text (unanalyzed)
i64     -> Tantivy I64
f64     -> Tantivy F64
date    -> Tantivy Date
boolean -> Tantivy U64 (0/1)
```

### Decision 5: Query Cache Strategy

**What:** Implement LRU cache for query results with configurable size.

**Why:**
- Repeated queries are common
- Significant performance improvement
- Memory bound is controllable

**Implementation:**
- Use `lru` crate
- Default: 1000 entries
- Cache key: hash of query + index + options
- Invalidate on index updates

## Architecture

### Module Structure

```
lexum-core/
├── src/
│   ├── lib.rs              # Public API
│   ├── config/             # Configuration
│   ├── error/              # Error types
│   ├── types/              # Common types
│   ├── logging/            # Logging setup
│   ├── storage/            # Storage abstraction
│   ├── index/              # Index management
│   ├── document/           # Document operations
│   ├── schema/             # Schema definitions
│   ├── query/              # Query types
│   └── search/             # Search execution
└── tests/                  # Integration tests
```

### Core Abstractions

```rust
// Index management
pub struct Index { /* ... */ }
impl Index {
    pub async fn create(settings: IndexSettings) -> Result<Self>;
    pub async fn delete(&self) -> Result<()>;
    pub async fn info(&self) -> IndexInfo;
}

// Document operations
pub trait DocumentStore {
    async fn add(&self, doc: Document) -> Result<DocumentId>;
    async fn get(&self, id: &DocumentId) -> Result<Document>;
    async fn update(&self, id: &DocumentId, doc: Document) -> Result<()>;
    async fn delete(&self, id: &DocumentId) -> Result<()>;
}

// Search
pub struct SearchExecutor { /* ... */ }
impl SearchExecutor {
    pub async fn search(&self, query: Query) -> Result<SearchResults>;
}
```

## Risks / Trade-offs

### Risk: Tantivy Breaking Changes
- **Mitigation**: Pin to specific version, test upgrades thoroughly
- **Fallback**: Can fork if necessary (Apache 2.0 license)

### Risk: Async Overhead
- **Impact**: spawn_blocking adds slight overhead
- **Mitigation**: Benchmark to ensure within targets
- **Acceptable**: Trade-off for better server scalability

### Risk: Memory Usage
- **Impact**: Large indices can consume significant memory
- **Mitigation**: 
  - Configure cache sizes
  - Monitor memory usage
  - Add memory limits in settings
  - Document memory requirements

### Trade-off: Abstraction vs Performance
- **Decision**: Prioritize clean API over micro-optimizations
- **Rationale**: Easier to optimize later than to fix bad API
- **Validation**: Benchmarks must still meet targets

## Performance Targets

### Indexing
- Throughput: 10,000 docs/sec (single node)
- Latency: < 100ms p95 for single document
- Bulk: > 50,000 docs/sec

### Search
- Latency: < 50ms p95 for simple queries
- Latency: < 200ms p95 for complex queries
- Throughput: > 1,000 queries/sec

### Resource Usage
- Memory: < 2GB base + index size
- CPU: < 50% on 4-core machine under load
- Disk: 1.5x original data size

## Migration Plan

N/A - This is the initial implementation.

## Testing Strategy

### Unit Tests
- Every public function
- Error cases
- Edge cases (empty strings, null values)
- Target: >95% coverage

### Integration Tests
- Complete workflows (index → search)
- Multiple indices
- Concurrent operations
- Recovery scenarios

### Performance Tests
- Benchmark suite with criterion
- Load testing with realistic data
- Memory profiling
- Regression detection

### Property-based Tests
- Use proptest for:
  - Query parsing
  - Document serialization
  - Index operations

## Open Questions

1. **Q**: Should we support custom analyzers in v0.1?
   **A**: No, defer to Phase 3. Use Tantivy defaults for now.

2. **Q**: What's the maximum document size?
   **A**: 100MB per document (configurable), consistent with ElasticSearch.

3. **Q**: How to handle index schema changes?
   **A**: Require reindexing in v0.1. Support migrations in Phase 2.

4. **Q**: Should we support geospatial queries?
   **A**: No, defer to Phase 6 (future enhancement).

