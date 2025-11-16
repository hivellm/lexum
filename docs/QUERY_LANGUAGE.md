# LQL (Lexum Query Language)

LQL is a powerful, SQL-inspired query language designed specifically for full-text search and analytics in Lexum. It combines the familiarity of SQL with search-specific operations.

## Overview

LQL provides a declarative way to express complex search queries, filters, aggregations, and transformations in a readable, composable syntax.

**Design Principles:**
- **Familiar Syntax**: SQL-like for ease of adoption
- **Search-Optimized**: Built for full-text search operations
- **Composable**: Chain operations with pipes
- **Type-Safe**: Strong typing with validation
- **Performant**: Optimizable query plans

## Basic Syntax

```sql
FROM <index_name>
[| <operation>]*
```

Operations are chained using the pipe operator `|`, similar to shell pipelines.

## Operations

### FROM

Specifies the source index(es).

```sql
-- Single index
FROM users

-- Multiple indices
FROM users, accounts

-- Index pattern
FROM logs-*

-- Aliasing
FROM users AS u
```

### WHERE

Filters documents using boolean predicates.

```sql
-- Basic comparison
FROM users | WHERE age > 18

-- Logical operators
FROM users | WHERE age > 18 AND status = "active"

-- IN operator
FROM users | WHERE country IN ["US", "CA", "UK"]

-- Range
FROM events | WHERE timestamp BETWEEN "2024-01-01" AND "2024-12-31"

-- Null checks
FROM users | WHERE email IS NOT NULL

-- Nested fields
FROM users | WHERE address.city = "New York"
```

**Supported Operators:**
- Comparison: `=`, `!=`, `>`, `>=`, `<`, `<=`
- Logical: `AND`, `OR`, `NOT`
- Membership: `IN`, `NOT IN`
- Null: `IS NULL`, `IS NOT NULL`
- Range: `BETWEEN ... AND ...`

### MATCH

Full-text search operation.

```sql
-- Simple match
FROM documents | MATCH "search query"

-- Match in specific field
FROM documents | MATCH "search query" IN title

-- Match in multiple fields
FROM documents | MATCH "search query" IN (title, content)

-- Fuzzy matching
FROM documents | MATCH "searhc~" IN content

-- Phrase matching
FROM documents | MATCH "\"exact phrase\"" IN content

-- Boosted fields
FROM documents | MATCH "query" IN (title^3, content^1)
```

**Match Options:**
```sql
-- With options
FROM documents | MATCH "query" IN content OPTIONS {
  analyzer: "english",
  fuzziness: 2,
  operator: "AND"
}
```

### SORT

Orders results.

```sql
-- Single field ascending
FROM users | SORT created_at

-- Explicit direction
FROM users | SORT created_at DESC

-- Multiple fields
FROM users | SORT score DESC, created_at ASC

-- By relevance score
FROM documents | MATCH "query" | SORT _score DESC
```

**Special Fields:**
- `_score`: Relevance score (search only)
- `_id`: Document ID
- `_timestamp`: Index timestamp

### LIMIT

Limits the number of results.

```sql
-- Basic limit
FROM users | LIMIT 100

-- With offset (pagination)
FROM users | LIMIT 100 OFFSET 200

-- Alternative syntax
FROM users | LIMIT 100, 200  -- offset, limit
```

### SELECT

Projects specific fields.

```sql
-- Select specific fields
FROM users | SELECT name, email

-- Select all fields (default)
FROM users | SELECT *

-- Exclude fields
FROM users | SELECT * EXCEPT (password, ssn)

-- Rename fields
FROM users | SELECT name AS full_name, email

-- Computed fields
FROM users | SELECT name, age * 2 AS double_age
```

### AGGREGATE

Performs aggregations.

```sql
-- Count
FROM users | AGGREGATE COUNT() AS total

-- Group by
FROM users | AGGREGATE COUNT() AS total BY country

-- Multiple aggregations
FROM sales 
| AGGREGATE 
    COUNT() AS total_orders,
    SUM(amount) AS total_revenue,
    AVG(amount) AS avg_order_value
  BY product_category

-- Statistical aggregations
FROM metrics
| AGGREGATE
    MIN(value) AS min_value,
    MAX(value) AS max_value,
    AVG(value) AS avg_value,
    PERCENTILE(value, 95) AS p95
  BY service_name
```

