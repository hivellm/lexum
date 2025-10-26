# Lexum CLI - Advanced Usage Examples

This document provides advanced examples and use cases for the Lexum CLI tool.

## Table of Contents

1. [Advanced Search Patterns](#advanced-search-patterns)
2. [Complex LQL Queries](#complex-lql-queries)
3. [Batch Operations](#batch-operations)
4. [Performance Optimization](#performance-optimization)
5. [Integration Examples](#integration-examples)
6. [Troubleshooting Scenarios](#troubleshooting-scenarios)

## Advanced Search Patterns

### Boolean Queries

```bash
# Must match (all required)
lexum search products "+category:electronics +brand:apple"

# Should match (optional, improves score)
lexum search products "category:electronics brand:apple"

# Must not match (exclude)
lexum search products "category:electronics -status:discontinued"

# Complex boolean combinations
lexum search products "+category:electronics +price:[100,1000] -status:discontinued brand:apple"
```

### Range Queries

```bash
# Numeric ranges
lexum search products "price:[100,500]"
lexum search products "rating:[4.0,5.0]"

# Date ranges
lexum search products "created_at:[2024-01-01,2024-12-31]"

# Open-ended ranges
lexum search products "price:[100,*]"  # >= 100
lexum search products "price:[*,500]"  # <= 500
```

### Fuzzy Search

```bash
# Fuzzy matching for typos
lexum search products "title:~laptp"  # matches "laptop"
lexum search products "description:~wireles"  # matches "wireless"

# Fuzzy with custom fuzziness
lexum search products "title:~gaming" --fuzziness 2
```

### Phrase Search

```bash
# Exact phrase matching
lexum search products "description:\"wireless headphones\""
lexum search products "title:\"gaming laptop\""

# Phrase with slop (word distance)
lexum search products "description:\"wireless gaming headphones\" --slop 2"
```

### Wildcard and Regex

```bash
# Wildcard patterns
lexum search products "title:*gaming*"
lexum search products "sku:PROD-*"

# Regex patterns (if supported)
lexum search products "email:/.*@company\.com$/"
```

## Complex LQL Queries

### Multi-table Joins (Conceptual)

```sql
-- Find products with reviews
FROM products p 
WHERE EXISTS (
  SELECT 1 FROM reviews r 
  WHERE r.product_id = p.id 
  AND r.rating >= 4.0
)
```

### Aggregation Queries

```sql
-- Count products by category
FROM products 
GROUP BY category 
SELECT category, COUNT(*) as count

-- Average price by brand
FROM products 
GROUP BY brand 
SELECT brand, AVG(price) as avg_price
```

### Nested Conditions

```sql
-- Complex nested conditions
FROM products 
WHERE (
  (category:electronics AND price:[100,500]) 
  OR 
  (category:books AND price:[10,50])
) 
AND in_stock:true
```

### Time-based Queries

```sql
-- Recent products
FROM products 
WHERE created_at:[2024-01-01,*] 
ORDER BY created_at DESC

-- Products updated in last 30 days
FROM products 
WHERE updated_at:[2024-01-01,*] 
AND last_modified:[2024-01-01,*]
```

## Batch Operations

### Bulk Document Operations

Create a bulk operations file (`bulk_operations.json`):

```json
[
  {
    "action": "index",
    "index": "products",
    "id": "prod_1",
    "document": {
      "title": "Gaming Laptop",
      "category": "electronics",
      "price": 1299.99,
      "in_stock": true
    }
  },
  {
    "action": "update",
    "index": "products", 
    "id": "prod_2",
    "document": {
      "price": 999.99,
      "in_stock": false
    }
  },
  {
    "action": "delete",
    "index": "products",
    "id": "prod_3"
  }
]
```

Execute bulk operations:

```bash
lexum doc bulk products --file bulk_operations.json
```

### Batch Search Operations

Create multiple query files:

**`electronics_query.json`**:
```json
{
  "match": {
    "field": "category",
    "query": "electronics"
  }
}
```

**`price_range_query.json`**:
```json
{
  "range": {
    "field": "price",
    "gte": 100,
    "lte": 500
  }
}
```

**`in_stock_query.json`**:
```json
{
  "term": {
    "field": "in_stock",
    "value": "true"
  }
}
```

Execute batch searches:

```bash
# Search with multiple queries
lexum search products @electronics_query.json @price_range_query.json @in_stock_query.json

# Or execute sequentially
for query in electronics_query.json price_range_query.json in_stock_query.json; do
  echo "Executing $query..."
  lexum search products @$query --limit 10
done
```

### Automated Data Pipeline

Create a script for automated data processing:

```bash
#!/bin/bash
# data_pipeline.sh

# Configuration
INDEX_NAME="products"
DATA_DIR="./data"
QUERY_DIR="./queries"

# Create index if it doesn't exist
if ! lexum index list | grep -q "$INDEX_NAME"; then
  echo "Creating index: $INDEX_NAME"
  lexum index create "$INDEX_NAME" --schema product_schema.yml
fi

# Process data files
for data_file in "$DATA_DIR"/*.json; do
  echo "Processing: $data_file"
  lexum doc bulk "$INDEX_NAME" --file "$data_file"
done

# Execute search queries
for query_file in "$QUERY_DIR"/*.json; do
  echo "Executing query: $query_file"
  lexum search "$INDEX_NAME" "@$query_file" --limit 50
done

# Generate reports
echo "Generating reports..."
lexum search "$INDEX_NAME" "*" --limit 1000 --format json > reports/all_products.json
lexum search "$INDEX_NAME" "category:electronics" --limit 1000 --format json > reports/electronics.json
```

## Performance Optimization

### Query Optimization

```bash
# Use specific fields to reduce response size
lexum search products "gaming" --fields "title,price,category"

# Use pagination for large result sets
lexum search products "*" --limit 100 --offset 0
lexum search products "*" --limit 100 --offset 100
lexum search products "*" --limit 100 --offset 200

# Use sorting for consistent results
lexum search products "electronics" --sort "price:asc"

# Use minimum score to filter low-relevance results
lexum search products "gaming" --min-score 0.5
```

### Index Optimization

```bash
# Check index statistics
lexum index stats products

# Refresh index after bulk operations
lexum index refresh products

# Flush index to disk
lexum index flush products
```

### Caching Strategies

```bash
# Use file-based queries for repeated searches
lexum search products "@common_queries.json"

# Cache frequently used queries
echo '{"match": {"field": "category", "query": "electronics"}}' > cached_queries/electronics.json
lexum search products "@cached_queries/electronics.json"
```

## Integration Examples

### Shell Script Integration

```bash
#!/bin/bash
# search_products.sh

# Function to search products
search_products() {
  local query="$1"
  local limit="${2:-10}"
  
  lexum search products "$query" --limit "$limit" --format json | \
    jq -r '.hits[] | "\(.id): \(.source.title) - $\(.source.price)"'
}

# Function to get product count
get_product_count() {
  local category="$1"
  lexum search products "category:$category" --limit 0 --format json | \
    jq -r '.total_hits'
}

# Usage examples
echo "Gaming products:"
search_products "category:gaming" 5

echo "Total electronics: $(get_product_count electronics)"
```

### Python Integration

```python
#!/usr/bin/env python3
import subprocess
import json
import sys

def search_products(query, limit=10):
    """Search products using Lexum CLI"""
    cmd = [
        "lexum", "search", "products", query,
        "--limit", str(limit),
        "--format", "json"
    ]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error: {e}", file=sys.stderr)
        return None

def get_product_stats():
    """Get product statistics"""
    cmd = ["lexum", "index", "stats", "products", "--format", "json"]
    
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return json.loads(result.stdout)
    except subprocess.CalledProcessError as e:
        print(f"Error: {e}", file=sys.stderr)
        return None

# Example usage
if __name__ == "__main__":
    # Search for gaming products
    results = search_products("category:gaming", 5)
    if results:
        print(f"Found {results['total_hits']} gaming products")
        for hit in results['hits']:
            print(f"- {hit['source']['title']}: ${hit['source']['price']}")
    
    # Get statistics
    stats = get_product_stats()
    if stats:
        print(f"Total documents: {stats['document_count']}")
        print(f"Index size: {stats['size_bytes']} bytes")
```

### Node.js Integration

```javascript
#!/usr/bin/env node
const { execSync } = require('child_process');
const fs = require('fs');

class LexumClient {
  constructor(serverUrl = 'http://localhost:9200') {
    this.serverUrl = serverUrl;
  }

  search(index, query, options = {}) {
    const cmd = [
      'lexum',
      '--url', this.serverUrl,
      'search', index, query,
      '--format', 'json',
      '--limit', options.limit || 10
    ];

    if (options.sort) {
      cmd.push('--sort', options.sort);
    }

    if (options.fields) {
      cmd.push('--fields', options.fields.join(','));
    }

    try {
      const result = execSync(cmd.join(' '), { encoding: 'utf8' });
      return JSON.parse(result);
    } catch (error) {
      console.error('Search error:', error.message);
      return null;
    }
  }

  getIndexStats(index) {
    const cmd = [
      'lexum',
      '--url', this.serverUrl,
      'index', 'stats', index,
      '--format', 'json'
    ];

    try {
      const result = execSync(cmd.join(' '), { encoding: 'utf8' });
      return JSON.parse(result);
    } catch (error) {
      console.error('Stats error:', error.message);
      return null;
    }
  }
}

// Example usage
const client = new LexumClient();

// Search for products
const results = client.search('products', 'category:electronics', {
  limit: 5,
  sort: 'price:asc',
  fields: ['title', 'price', 'category']
});

if (results) {
  console.log(`Found ${results.total_hits} products`);
  results.hits.forEach(hit => {
    console.log(`- ${hit.source.title}: $${hit.source.price}`);
  });
}

// Get statistics
const stats = client.getIndexStats('products');
if (stats) {
  console.log(`Total documents: ${stats.document_count}`);
}
```

## Troubleshooting Scenarios

### Performance Issues

```bash
# Check server status
lexum server status

# Check index statistics
lexum index stats products

# Test with simple query
lexum search products "*" --limit 1

# Check query performance
lexum search products "gaming" --explain
```

### Data Consistency Issues

```bash
# Refresh index after bulk operations
lexum index refresh products

# Check for missing documents
lexum search products "id:missing_doc_id"

# Verify document exists
lexum doc get products "existing_doc_id"
```

### Query Syntax Issues

```bash
# Test simple query first
lexum search products "*"

# Test field-specific query
lexum search products "title:test"

# Test complex query step by step
lexum search products "category:electronics"
lexum search products "category:electronics AND price:[100,500]"
```

### Connection Issues

```bash
# Test server connectivity
curl http://localhost:9200/health

# Check server logs
lexum server status --verbose

# Test with different URL
lexum --url http://127.0.0.1:9200 search products "*"
```

### Memory and Resource Issues

```bash
# Check system resources
lexum server status

# Monitor during operations
lexum search products "*" --limit 1000 --explain

# Use pagination for large results
lexum search products "*" --limit 100 --offset 0
lexum search products "*" --limit 100 --offset 100
```

## Best Practices

### Query Design

1. **Start simple**: Begin with basic queries and add complexity gradually
2. **Use specific fields**: Target specific fields rather than searching all text
3. **Leverage boolean operators**: Use `+`, `-`, `AND`, `OR` for precise control
4. **Test with explain**: Use `--explain` to understand query execution
5. **Use appropriate limits**: Set reasonable limits to avoid overwhelming results

### Data Management

1. **Use bulk operations**: Process multiple documents efficiently
2. **Validate data**: Check JSON validity before adding documents
3. **Use appropriate schemas**: Design schemas for your use case
4. **Regular maintenance**: Refresh and flush indices as needed
5. **Monitor statistics**: Keep track of index size and document counts

### Performance

1. **Use field selection**: Return only needed fields
2. **Implement pagination**: Use offset/limit for large result sets
3. **Cache common queries**: Save frequently used queries as files
4. **Optimize sorting**: Use appropriate sort fields
5. **Monitor performance**: Use explain mode to understand query costs

### Integration

1. **Use file-based queries**: Store complex queries in files
2. **Implement error handling**: Check return codes and handle errors
3. **Use appropriate formats**: Choose JSON for programmatic use
4. **Batch operations**: Group related operations together
5. **Monitor logs**: Check server logs for issues