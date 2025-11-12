## ADDED Requirements

### Requirement: FROM Clause
The system SHALL support FROM clause to specify source index(es).

#### Scenario: Single index
- **WHEN** query contains "FROM users"
- **THEN** query targets "users" index

#### Scenario: Multiple indices
- **WHEN** query contains "FROM users, accounts"
- **THEN** query targets both indices

#### Scenario: Index pattern
- **WHEN** query contains "FROM logs-*"
- **THEN** query matches all indices starting with "logs-"

### Requirement: WHERE Clause
The system SHALL support WHERE clause with boolean expressions for filtering.

#### Scenario: Simple comparison
- **WHEN** query contains "WHERE age > 18"
- **THEN** only documents with age greater than 18 are returned

#### Scenario: AND operator
- **WHEN** query contains "WHERE age > 18 AND status = \"active\""
- **THEN** only documents matching both conditions are returned

#### Scenario: OR operator
- **WHEN** query contains "WHERE country = \"US\" OR country = \"CA\""
- **THEN** documents matching either condition are returned

#### Scenario: IN operator
- **WHEN** query contains "WHERE country IN [\"US\", \"CA\", \"UK\"]"
- **THEN** documents with country in the list are returned

### Requirement: MATCH Clause
The system SHALL support MATCH clause for full-text search.

#### Scenario: Simple match
- **WHEN** query contains "MATCH \"search terms\""
- **THEN** full-text search is performed on default fields

#### Scenario: Field-specific match
- **WHEN** query contains "MATCH \"search terms\" IN title"
- **THEN** full-text search is performed only on title field

#### Scenario: Multi-field match
- **WHEN** query contains "MATCH \"query\" IN (title, content)"
- **THEN** both fields are searched

#### Scenario: Boosted fields
- **WHEN** query contains "MATCH \"query\" IN (title^3, content)"
- **THEN** title field has 3x boost in scoring

### Requirement: SORT Clause
The system SHALL support SORT clause for result ordering.

#### Scenario: Single field ascending
- **WHEN** query contains "SORT created_at"
- **THEN** results are ordered by created_at ascending

#### Scenario: Explicit direction
- **WHEN** query contains "SORT created_at DESC"
- **THEN** results are ordered descending

#### Scenario: Multiple fields
- **WHEN** query contains "SORT score DESC, created_at ASC"
- **THEN** results are sorted by score first, then created_at

### Requirement: LIMIT Clause
The system SHALL support LIMIT clause for pagination.

#### Scenario: Simple limit
- **WHEN** query contains "LIMIT 10"
- **THEN** only first 10 results are returned

#### Scenario: Limit with offset
- **WHEN** query contains "LIMIT 10 OFFSET 20"
- **THEN** results 21-30 are returned

### Requirement: SELECT Clause
The system SHALL support SELECT clause for field projection.

#### Scenario: Specific fields
- **WHEN** query contains "SELECT name, email"
- **THEN** only name and email fields are returned

#### Scenario: All fields
- **WHEN** query contains "SELECT *"
- **THEN** all fields are returned

#### Scenario: Field exclusion
- **WHEN** query contains "SELECT * EXCEPT (password, ssn)"
- **THEN** all fields except password and ssn are returned

### Requirement: AGGREGATE Clause
The system SHALL support AGGREGATE clause for aggregations.

#### Scenario: Count aggregation
- **WHEN** query contains "AGGREGATE COUNT() AS total"
- **THEN** total count of documents is returned

#### Scenario: Group by
- **WHEN** query contains "AGGREGATE COUNT() AS total BY country"
- **THEN** count is grouped by country

#### Scenario: Multiple aggregations
- **WHEN** query contains "AGGREGATE COUNT() AS total, AVG(age) AS avg_age BY country"
- **THEN** both aggregations are computed per country

### Requirement: HISTOGRAM Clause
The system SHALL support HISTOGRAM clause for bucketing.

#### Scenario: Numeric histogram
- **WHEN** query contains "HISTOGRAM price BY 100"
- **THEN** prices are bucketed into 100-unit ranges

#### Scenario: Date histogram
- **WHEN** query contains "HISTOGRAM timestamp BY \"1h\""
- **THEN** timestamps are bucketed by hour

### Requirement: TERMS Clause
The system SHALL support TERMS clause for top terms aggregation.

#### Scenario: Top terms
- **WHEN** query contains "TERMS status SIZE 10"
- **THEN** top 10 status values are returned with counts

### Requirement: Pipe Operator
The system SHALL support pipe (|) operator for chaining operations.

#### Scenario: Chained operations
- **WHEN** query contains "FROM users | WHERE age > 18 | SORT name | LIMIT 10"
- **THEN** operations are applied in sequence

### Requirement: String Functions
The system SHALL support string manipulation functions.

#### Scenario: LOWER function
- **WHEN** query contains "SELECT LOWER(name) AS lowercase_name"
- **THEN** name field is converted to lowercase

#### Scenario: CONCAT function
- **WHEN** query contains "SELECT CONCAT(first_name, \" \", last_name) AS full_name"
- **THEN** fields are concatenated

### Requirement: Date Functions
The system SHALL support date manipulation functions.

#### Scenario: DATE_TRUNC function
- **WHEN** query contains "SELECT DATE_TRUNC(timestamp, \"hour\")"
- **THEN** timestamp is truncated to hour

### Requirement: Math Functions
The system SHALL support mathematical functions.

#### Scenario: ROUND function
- **WHEN** query contains "SELECT ROUND(price, 2)"
- **THEN** price is rounded to 2 decimal places

### Requirement: Type System
The system SHALL enforce strong typing with automatic type inference.

#### Scenario: Type checking
- **WHEN** query attempts to compare incompatible types
- **THEN** type error is reported before execution

#### Scenario: Type coercion
- **WHEN** query compares numeric string to number
- **THEN** automatic coercion is applied if safe

### Requirement: Error Messages
The system SHALL provide clear, actionable error messages.

#### Scenario: Syntax error
- **WHEN** query has syntax error
- **THEN** error message includes position and expected tokens

#### Scenario: Semantic error
- **WHEN** query references non-existent field
- **THEN** error message suggests valid fields

### Requirement: Query Optimization
The system SHALL optimize queries before execution.

#### Scenario: Filter pushdown
- **WHEN** query has WHERE clause after MATCH
- **THEN** filter is pushed down to search level

#### Scenario: Constant folding
- **WHEN** query contains constant expressions
- **THEN** expressions are evaluated at compile time

### Requirement: Performance - Parsing
The system SHALL parse and plan queries in less than 10ms.

#### Scenario: Simple query parsing
- **WHEN** parsing simple query
- **THEN** parsing completes in less than 5ms

#### Scenario: Complex query parsing
- **WHEN** parsing complex query with multiple operations
- **THEN** parsing completes in less than 10ms

### Requirement: LQL API Endpoint
The system SHALL expose POST /_lql endpoint for executing LQL queries.

#### Scenario: Execute LQL query
- **WHEN** client posts LQL query to /_lql
- **THEN** query is parsed, executed, and results returned

#### Scenario: Query parameters
- **WHEN** LQL query contains parameters like $var
- **THEN** parameters are substituted from request body

### Requirement: Streaming Results
The system SHALL support streaming large result sets via LQL.

#### Scenario: Stream large results
- **WHEN** LQL query returns >10K results
- **THEN** results are streamed incrementally
- **AND** client can start processing before query completes

