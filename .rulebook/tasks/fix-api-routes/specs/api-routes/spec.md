# API Routes Specification

## Purpose
This specification defines requirements for fixing all API route issues in Lexum Server to achieve 100% route pass rate with proper error handling, validation, and comprehensive test coverage.

## ADDED Requirements

### Requirement: JSON Request Validation
The system SHALL validate JSON request bodies with detailed error messages including line/column information and field-level error reporting.

#### Scenario: Invalid JSON format rejection
Given a request with malformed JSON body
When the request is processed
Then a 400 Bad Request error is returned
And the error message includes line and column information
And the error message indicates which field caused the error

#### Scenario: Content-Type header validation
Given a POST request without Content-Type: application/json header
When the request is processed
Then a 415 Unsupported Media Type error is returned
And a helpful error message suggests adding the correct Content-Type header

### Requirement: Request Validation Middleware
The system SHALL provide request validation middleware that pre-validates Content-Type headers and JSON format before handlers process requests.

#### Scenario: Content-Type validation
Given a request with invalid Content-Type header
When the middleware processes the request
Then the request is rejected early with appropriate error
And the handler is not invoked

#### Scenario: JSON format validation
Given a request with invalid JSON format
When the middleware processes the request
Then the request is rejected with detailed JSON parsing error
And the handler is not invoked

## MODIFIED Requirements

### Requirement: Create Index Endpoint Error Handling
The system SHALL properly deserialize CreateIndexRequest JSON payloads and return detailed error messages for invalid requests.

#### Scenario: Valid index creation
Given a valid CreateIndexRequest JSON payload with name and settings
When POST /api/v1/indices is called
Then the index is created successfully
And a 201 Created response is returned

#### Scenario: Invalid JSON in create index
Given an invalid JSON payload for CreateIndexRequest
When POST /api/v1/indices is called
Then a 400 Bad Request error is returned
And the error message indicates the JSON parsing issue
And the error message shows which field caused the problem

#### Scenario: Nested settings deserialization
Given a CreateIndexRequest with nested settings object
When POST /api/v1/indices is called
Then the settings are correctly deserialized
And the index is created with the specified settings

### Requirement: Delete Index Endpoint Error Handling
The system SHALL return 404 Not Found for non-existent indices instead of 500 Internal Server Error.

#### Scenario: Delete non-existent index
Given an index name that does not exist
When DELETE /api/v1/indices/{name} is called
Then a 404 Not Found error is returned
And the error message indicates the index was not found
And no 500 Internal Server Error occurs

#### Scenario: Delete index with aliases
Given an index with aliases
When DELETE /api/v1/indices/{name} is called
Then aliases are properly removed before index deletion
And the index is deleted successfully
And a 200 OK response is returned

### Requirement: Search POST Endpoint JSON Format
The system SHALL properly deserialize SearchRequest JSON payloads and validate query structure.

#### Scenario: Valid search request
Given a valid SearchRequest JSON payload with query, size, and from parameters
When POST /api/v1/indices/{index}/search is called
Then the search is executed successfully
And results are returned in the expected format

#### Scenario: Invalid search request format
Given an invalid SearchRequest JSON payload
When POST /api/v1/indices/{index}/search is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates which field is invalid

### Requirement: Geo Check Bounds Endpoint Validation
The system SHALL properly validate Check Bounds request format and return appropriate errors for invalid requests.

#### Scenario: Valid bounds check
Given a valid Check Bounds request with point and bounds
When POST /api/v1/geo/bounds is called
Then the bounds check is executed successfully
And a boolean result is returned indicating if point is within bounds

#### Scenario: Invalid bounds request format
Given an invalid Check Bounds request format
When POST /api/v1/geo/bounds is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates which field is invalid

### Requirement: Bulk Operations JSON Parsing
The system SHALL properly deserialize BulkRequest JSON payloads and handle bulk operations correctly.

#### Scenario: Valid bulk request
Given a valid BulkRequest JSON payload
When POST /api/v1/bulk is called
Then bulk operations are executed successfully
And results are returned for each operation

#### Scenario: Invalid bulk request format
Given an invalid BulkRequest JSON payload
When POST /api/v1/bulk is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates the parsing issue

### Requirement: Template Create JSON Parsing
The system SHALL properly deserialize TemplateRequest JSON payloads and validate template format.

#### Scenario: Valid template creation
Given a valid TemplateRequest JSON payload
When PUT /_template/{name} is called
Then the template is created successfully
And a 200 OK response is returned

#### Scenario: Invalid template request format
Given an invalid TemplateRequest JSON payload
When PUT /_template/{name} is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates which field is invalid

### Requirement: Rollover Endpoint JSON Format
The system SHALL properly deserialize RolloverConfig JSON payloads and validate rollover conditions.

#### Scenario: Valid rollover request
Given a valid RolloverConfig JSON payload with conditions
When POST /api/v1/indices/{alias}/rollover is called
Then the rollover is executed successfully
And a 200 OK response is returned

#### Scenario: Invalid rollover request format
Given an invalid RolloverConfig JSON payload
When POST /api/v1/indices/{alias}/rollover is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates which field is invalid

### Requirement: Add Alias Endpoint Validation
The system SHALL properly validate alias creation requests and return appropriate errors.

#### Scenario: Valid alias creation
Given a valid alias name and existing index
When PUT /{index}/_alias/{alias} is called
Then the alias is created successfully
And a 200 OK response is returned

#### Scenario: Alias creation with non-existent index
Given a non-existent index name
When PUT /{index}/_alias/{alias} is called
Then a 404 Not Found error is returned
And the error message indicates the index was not found

### Requirement: Suggest POST Endpoint JSON Format
The system SHALL properly deserialize SuggestRequest JSON payloads and validate suggest format.

#### Scenario: Valid suggest request
Given a valid SuggestRequest JSON payload
When POST /api/v1/indices/{index}/_suggest is called
Then suggestions are returned successfully
And results match the expected format

#### Scenario: Invalid suggest request format
Given an invalid SuggestRequest JSON payload
When POST /api/v1/indices/{index}/_suggest is called
Then a 422 Unprocessable Entity error is returned
And the error message indicates which field is invalid

### Requirement: Comprehensive Route Testing
The system SHALL have comprehensive integration tests for all 71 API routes with >95% test coverage.

#### Scenario: All routes pass tests
Given the test script is executed
When all 71 routes are tested
Then all routes return expected responses
And no routes return unexpected errors

#### Scenario: Test script with retry logic
Given rate-limited requests during testing
When the test script encounters 429 Too Many Requests
Then requests are retried with exponential backoff
And tests complete successfully

#### Scenario: Test dependency management
Given tests that require indices to exist
When the test script is executed
Then indices are created before dependent tests
And test data is cleaned up after tests complete

