## Why

Lexum Server currently has 35 out of 71 API routes passing (49.3% pass rate), which severely limits its usability and reliability. Critical issues include JSON parsing failures that block index creation, preventing users from performing basic operations. Additionally, 33 routes are failing with various errors (400, 422, 500), and several endpoints have incorrect error handling. To serve as a production-ready search engine, Lexum must achieve 100% route pass rate with proper error handling, validation, and comprehensive test coverage. This fix is critical for user adoption and system reliability.

## What Changes

- Fix JSON deserialization issues in Create Index endpoint and other POST endpoints (Bulk, Search, Template, Rollover, Suggestions)
- Fix Delete Index endpoint to return proper 404 errors instead of 500 Internal Server Error
- Fix Check Bounds geo endpoint validation and request format
- Fix Search POST endpoint JSON format and validation
- Fix Add Alias endpoint validation
- Fix Rollover endpoint JSON format
- Improve error handling across all endpoints with detailed error messages
- Add request validation middleware for Content-Type headers and JSON validation
- Enhance test script with retry logic, dependency management, and detailed error logging
- Create comprehensive integration test suite for all 71 routes
- Add CI/CD pipeline tests for route validation
- **BREAKING**: Some error response formats may change to be more consistent

## Impact

- Affected specs: `api-routes`, `error-handling`, `validation`, `testing`
- Affected code: Extensive changes across:
  - `lexum-server/src/router.rs` - Route configuration
  - `lexum-server/src/handlers/index.rs` - Index operations handlers
  - `lexum-server/src/handlers/document.rs` - Document operations handlers
  - `lexum-server/src/handlers/search.rs` - Search operations handlers
  - `lexum-server/src/handlers/geo.rs` - Geo operations handlers
  - `lexum-server/src/handlers/template.rs` - Template handlers
  - `lexum-server/src/error.rs` - Error handling improvements
  - `lexum-server/src/middleware/` - New validation middleware (if needed)
  - `scripts/test_all_routes.ps1` - Enhanced test script
  - `lexum-server/tests/integration/` - New integration tests
- Dependencies: No new external dependencies required
- Performance target: No performance degradation, maintain current response times
- Breaking change: Some error response formats may change for consistency
- User benefit: 100% route reliability, proper error messages, better debugging experience
- Estimated duration: 8-16 hours
- Test coverage target: >95% for all handlers

