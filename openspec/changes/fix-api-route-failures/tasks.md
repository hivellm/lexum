# API Route Failures - Bug Fix Tasks

**Created**: 2025-11-12  
**Status**: ~82% API Routes Working (32/39 successful)  
**Priority**: Fix critical failures first, then high priority, then medium/low

## Summary

During comprehensive API route testing, 7 routes failed out of 39 tested routes (82.05% success rate). This document tracks the bugs found and their fixes, ordered by criticality.

## Critical Priority (Core Functionality)

### 1. Document Retrieval Failure (404)
**Route**: `GET /api/v1/indices/{index}/documents/{id}`  
**Status Code**: 404 Not Found  
**Impact**: CRITICAL - Core document retrieval functionality broken  
**Category**: Document Operations

**Problem**:
- Document is created successfully (201) but cannot be retrieved immediately after creation
- Document ID is returned correctly but GET request fails with 404
- May be related to index refresh/commit timing

**Root Cause Analysis Needed**:
- [ ] Check if document is properly committed to index
- [ ] Verify index refresh is working correctly
- [ ] Check if document ID field mapping is correct
- [ ] Verify document store get_document implementation
- [ ] Check if there's a race condition between commit and read

**Tasks**:
- [ ] 1.1 Investigate DocumentStore.get_document() implementation
- [ ] 1.2 Check if commit() is being called after add_document()
- [ ] 1.3 Verify index refresh is working after document creation
- [ ] 1.4 Test document retrieval with explicit refresh call
- [ ] 1.5 Add automatic refresh after document operations (if needed)
- [ ] 1.6 Fix document ID field mapping/retrieval logic
- [ ] 1.7 Add integration test for document create -> get flow
- [ ] 1.8 Verify fix with API route test

**Estimated Effort**: Medium  
**Dependencies**: None

---

### 2. Search Query String Failure (500)
**Route**: `GET /api/v1/indices/{index}/search?q=test`  
**Status Code**: 500 Internal Server Error  
**Impact**: CRITICAL - Search functionality broken for query string format  
**Category**: Search

**Problem**:
- POST search with JSON body works correctly
- GET search with query string parameter fails with 500 error
- Query string parsing/conversion may be failing

**Root Cause Analysis Needed**:
- [ ] Check search_get handler implementation
- [ ] Verify query string parsing logic
- [ ] Check error logs for specific exception
- [ ] Verify query conversion from string to Query object
- [ ] Test query string parameter handling

**Tasks**:
- [ ] 2.1 Review search_get handler in handlers/search.rs
- [ ] 2.2 Check query string parsing implementation
- [ ] 2.3 Add error handling for query string conversion
- [ ] 2.4 Verify Query::from_query_string() or equivalent exists
- [ ] 2.5 Add proper error messages for invalid query strings
- [ ] 2.6 Test with various query string formats
- [ ] 2.7 Add integration test for GET search endpoint
- [ ] 2.8 Verify fix with API route test

**Estimated Effort**: Medium  
**Dependencies**: None

---

## High Priority (Important Features)

### 3. Template Creation Failure (500)
**Route**: `PUT /_template/{name}`  
**Status Code**: 500 Internal Server Error  
**Impact**: HIGH - Template functionality completely broken  
**Category**: Templates

**Problem**:
- Template creation fails with 500 error
- This causes all template-related operations to fail (GET, DELETE)
- Template management is an important feature for index management

**Root Cause Analysis Needed**:
- [ ] Check template handler implementation
- [ ] Verify template request validation
- [ ] Check template manager create/update logic
- [ ] Verify template storage/persistence
- [ ] Check error logs for specific exception

**Tasks**:
- [ ] 3.1 Review template PUT handler in handlers/template.rs
- [ ] 3.2 Check TemplateManager implementation
- [ ] 3.3 Verify template request structure validation
- [ ] 3.4 Check template storage/persistence logic
- [ ] 3.5 Add proper error handling and logging
- [ ] 3.6 Test template creation with various configurations
- [ ] 3.7 Add integration test for template operations
- [ ] 3.8 Verify fix with API route test

**Estimated Effort**: Medium-High  
**Dependencies**: None

---

## Medium Priority (Configuration)

