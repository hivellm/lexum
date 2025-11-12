## ADDED Requirements

### Requirement: HTTP Server
The system SHALL provide an HTTP/1.1 and HTTP/2 REST API server on configurable port (default 9200).

#### Scenario: Server starts successfully
- **WHEN** server is started with valid configuration
- **THEN** server binds to specified port
- **AND** responds to health check requests
- **AND** logs startup message with version info

#### Scenario: Graceful shutdown
- **WHEN** server receives shutdown signal (SIGTERM or SIGINT)
- **THEN** server stops accepting new connections
- **AND** waits for in-flight requests to complete (up to 30s)
- **AND** closes all connections gracefully

### Requirement: Create Index Endpoint
The system SHALL expose PUT /{index} endpoint to create indices.

#### Scenario: Create index successfully
- **WHEN** client sends PUT /my_index with valid settings
- **THEN** server returns 200 OK
- **AND** response includes index creation acknowledgment
- **AND** index is immediately available

#### Scenario: Create index with invalid settings
- **WHEN** client sends PUT /my_index with invalid JSON
- **THEN** server returns 400 Bad Request
- **AND** error message explains validation failure

#### Scenario: Create duplicate index
- **WHEN** client attempts to create index that already exists
- **THEN** server returns 400 Bad Request
- **AND** error indicates index already exists

### Requirement: Get Index Endpoint
The system SHALL expose GET /{index} endpoint to retrieve index information.

#### Scenario: Get existing index
- **WHEN** client sends GET /my_index for existing index
- **THEN** server returns 200 OK
- **AND** response includes index settings and mappings

#### Scenario: Get non-existent index
- **WHEN** client sends GET /my_index for non-existent index
- **THEN** server returns 404 Not Found

### Requirement: Delete Index Endpoint
The system SHALL expose DELETE /{index} endpoint to delete indices.

#### Scenario: Delete existing index
- **WHEN** client sends DELETE /my_index
- **THEN** server returns 200 OK
- **AND** index is permanently deleted

### Requirement: Index Document Endpoint
The system SHALL expose POST /{index}/_doc to index documents with auto-generated IDs.

#### Scenario: Index document successfully
- **WHEN** client sends POST /my_index/_doc with valid JSON document
- **THEN** server returns 201 Created
- **AND** response includes generated document ID
- **AND** document is indexed

#### Scenario: Index invalid JSON
- **WHEN** client sends malformed JSON
- **THEN** server returns 400 Bad Request
- **AND** error message explains JSON parsing error

### Requirement: Index Document with ID Endpoint
The system SHALL expose PUT /{index}/_doc/{id} to index documents with specific IDs.

#### Scenario: Index with custom ID
- **WHEN** client sends PUT /my_index/_doc/doc_123 with document
- **THEN** server returns 200 OK or 201 Created
- **AND** document is stored with specified ID

### Requirement: Get Document Endpoint
The system SHALL expose GET /{index}/_doc/{id} to retrieve documents.

#### Scenario: Get existing document
- **WHEN** client sends GET /my_index/_doc/doc_123
- **THEN** server returns 200 OK
- **AND** response includes complete document

#### Scenario: Get non-existent document
- **WHEN** client requests document that doesn't exist
- **THEN** server returns 404 Not Found

### Requirement: Delete Document Endpoint
The system SHALL expose DELETE /{index}/_doc/{id} to delete documents.

#### Scenario: Delete existing document
- **WHEN** client sends DELETE /my_index/_doc/doc_123
- **THEN** server returns 200 OK
- **AND** document is marked for deletion

### Requirement: Search Endpoint
The system SHALL expose POST /{index}/_search endpoint for searching.

#### Scenario: Simple search query
- **WHEN** client posts search query to /my_index/_search
- **THEN** server returns 200 OK
- **AND** response includes hits array with matching documents
- **AND** includes total count of matches
- **AND** includes search duration in milliseconds

#### Scenario: Search with pagination
- **WHEN** client includes "size" and "from" parameters
- **THEN** server returns requested page of results
- **AND** maintains consistent ordering

### Requirement: Bulk Operations Endpoint
The system SHALL expose POST /_bulk for batch operations.

#### Scenario: Bulk index multiple documents
- **WHEN** client sends NDJSON with multiple index operations
- **THEN** server processes all operations
- **AND** returns success/failure status for each
- **AND** partial failures don't abort entire batch

#### Scenario: Mixed bulk operations
- **WHEN** client sends mix of index, update, delete operations
- **THEN** server processes each according to its type
- **AND** returns individual results

### Requirement: Health Check Endpoint
The system SHALL expose GET /_health endpoint for health monitoring.

#### Scenario: Server is healthy
- **WHEN** client sends GET /_health request
- **THEN** server returns 200 OK
- **AND** response indicates healthy status

#### Scenario: Server is unhealthy
- **WHEN** server cannot access storage or indices
- **THEN** GET /_health returns 503 Service Unavailable

### Requirement: Cluster Info Endpoint
The system SHALL expose GET / endpoint returning cluster information.

#### Scenario: Get cluster info
- **WHEN** client sends GET / request
- **THEN** server returns cluster name, version, tagline

### Requirement: Authentication
The system SHALL support API key authentication via X-API-Key header.

#### Scenario: Authenticated request
- **WHEN** client includes valid API key in X-API-Key header
- **THEN** request is processed normally

#### Scenario: Missing authentication
- **WHEN** client sends request without API key
- **THEN** server returns 401 Unauthorized

#### Scenario: Invalid API key
- **WHEN** client sends request with invalid API key
- **THEN** server returns 401 Unauthorized

### Requirement: Rate Limiting
The system SHALL implement rate limiting per API key.

#### Scenario: Within rate limit
- **WHEN** client makes requests below rate limit
- **THEN** all requests are processed
- **AND** response includes rate limit headers

#### Scenario: Exceeded rate limit
- **WHEN** client exceeds configured rate limit
- **THEN** server returns 429 Too Many Requests
- **AND** includes Retry-After header

### Requirement: Error Responses
The system SHALL return consistent error response format for all errors.

#### Scenario: Error response format
- **WHEN** any error occurs
- **THEN** response includes success=false
- **AND** includes error code and message
- **AND** includes timestamp
- **AND** uses appropriate HTTP status code

### Requirement: Request Logging
The system SHALL log all incoming requests with method, path, status, and duration.

#### Scenario: Successful request logged
- **WHEN** request is processed successfully
- **THEN** log entry includes request details and 200 status

#### Scenario: Failed request logged
- **WHEN** request fails
- **THEN** log entry includes error details and error status

### Requirement: CORS Support
The system SHALL support Cross-Origin Resource Sharing (CORS) configuration.

#### Scenario: CORS preflight request
- **WHEN** client sends OPTIONS request for CORS
- **THEN** server returns appropriate CORS headers

### Requirement: Request Timeout
The system SHALL enforce configurable request timeout (default 30s).

#### Scenario: Request exceeds timeout
- **WHEN** request processing exceeds timeout
- **THEN** server returns 408 Request Timeout
- **AND** cancels ongoing operation

### Requirement: Performance - Request Routing
The system SHALL add less than 10ms overhead for request routing and middleware.

#### Scenario: Routing performance
- **WHEN** measuring request routing time
- **THEN** p95 routing overhead is less than 10ms
- **AND** does not include query execution time

### Requirement: Performance - Throughput
The system SHALL handle at least 1000 requests per second on standard hardware.

#### Scenario: Load test throughput
- **WHEN** server is under sustained load of 1000 req/s
- **THEN** all requests are processed successfully
- **AND** latency remains acceptable

