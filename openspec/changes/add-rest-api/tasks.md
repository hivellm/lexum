## 1. Server Setup
- [x] 1.1 Create lexum-server crate
- [x] 1.2 Add Axum and Tower dependencies
- [x] 1.3 Implement basic server configuration
- [x] 1.4 Setup graceful shutdown
- [x] 1.5 Add health check endpoint

## 2. Index Management Endpoints
- [x] 2.1 Implement PUT /{index} - Create index
- [x] 2.2 Implement GET /{index} - Get index info
- [x] 2.3 Implement DELETE /{index} - Delete index
- [x] 2.4 Implement GET /_cat/indices - List indices
- [x] 2.5 Add request validation
- [x] 2.6 Add integration tests

## 3. Document Endpoints
- [x] 3.1 Implement POST /{index}/_doc - Index document (auto ID)
- [x] 3.2 Implement PUT /{index}/_doc/{id} - Index with ID
- [x] 3.3 Implement GET /{index}/_doc/{id} - Get document
- [x] 3.4 Implement POST /{index}/_update/{id} - Update document
- [x] 3.5 Implement DELETE /{index}/_doc/{id} - Delete document
- [x] 3.6 Add validation and error handling
- [x] 3.7 Add tests

## 4. Bulk Operations
- [x] 4.1 Implement POST /_bulk endpoint
- [x] 4.2 Support index, create, update, delete operations
- [x] 4.3 Add NDJSON parsing
- [x] 4.4 Implement batch processing
- [x] 4.5 Add error handling per operation
- [x] 4.6 Add performance tests
- [x] 4.7 Add ToSchema for bulk response types

## 5. Search Endpoints
- [x] 5.1 Implement POST /{index}/_search
- [x] 5.2 Support query DSL parsing
- [x] 5.3 Add pagination parameters
- [x] 5.4 Add sorting parameters
- [x] 5.5 Add field selection
- [x] 5.6 Implement result formatting
- [x] 5.7 Add search tests
- [x] 5.8 Add ToSchema for SearchRequest

## 6. Snapshot & Repository Endpoints
- [x] 6.1 Implement PUT /_snapshot/{repository} - Create repository
- [x] 6.2 Implement GET /_snapshot/{repository} - Get repository
- [x] 6.3 Implement GET /_snapshot - List repositories
- [x] 6.4 Implement PUT /_snapshot/{repository}/{snapshot} - Create snapshot
- [x] 6.5 Implement GET /_snapshot/{repository}/{snapshot} - Get snapshot
- [x] 6.6 Implement DELETE /_snapshot/{repository}/{snapshot} - Delete snapshot
- [x] 6.7 Implement GET /_snapshot/{repository}/_all - List snapshots
- [x] 6.8 Implement POST /_snapshot/{repository}/{snapshot}/_restore - Restore snapshot
- [x] 6.9 Implement GET /_snapshot/_stats - Get snapshot stats
- [x] 6.10 Add tests for all snapshot endpoints

## 7. Cluster Endpoints (Phase 2 - Distributed System)
- [x] 7.1 Implement GET / - Cluster info
- [x] 7.2 Implement GET /_cluster/health
- [x] 7.3 Implement GET /_cluster/stats
- [x] 7.4 Add cluster state endpoint
- [x] 7.5 Add tests

## 8. Middleware
- [x] 8.1 Implement request logging middleware
- [x] 8.2 Implement authentication middleware (API key)
- [x] 8.3 Implement rate limiting middleware
- [x] 8.4 Add CORS middleware
- [x] 8.5 Add request timeout middleware
- [x] 8.6 Test middleware chain
- [x] 8.7 Fix unsafe code in auth tests

## 9. Error Handling
- [x] 9.1 Define error response format
- [x] 9.2 Implement error mapping from core
- [x] 9.3 Add proper HTTP status codes
- [x] 9.4 Implement error logging
- [x] 9.5 Add error response tests
- [x] 9.6 Add ToSchema for error types
- [x] 9.7 Add ValidationError type

## 10. Documentation & Testing
- [x] 10.1 Generate OpenAPI specification
- [x] 10.2 Add API documentation
- [x] 10.3 Create integration test suite (35+ tests)
- [x] 10.4 Add load tests (http_load_test.rs + load_test.rs)
- [x] 10.5 Document all endpoints
- [x] 10.6 Create usage examples
- [x] 10.7 Setup utoipa with SwaggerUI
- [x] 10.8 Resolve utoipa version conflicts (5.4 + swagger-ui 8.0)
- [x] 10.9 Add utoipa::path annotations (34 endpoints documented)

## 11. Quality Checks
- [x] 11.1 Run cargo fmt and clippy
- [x] 11.2 Achieve >95% test coverage
- [x] 11.3 Run performance benchmarks
- [x] 11.4 Verify all acceptance criteria
- [x] 11.5 Update CHANGELOG
- [x] 11.6 Fix all compiler errors
- [x] 11.7 Remove unused imports and variables
- [x] 11.8 Configure unsafe_code linting for tests

## 12. Endpoint Count
- [x] 12.1 Health: 1 endpoint
- [x] 12.2 Index: 7 endpoints
- [x] 12.3 Document: 5 endpoints
- [x] 12.4 Search: 1 endpoint
- [x] 12.5 Snapshot: 10 endpoints
- [x] 12.6 Template: 4 endpoints
- [x] 12.7 Admin: 6 endpoints
- [x] 12.8 Total: 34 documented endpoints

