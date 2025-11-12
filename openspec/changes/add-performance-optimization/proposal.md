## Why

To meet production performance targets (100K docs/sec indexing, <10ms p95 search), Lexum requires systematic performance optimization across caching, memory management, concurrency, and I/O operations.

## What Changes

- Optimize query cache with better eviction policies
- Implement filter cache for common filters
- Add field cache optimization for sorting/aggregations
- Optimize memory management with arena allocation
- Implement object pooling for expensive objects
- Add memory-mapped index files
- Optimize compression (stored fields, network)
- Implement connection pooling
- Add request batching
- Optimize concurrent query execution

## Impact

- Affected specs: `performance-optimization`
- Affected code: Optimizations throughout `lexum-core/`:
  - `cache/` - Cache implementations
  - `memory/` - Memory management
  - `pool/` - Connection and object pooling
- Must maintain API compatibility
- Target: 10x improvement in some operations

