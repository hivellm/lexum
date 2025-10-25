## Why

Lexum needs a foundational search engine to provide full-text search capabilities. This is the core functionality that all other features depend on. Without this, there is no search product.

## What Changes

- Add Tantivy integration for full-text search
- Implement index management (create, delete, configure)
- Add document operations (index, get, update, delete)
- Implement basic query types (match, term, range, boolean)
- Add BM25 scoring algorithm
- Implement field type support (text, keyword, integer, float, date)
- Add query result caching
- Implement document storage and retrieval

## Impact

- Affected specs: `core-search`, `index-management`, `document-operations`
- Affected code: Creates new `lexum-core` crate with:
  - `src/index/` - Index management
  - `src/search/` - Search engine
  - `src/document/` - Document store
  - `src/query/` - Query types
  - `src/storage/` - Storage layer
- Dependencies: tantivy, tokio, serde
- Performance target: 10K docs/sec indexing, <50ms p95 search latency
- Test coverage requirement: >95%

