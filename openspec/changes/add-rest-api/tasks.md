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
- [ ] 2.4 Implement GET /_cat/indices - List indices
- [ ] 2.5 Add request validation
- [ ] 2.6 Add integration tests

## 3. Document Endpoints
- [x] 3.1 Implement POST /{index}/_doc - Index document (auto ID)
- [x] 3.2 Implement PUT /{index}/_doc/{id} - Index with ID
- [x] 3.3 Implement GET /{index}/_doc/{id} - Get document
- [x] 3.4 Implement POST /{index}/_update/{id} - Update document
- [x] 3.5 Implement DELETE /{index}/_doc/{id} - Delete document
- [ ] 3.6 Add validation and error handling
- [ ] 3.7 Add tests

## 4. Bulk Operations
- [x] 4.1 Implement POST /_bulk endpoint
- [x] 4.2 Support index, create, update, delete operations
- [ ] 4.3 Add NDJSON parsing
- [x] 4.4 Implement batch processing
- [x] 4.5 Add error handling per operation
- [ ] 4.6 Add performance tests

## 5. Search Endpoints
- [x] 5.1 Implement POST /{index}/_search
- [ ] 5.2 Support query DSL parsing
- [x] 5.3 Add pagination parameters
- [ ] 5.4 Add sorting parameters
- [x] 5.5 Add field selection
- [x] 5.6 Implement result formatting
- [ ] 5.7 Add search tests

## 6. Cluster Endpoints
- [ ] 6.1 Implement GET / - Cluster info
- [ ] 6.2 Implement GET /_cluster/health
- [ ] 6.3 Implement GET /_cluster/stats
- [ ] 6.4 Add cluster state endpoint
- [ ] 6.5 Add tests

## 7. Middleware
- [ ] 7.1 Implement request logging middleware
- [ ] 7.2 Implement authentication middleware (API key)
- [ ] 7.3 Implement rate limiting middleware
- [ ] 7.4 Add CORS middleware
- [ ] 7.5 Add request timeout middleware
- [ ] 7.6 Test middleware chain

## 8. Error Handling
- [x] 8.1 Define error response format
- [x] 8.2 Implement error mapping from core
- [x] 8.3 Add proper HTTP status codes
- [ ] 8.4 Implement error logging
- [ ] 8.5 Add error response tests

## 9. Documentation & Testing
- [ ] 9.1 Generate OpenAPI specification
- [ ] 9.2 Add API documentation
- [x] 9.3 Create integration test suite
- [ ] 9.4 Add load tests
- [ ] 9.5 Document all endpoints
- [ ] 9.6 Create usage examples

## 10. Quality Checks
- [ ] 10.1 Run cargo fmt and clippy
- [x] 10.2 Achieve >95% test coverage
- [ ] 10.3 Run performance benchmarks
- [ ] 10.4 Verify all acceptance criteria
- [x] 10.5 Update CHANGELOG