### 4. Cluster Settings Update Failure (422)
**Route**: `PUT /_cluster/settings`  
**Status Code**: 422 Unprocessable Entity  
**Impact**: MEDIUM - Cluster configuration update not working  
**Category**: Cluster Management

**Problem**:
- Cluster settings update fails with 422 (validation error)
- Request format may be incorrect or validation too strict
- GET cluster settings works correctly

**Root Cause Analysis Needed**:
- [ ] Check cluster settings PUT handler
- [ ] Verify request body format expected
- [ ] Check validation logic for settings update
- [ ] Compare with Elasticsearch API format
- [ ] Test with different request body formats

**Tasks**:
- [ ] 4.1 Review cluster settings PUT handler
- [ ] 4.2 Check request body structure validation
- [ ] 4.3 Verify settings update logic
- [ ] 4.4 Add better error messages for validation failures
- [ ] 4.5 Test with correct request format
- [ ] 4.6 Update API documentation with correct format
- [ ] 4.7 Add integration test for cluster settings update
- [ ] 4.8 Verify fix with API route test

**Estimated Effort**: Low-Medium  
**Dependencies**: None

---

## Low Priority (Edge Cases / Expected Behavior)

### 5. Template Operations (404) - Depends on #3
**Routes**: 
- `GET /_template/{name}` - Status 404
- `DELETE /_template/{name}` - Status 404

**Status Code**: 404 Not Found  
**Impact**: LOW - Expected behavior if template doesn't exist  
**Category**: Templates

**Problem**:
- These failures are expected if template creation (#3) fails
- Once template creation is fixed, these should work
- May need better error handling for non-existent templates

**Tasks**:
- [ ] 5.1 Fix template creation first (#3)
- [ ] 5.2 Verify GET template works after creation
- [ ] 5.3 Verify DELETE template works after creation
- [ ] 5.4 Add proper 404 error messages for non-existent templates
- [ ] 5.5 Verify fix with API route test

**Estimated Effort**: Low  
**Dependencies**: Task #3 (Template Creation)

---

### 6. Task ID Validation (400) - Expected Behavior
**Route**: `GET /_tasks/{task_id}`  
**Status Code**: 400 Bad Request  
**Impact**: LOW - Expected behavior for invalid task ID  
**Category**: Tasks

**Problem**:
- This is actually expected behavior for invalid/non-existent task IDs
- May need better error message or documentation
- Consider if this should return 404 instead of 400

**Tasks**:
- [ ] 6.1 Review task ID validation logic
- [ ] 6.2 Consider returning 404 instead of 400 for non-existent tasks
- [ ] 6.3 Add better error message for invalid task IDs
- [ ] 6.4 Update API documentation
- [ ] 6.5 Verify behavior is consistent with API design

**Estimated Effort**: Very Low  
**Dependencies**: None

---

## Testing & Verification

### 7. Comprehensive Test Suite
**Impact**: HIGH - Ensure all fixes are properly tested

**Tasks**:
- [ ] 7.1 Update API route test script with fixes
- [ ] 7.2 Add specific integration tests for each fixed route
- [ ] 7.3 Add edge case tests for each endpoint
- [ ] 7.4 Add performance tests for fixed routes
- [ ] 7.5 Verify all 39 routes pass after fixes
- [ ] 7.6 Document any remaining known issues

**Estimated Effort**: Medium  
**Dependencies**: All bug fixes above

---

## Progress Tracking

**Total Tasks**: ~40  
**Completed**: 0  
**In Progress**: 0  
**Pending**: ~40

### By Priority:
- **Critical**: 2 bugs, ~16 tasks
- **High**: 1 bug, ~8 tasks
- **Medium**: 1 bug, ~8 tasks
- **Low**: 2 bugs, ~8 tasks

### By Category:
- Document Operations: 1 bug
- Search: 1 bug
- Templates: 1 bug (affects 3 routes)
- Cluster Management: 1 bug
- Tasks: 1 bug (expected behavior)

---

## Notes

- All fixes should include proper error handling and logging
- Each fix should be verified with the API route test script
- Integration tests should be added for each fixed route
- API documentation should be updated if request/response formats change
- Consider adding request validation middleware to catch format errors earlier

---

## Recent Changes

- 2025-11-12: Initial bug list created from comprehensive API route testing
- 2025-11-12: Identified 7 failing routes out of 39 tested (82.05% success rate)