**Supported Aggregations:**
- `COUNT()`: Count documents
- `SUM(field)`: Sum numeric field
- `AVG(field)`: Average of field
- `MIN(field)`: Minimum value
- `MAX(field)`: Maximum value
- `PERCENTILE(field, percentile)`: Percentile calculation
- `CARDINALITY(field)`: Unique value count
- `STATS(field)`: All statistics

### HISTOGRAM

Creates histograms for numeric or date fields.

```sql
-- Numeric histogram
FROM metrics
| HISTOGRAM value BY 10 AS value_hist
| AGGREGATE COUNT() BY value_hist

-- Date histogram
FROM logs
| HISTOGRAM timestamp BY "1h" AS time_bucket
| AGGREGATE COUNT() BY time_bucket

-- With aggregations
FROM sales
| HISTOGRAM price BY 100 AS price_range
| AGGREGATE SUM(quantity) AS total_sold BY price_range
```

**Date Intervals:**
- `s`, `m`, `h`, `d`, `w`, `M`, `y` (seconds, minutes, hours, days, weeks, months, years)
- Examples: `"30s"`, `"5m"`, `"1h"`, `"1d"`

### TERMS

Groups by top terms.

```sql
-- Top terms
FROM logs | TERMS status_code SIZE 10

-- With aggregations
FROM logs 
| TERMS status_code SIZE 10
| AGGREGATE COUNT() AS request_count

-- Nested terms
FROM logs
| TERMS status_code
| TERMS user_agent
| AGGREGATE COUNT()
```

### JOIN

Joins data from multiple indices (limited support).

```sql
-- Inner join
FROM orders
| JOIN customers ON orders.customer_id = customers.id

-- Left join
FROM orders
| LEFT JOIN customers ON orders.customer_id = customers.id

-- With alias
FROM orders AS o
| JOIN customers AS c ON o.customer_id = c.id
```

**Note:** Joins are expensive operations and should be used sparingly.

## Advanced Features

### Nested Queries

```sql
-- Subquery in WHERE
FROM users
| WHERE user_id IN (
    FROM orders 
    | WHERE total > 1000 
    | SELECT customer_id
  )
```

### Window Functions

```sql
-- Ranking
FROM users
| SELECT 
    name,
    score,
    RANK() OVER (ORDER BY score DESC) AS rank

-- Running totals
FROM sales
| SELECT
    date,
    amount,
    SUM(amount) OVER (ORDER BY date) AS running_total
```

### Array Operations

```sql
-- Array contains
FROM documents | WHERE tags CONTAINS "rust"

-- Array length
FROM documents | WHERE ARRAY_LENGTH(tags) > 5

-- Unnest array
FROM documents | UNNEST tags AS tag
```

### String Functions

```sql
-- String operations
FROM users
| SELECT 
    LOWER(name) AS lowercase_name,
    UPPER(email) AS uppercase_email,
    CONCAT(first_name, " ", last_name) AS full_name,
    SUBSTRING(phone, 1, 3) AS area_code
```

### Date Functions

```sql
-- Date manipulation
FROM events
| SELECT
    DATE_FORMAT(timestamp, "%Y-%m-%d") AS date,
    DATE_TRUNC(timestamp, "hour") AS hour,
    DATE_DIFF(timestamp, created_at, "days") AS days_since_creation
```

### Math Functions

```sql
-- Mathematical operations
FROM metrics
| SELECT
    ROUND(value, 2) AS rounded,
    CEIL(value) AS ceiling,
    FLOOR(value) AS floored,
    ABS(difference) AS absolute_diff,
    POW(value, 2) AS squared
```

## Query Examples

### Example 1: Basic Search

Find active users in the US created in the last 30 days.

```sql
FROM users
| WHERE 
    status = "active" 
    AND country = "US"
    AND created_at > NOW() - INTERVAL "30 days"
| SORT created_at DESC
| LIMIT 100
```

### Example 2: Full-Text Search with Filters

