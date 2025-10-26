# Lexum CLI - Basic Usage Examples

This document provides comprehensive examples of using the Lexum CLI tool.

## Table of Contents

1. [Installation](#installation)
2. [Basic Commands](#basic-commands)
3. [Server Management](#server-management)
4. [Index Operations](#index-operations)
5. [Document Operations](#document-operations)
6. [Search Operations](#search-operations)
7. [LQL (Lexum Query Language)](#lql-lexum-query-language)
8. [Snapshot Management](#snapshot-management)
9. [Interactive REPL](#interactive-repl)
10. [Advanced Examples](#advanced-examples)

## Installation

```bash
# Build from source
cargo build --release

# Install globally
cargo install --path .

# Or run directly
cargo run -- --help
```

## Basic Commands

### Help and Version

```bash
# Show help
lexum --help

# Show version
lexum --version

# Show help for specific command
lexum search --help
lexum index --help
```

### Global Options

```bash
# Specify server URL
lexum --url http://localhost:9200 search my_index "query"

# Specify output format
lexum --format json search my_index "query"
lexum --format table search my_index "query"
lexum --format json-pretty search my_index "query"
```

## Server Management

### Start Server

```bash
# Start server with default config
lexum server start

# Start server with custom config
lexum server start --config /path/to/config.yml

# Start server in daemon mode
lexum server start --daemon
```

### Server Status

```bash
# Check server status
lexum server status

# Validate configuration
lexum server config --validate /path/to/config.yml
```

### Stop Server

```bash
# Stop server
lexum server stop
```

## Index Operations

### List Indices

```bash
# List all indices
lexum index list

# List with custom server
lexum --url http://remote-server:9200 index list
```

### Create Index

```bash
# Create index with default settings
lexum index create my_index

# Create index with custom settings (JSON file)
lexum index create my_index --settings @settings.json
```

### Get Index Information

```bash
# Get index details
lexum index get my_index

# Get index statistics
lexum index stats my_index
```

### Delete Index

```bash
# Delete index
lexum index delete my_index
```

## Document Operations

### Add Document

```bash
# Add single document
lexum doc add my_index @document.json

# Add document with specific ID
lexum doc add my_index --id doc_123 @document.json
```

### Get Document

```bash
# Get document by ID
lexum doc get my_index doc_123
```

### Delete Document

```bash
# Delete document by ID
lexum doc delete my_index doc_123
```

### Bulk Operations

```bash
# Bulk add documents
lexum doc bulk my_index @bulk_documents.json
```

## Search Operations

### Basic Search

```bash
# Simple text search
lexum search my_index "search query"

# Search all documents
lexum search my_index "*"

# Search with limit
lexum search my_index "query" --limit 20
```

### Advanced Search

```bash
# Field-specific search
lexum search my_index "title:rust"

# Phrase search
lexum search my_index "content:\"exact phrase\""

# Range search
lexum search my_index "price:[100,500]"

# Fuzzy search
lexum search my_index "name:~fuzzy"

# Boolean search
lexum search my_index "+category:tech -status:deprecated"
```

### Search with Options

```bash
# Search with sorting
lexum search my_index "query" --sort title:asc --sort price:desc

# Search with field selection
lexum search my_index "query" --fields title,content,price

# Search with highlighting
lexum search my_index "query" --highlight

# Search with explanation
lexum search my_index "query" --explain

# Search with minimum score
lexum search my_index "query" --min-score 0.5

# Search with pagination
lexum search my_index "query" --offset 20 --limit 10
```

### File-based Queries

```bash
# Search using JSON query file
lexum search my_index @query.json

# Search using LQL file
lexum search my_index @query.lql
```

## LQL (Lexum Query Language)

### Basic LQL

```bash
# Simple LQL query
lexum lql my_index "FROM my_index WHERE title:rust"

# LQL with SELECT
lexum lql my_index "SELECT title, content FROM my_index WHERE category:tech"

# LQL with sorting
lexum lql my_index "FROM my_index WHERE price:[100,500] ORDER BY price DESC"
```

### Advanced LQL

```bash
# LQL with aggregation
lexum lql my_index "SELECT COUNT(*) FROM my_index WHERE category:tech"

# LQL with grouping
lexum lql my_index "SELECT category, COUNT(*) FROM my_index GROUP BY category"

# LQL with file input
lexum lql my_index @query.lql
```

## Snapshot Management

### Repository Operations

```bash
# List repositories
lexum snapshot list-repos

# Get repository info
lexum snapshot repo my_repo

# Create repository
lexum snapshot repo-create my_repo --type fs --location /path/to/repo
```

### Snapshot Operations

```bash
# List snapshots
lexum snapshot list my_repo

# Create snapshot
lexum snapshot create my_repo my_snapshot --indices my_index,other_index

# Get snapshot info
lexum snapshot get my_repo my_snapshot

# Delete snapshot
lexum snapshot delete my_repo my_snapshot

# Restore snapshot
lexum snapshot restore my_repo my_snapshot --indices my_index
```

## Interactive REPL

### Start REPL

```bash
# Start interactive session
lexum repl

# Start REPL with custom server
lexum --url http://remote-server:9200 repl
```

### REPL Commands

```bash
# In REPL, you can use all commands without 'lexum' prefix
> index list
> search my_index "query"
> doc add my_index @document.json
> help
> exit
```

## Advanced Examples

### Complex Search Queries

```bash
# Multi-field boolean query
lexum search my_index "+title:rust +category:programming -status:deprecated"

# Nested range queries
lexum search my_index "price:[100,500] AND rating:[4.0,5.0]"

# Fuzzy search with multiple fields
lexum search my_index "title:~rust OR content:~programming"
```

### Batch Operations

```bash
# Create multiple indices
for i in {1..5}; do
  lexum index create "index_$i"
done

# Bulk search across multiple indices
for index in index_1 index_2 index_3; do
  lexum search "$index" "query" --limit 10
done
```

### Integration with Shell Scripts

```bash
#!/bin/bash
# Search and process results
results=$(lexum search my_index "query" --format json)
echo "$results" | jq '.hits[] | .source.title'

# Create index if it doesn't exist
if ! lexum index list | grep -q "my_index"; then
  lexum index create my_index
fi
```

### Configuration Examples

```yaml
# config.yml
server:
  host: "0.0.0.0"
  port: 9200

logging:
  level: "info"
  format: "json"

indices:
  default_shards: 3
  default_replicas: 1
```

## Tips and Best Practices

1. **Use file-based queries** for complex searches
2. **Use the REPL** for interactive exploration
3. **Set appropriate limits** to avoid large result sets
4. **Use field selection** to reduce response size
5. **Use sorting** for consistent result ordering
6. **Use highlighting** for better result visibility
7. **Use pagination** for large result sets
8. **Use snapshots** for data backup and recovery

## Troubleshooting

### Common Issues

1. **Connection refused**: Check if server is running
2. **Index not found**: Create index first
3. **Invalid query syntax**: Check query format
4. **Permission denied**: Check file permissions for file-based operations

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug lexum search my_index "query"
```

### Verbose Output

```bash
# Use verbose mode for more information
lexum --verbose search my_index "query"
```
