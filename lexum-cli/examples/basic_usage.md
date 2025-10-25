# Lexum CLI Basic Usage Examples

This document provides practical examples of using the Lexum CLI for common search engine operations.

## Prerequisites

1. Start the Lexum server:
   ```bash
   lexum-server
   ```

2. Open a new terminal for CLI operations.

## Basic Operations

### 1. Create an Index

```bash
# Create a simple index with default settings
lexum index create products

# Create an index with a custom schema
lexum index create products --schema product_schema.yml
```

**Example schema file (`product_schema.yml`):**
```yaml
mappings:
  properties:
    title:
      type: text
      analyzer: standard
    description:
      type: text
      analyzer: standard
    category:
      type: keyword
    price:
      type: float
    in_stock:
      type: boolean
    created_at:
      type: date
```

### 2. Add Documents

```bash
# Add a single document from JSON file
lexum doc add products --file product.json

# Add document with specific ID
lexum doc add products --file product.json --id "prod_123"

# Bulk add multiple documents
lexum doc bulk products --file products.json
```

**Example document (`product.json`):**
```json
{
  "title": "Wireless Bluetooth Headphones",
  "description": "High-quality wireless headphones with noise cancellation",
  "category": "electronics",
  "price": 199.99,
  "in_stock": true,
  "created_at": "2024-01-15T10:30:00Z"
}
```

**Example bulk file (`products.json`):**
```json
[
  {
    "title": "Gaming Mouse",
    "description": "High-precision gaming mouse with RGB lighting",
    "category": "electronics",
    "price": 79.99,
    "in_stock": true,
    "created_at": "2024-01-15T10:30:00Z"
  },
  {
    "title": "Mechanical Keyboard",
    "description": "Cherry MX switches mechanical keyboard",
    "category": "electronics",
    "price": 149.99,
    "in_stock": false,
    "created_at": "2024-01-15T11:00:00Z"
  }
]
```

### 3. Search Documents

```bash
# Basic text search
lexum search products "wireless headphones"

# Search with limit
lexum search products "gaming" --limit 5

# Search with sorting
lexum search products "electronics" --sort price:desc

# Search with field selection
lexum search products "keyboard" --fields title,price,category

# Search with multiple sort fields
lexum search products "electronics" --sort price:asc,created_at:desc
```

### 4. Advanced Search Queries

```bash
# Field-specific search
lexum search products "title:gaming"

# Phrase search
lexum search products "description:\"wireless headphones\""

# Range search
lexum search products "price:[50,200]"

# Fuzzy search
lexum search products "title:~gaming"

# Boolean search
lexum search products "+category:electronics -in_stock:false"
```

### 5. LQL (Lexum Query Language)

```bash
# Basic LQL query
lexum lql products "FROM products WHERE category:electronics"

# LQL with conditions
lexum lql products "FROM products WHERE price:[100,500] AND in_stock:true"

# LQL with sorting
lexum lql products "FROM products WHERE category:electronics" --sort price:desc

# LQL from file
lexum lql products "@complex_query.lql"
```

**Example LQL file (`complex_query.lql`):**
```sql
FROM products 
WHERE category:electronics 
  AND price:[50,300] 
  AND in_stock:true
```

### 6. Index Management

```bash
# List all indices
lexum index list

# Get index information
lexum index get products

# Get index statistics
lexum index stats products

# Delete an index
lexum index delete products
```

### 7. Interactive REPL

```bash
# Start interactive mode
lexum

# Or explicitly
lexum repl
```

**REPL Commands:**
```
lexum> help
lexum> index list
lexum> search products "wireless"
lexum> lql products "FROM products WHERE price:[100,200]"
lexum> exit
```

## Advanced Examples

### 1. E-commerce Product Search

```bash
# Create products index
lexum index create products --schema product_schema.yml

# Add sample products
lexum doc bulk products --file sample_products.json

# Search for gaming products under $200
lexum search products "category:gaming AND price:[0,200]"

# Search for wireless products with fuzzy matching
lexum search products "title:~wireless"

# Search for products in stock, sorted by price
lexum search products "in_stock:true" --sort price:asc
```

### 2. Content Management System

```bash
# Create articles index
lexum index create articles --schema article_schema.yml

# Add articles
lexum doc bulk articles --file articles.json

# Search for recent articles about technology
lexum lql articles "FROM articles WHERE category:technology AND created_at:[2024-01-01,2024-12-31]"

# Search for articles with specific tags
lexum search articles "tags:rust OR tags:programming"
```

### 3. Log Analysis

```bash
# Create logs index
lexum index create logs --schema log_schema.yml

# Add log entries
lexum doc bulk logs --file logs.json

# Search for error logs
lexum search logs "level:error"

# Search for logs from specific time range
lexum lql logs "FROM logs WHERE timestamp:[2024-01-15T00:00:00Z,2024-01-15T23:59:59Z]"

# Search for logs from specific IP
lexum search logs "ip:192.168.1.100"
```

## File-based Operations

### Query Files

Create query files for complex searches:

**`price_range_query.lql`:**
```sql
FROM products 
WHERE price:[100,500] 
  AND category:electronics
  AND in_stock:true
```

**`search_products.json`:**
```json
{
  "query": {
    "bool": {
      "must": [
        {
          "term": {
            "category": "electronics"
          }
        },
        {
          "range": {
            "price": {
              "gte": 100,
              "lte": 500
            }
          }
        }
      ]
    }
  },
  "limit": 20,
  "sort": [
    {
      "price": "asc"
    }
  ]
}
```

### Usage with Query Files

```bash
# Execute LQL from file
lexum lql products "@price_range_query.lql"

# Execute JSON query from file
lexum search products "@search_products.json"
```

## Tips and Best Practices

1. **Use appropriate field types** in your schema for better search performance
2. **Index frequently searched fields** as keywords for exact matches
3. **Use text fields** for full-text search capabilities
4. **Leverage sorting** to control result ordering
5. **Use field selection** to reduce response size for large documents
6. **Test queries** in the interactive REPL before using in scripts
7. **Use file-based queries** for complex, reusable search patterns
8. **Monitor index statistics** to understand your data distribution

## Troubleshooting

### Common Issues

1. **Index not found**: Make sure the index exists with `lexum index list`
2. **No results**: Check your query syntax and field names
3. **Connection refused**: Ensure the Lexum server is running
4. **Invalid JSON**: Validate your document JSON before adding

### Getting Help

```bash
# Show help for specific commands
lexum search --help
lexum lql --help
lexum index --help

# Use interactive help in REPL
lexum
lexum> help
```