Search for "machine learning" in documents, filtering by category.

```sql
FROM documents
| MATCH "machine learning" IN (title^3, content)
| WHERE category IN ["ai", "ml", "data-science"]
| WHERE published_date > "2024-01-01"
| SORT _score DESC, published_date DESC
| SELECT title, author, published_date, _score
| LIMIT 50
```

### Example 3: Aggregation Query

Analyze sales by product category and region.

```sql
FROM sales
| WHERE order_date BETWEEN "2024-01-01" AND "2024-12-31"
| AGGREGATE
    COUNT() AS total_orders,
    SUM(amount) AS total_revenue,
    AVG(amount) AS avg_order_value
  BY product_category, region
| SORT total_revenue DESC
```

### Example 4: Time-Series Analysis

Analyze request rates over time.

```sql
FROM logs
| WHERE timestamp > NOW() - INTERVAL "24 hours"
| HISTOGRAM timestamp BY "5m" AS time_bucket
| AGGREGATE
    COUNT() AS request_count,
    AVG(response_time) AS avg_response_time,
    PERCENTILE(response_time, 95) AS p95_response_time
  BY time_bucket
| SORT time_bucket ASC
```

### Example 5: Top Terms Analysis

Find most common error codes and their counts.

```sql
FROM error_logs
| WHERE severity = "error"
| WHERE timestamp > NOW() - INTERVAL "1 day"
| TERMS error_code SIZE 20
| AGGREGATE 
    COUNT() AS error_count,
    CARDINALITY(user_id) AS affected_users
| SORT error_count DESC
```

### Example 6: Complex Search with Nested Aggregations

```sql
FROM products
| MATCH "laptop" IN (name, description)
| WHERE price BETWEEN 500 AND 2000
| WHERE in_stock = true
| AGGREGATE
    COUNT() AS product_count,
    AVG(price) AS avg_price,
    MIN(price) AS min_price,
    MAX(price) AS max_price
  BY brand
| SORT product_count DESC
| LIMIT 10
```

### Example 7: User Behavior Analysis

```sql
FROM user_events
| WHERE event_type = "purchase"
| WHERE timestamp BETWEEN "2024-10-01" AND "2024-10-31"
| AGGREGATE
    COUNT() AS purchase_count,
    SUM(amount) AS total_spent,
    CARDINALITY(product_id) AS unique_products
  BY user_id
| WHERE purchase_count > 5
| SORT total_spent DESC
| LIMIT 100
```

## Query Optimization

### Best Practices

1. **Filter Early**: Apply filters before expensive operations
   ```sql
   -- Good
   FROM logs | WHERE timestamp > "2024-01-01" | MATCH "error"
   
   -- Less optimal
   FROM logs | MATCH "error" | WHERE timestamp > "2024-01-01"
   ```

2. **Limit Results**: Always use LIMIT to prevent excessive results
   ```sql
   FROM users | LIMIT 1000  -- Good practice
   ```

3. **Use Specific Fields**: Select only needed fields
   ```sql
   -- Good
   FROM users | SELECT id, name, email
   
   -- Less optimal
   FROM users | SELECT *
   ```

4. **Index Appropriate Fields**: Ensure filtered/sorted fields are indexed

5. **Avoid Wildcards at Start**: `*query` is slower than `query*`

6. **Use Filters Over Queries**: Filters are cached and faster
   ```sql
   -- Prefer
   WHERE status = "active"
   
   -- Over
   MATCH "active" IN status
   ```

### Query Hints

Provide optimization hints to the query planner.

```sql
FROM users /*+ INDEX(idx_created_at) */
| WHERE created_at > "2024-01-01"

FROM large_index /*+ PARALLEL(4) */
| MATCH "search query"

FROM logs /*+ NO_CACHE */
| WHERE timestamp > NOW() - INTERVAL "1m"
```

## Error Handling

### Syntax Errors

```sql
-- Error: Missing FROM clause
WHERE status = "active"
-- Correct:
FROM users | WHERE status = "active"
```

### Type Errors

```sql
-- Error: Cannot compare string to number
FROM users | WHERE age = "eighteen"
-- Correct:
FROM users | WHERE age = 18
```

