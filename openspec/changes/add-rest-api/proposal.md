## Why

Lexum needs an HTTP REST API to expose search functionality to clients. This is the primary interface for users to interact with the search engine. Without it, the core search engine cannot be accessed externally.

## What Changes

- Add Axum-based REST API server
- Implement index management endpoints (PUT, GET, DELETE)
- Implement document endpoints (POST, GET, PUT, DELETE)
- Implement search endpoint (POST)
- Add bulk operations endpoint
- Implement cluster health endpoint
- Add request validation and error handling
- Implement authentication and authorization middleware
- Add rate limiting
- Implement request/response logging

## Impact

- Affected specs: `rest-api`, `index-management`, `document-operations`, `search-api`
- Affected code: Creates new `lexum-server/src/api/rest/` with:
  - `index.rs` - Index endpoints
  - `document.rs` - Document endpoints
  - `search.rs` - Search endpoints
  - `cluster.rs` - Cluster endpoints
  - `middleware/` - Auth, logging, rate limiting
- Dependencies: axum, tower, tower-http, hyper
- Performance target: 1K requests/sec, <10ms p95 routing overhead
- Must integrate with core-search engine from Phase 1