### Runtime Errors

```sql
-- Error: Field does not exist
FROM users | WHERE non_existent_field = "value"
-- Check schema first
```

## Performance Considerations

### Query Complexity
- Simple queries: < 10ms
- Complex aggregations: 100ms - 1s
- Multi-index joins: 1s+

### Recommended Limits
- LIMIT: < 10,000 per query
- Aggregation buckets: < 10,000
- JOIN size: < 100,000 documents

### Monitoring

Enable slow query logging:
```yaml
# config.yml
query:
  slow_query_threshold: 1000  # ms
  log_slow_queries: true
```

## CLI Usage

```bash
# Interactive LQL shell
lexum lql

# Execute query from file
lexum lql -f query.lql

# Execute inline query
lexum lql -e "FROM users | LIMIT 10"

# Output format
lexum lql -e "FROM users" --format json
lexum lql -e "FROM users" --format table
lexum lql -e "FROM users" --format csv
```

## HTTP API

```bash
# POST request with LQL
curl -X POST http://localhost:9200/_lql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM users | WHERE age > 18 | LIMIT 10"
  }'

# With parameters
curl -X POST http://localhost:9200/_lql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM users | WHERE age > $min_age | LIMIT $limit",
    "params": {
      "min_age": 18,
      "limit": 10
    }
  }'
```

## Grammar Reference

```ebnf
Query          = FromClause ("|" Operation)*
FromClause     = "FROM" IndexPattern ("AS" Identifier)?
Operation      = WhereClause | MatchClause | SortClause | LimitClause 
               | SelectClause | AggregateClause | HistogramClause
               | TermsClause | JoinClause

WhereClause    = "WHERE" Expression
MatchClause    = "MATCH" String ("IN" FieldList)? ("OPTIONS" Options)?
SortClause     = "SORT" SortField ("," SortField)*
LimitClause    = "LIMIT" Number ("OFFSET" Number)? | "LIMIT" Number "," Number
SelectClause   = "SELECT" SelectList
AggregateClause= "AGGREGATE" AggregateList ("BY" FieldList)?
HistogramClause= "HISTOGRAM" Field "BY" Interval ("AS" Identifier)?
TermsClause    = "TERMS" Field ("SIZE" Number)?
JoinClause     = ("LEFT")? "JOIN" IndexPattern "ON" JoinCondition

Expression     = OrExpr
OrExpr         = AndExpr ("OR" AndExpr)*
AndExpr        = NotExpr ("AND" NotExpr)*
NotExpr        = ("NOT")? CompareExpr
CompareExpr    = Term (CompareOp Term)?
CompareOp      = "=" | "!=" | ">" | ">=" | "<" | "<=" | "IN" | "NOT IN"
               | "IS NULL" | "IS NOT NULL" | "BETWEEN" | "CONTAINS"

Term           = Field | Literal | Function | "(" Expression ")"
Field          = Identifier ("." Identifier)*
Literal        = String | Number | Boolean | Null
Function       = Identifier "(" (Expression ("," Expression)*)? ")"
```

## Type System

### Supported Types

- **Text**: Full-text searchable string
- **Keyword**: Exact-match string (not analyzed)
- **Integer**: 64-bit signed integer
- **Float**: 64-bit floating point
- **Boolean**: true/false
- **Date**: ISO 8601 timestamp
- **Array**: Array of any type
- **Object**: Nested object
- **Geo**: Geospatial coordinates

### Type Casting

```sql
-- Explicit casting
FROM users | WHERE CAST(age AS TEXT) = "25"

-- Implicit casting (when safe)
FROM users | WHERE age = 25  -- number literal
```

## Future Enhancements

- **Vector Search**: Semantic similarity queries
- **Graph Queries**: Relationship traversal
- **Machine Learning**: ML-based scoring
- **Fuzzy Joins**: Approximate matching joins
- **CTEs**: Common Table Expressions
- **Lateral Joins**: Correlated subqueries

## References

- [ElasticSearch Query DSL](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html)
- [SQL Standard](https://www.iso.org/standard/76583.html)
- [Apache Calcite](https://calcite.apache.org/)

