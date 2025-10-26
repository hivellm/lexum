# Lexum CLI User Manual

Complete guide to using the Lexum command-line interface for search engine operations.

## Table of Contents

1. [Installation](#installation)
2. [Quick Start](#quick-start)
3. [Interactive REPL](#interactive-repl)
4. [Command Reference](#command-reference)
5. [LQL Query Language](#lql-query-language)
6. [Configuration](#configuration)
7. [Examples](#examples)
8. [Troubleshooting](#troubleshooting)

## Installation

### From Source

```bash
# Clone the repository
git clone https://github.com/hivellm/lexum.git
cd lexum

# Build the CLI
cargo build --package lexum-cli --release

# Install globally (optional)
cargo install --path lexum-cli
```

### Binary Release

Download the latest release from the [GitHub releases page](https://github.com/hivellm/lexum/releases).

## Quick Start

1. **Start the Lexum server**:
   ```bash
   lexum-server
   ```

2. **Open the CLI**:
   ```bash
   lexum
   ```

3. **Create your first index**:
   ```bash
   index create my_index --schema schema.yml
   ```

4. **Add some documents**:
   ```bash
   doc add my_index --file documents.json
   ```

5. **Search your data**:
   ```bash
   search my_index "hello world"
   ```

## Interactive REPL

The Lexum CLI provides an interactive Read-Eval-Print Loop (REPL) for exploring your data.

### Starting the REPL

```bash
# Start interactive mode
lexum

# Or explicitly
lexum repl
```

### REPL Features

- **Tab Completion**: Press `Tab` to autocomplete commands and options
- **Command History**: Use `↑`/`↓` arrows to navigate command history
- **Colored Output**: Syntax highlighting and colored results
- **Help System**: Type `help` for available commands
- **Exit**: Type `exit`, `quit`, or press `Ctrl+D`

### REPL Commands

| Command | Description | Example |
|---------|-------------|---------|
| `help` | Show help information | `help` |
| `exit` | Exit the REPL | `exit` |
| `index` | Index management | `index list` |
| `doc` | Document operations | `doc add my_index --file data.json` |
| `search` | Search documents | `search my_index "query"` |
| `lql` | LQL query language | `lql my_index "FROM my_index WHERE title:hello"` |
| `server` | Server management | `server status` |
| `snapshot` | Snapshot operations | `snapshot list-repos` |

### Enhanced REPL Features

The Lexum CLI REPL includes several advanced features to improve the user experience:

#### Tab Completion

The REPL supports comprehensive tab completion for commands, options, and query patterns:

```bash
lexum> <TAB>
help    exit    quit    index   doc     search  server  snapshot lql

lexum> search <TAB>
# Shows search query patterns and options

lexum> search products <TAB>
# Shows query patterns like *, field:value, field:"phrase", etc.

lexum> search products "test" --<TAB>
--limit     --offset    --sort      --fields    --highlight --explain   --min-score
```

#### Command Suggestions

When you make a typo or use an invalid command, the REPL provides intelligent suggestions:

```bash
lexum> indx list
Error: Command not found
💡 Did you mean one of these commands?
  index list
  help
  Type 'help' for complete command reference

lexum> serach products "test"
Error: Command not found
💡 Did you mean one of these commands?
  search <index> <query>
  help
  Type 'help' for complete command reference
```

#### Enhanced Error Handling

The REPL provides detailed error messages and suggestions for common mistakes:

- **Invalid commands**: Suggests similar commands
- **Missing arguments**: Shows usage information
- **Invalid options**: Lists available options
- **Connection errors**: Provides troubleshooting tips

## Command Reference

### Global Options

| Option | Description | Default |
|--------|-------------|---------|
| `--url` | Server URL | `http://localhost:9200` |
| `--format` | Output format (json, table, pretty) | `pretty` |
| `--help` | Show help | - |
| `--version` | Show version | - |

### Index Management

#### List Indices

```bash
lexum index list [--format json|table|pretty]
```

Lists all available indices with their basic information.

#### Create Index

```bash
lexum index create <name> --schema <schema_file>
```

Creates a new index with the specified schema.

**Example**:
```bash
lexum index create products --schema product_schema.yml
```

#### Get Index Information

```bash
lexum index get <name> [--format json|table|pretty]
```

Retrieves detailed information about an index.

#### Index Statistics

```bash
lexum index stats <name> [--format json|table|pretty]
```

Shows statistics about an index (document count, size, etc.).

#### Delete Index

```bash
lexum index delete <name>
```

Deletes an index and all its data.

**Warning**: This action cannot be undone!

### Document Operations

#### Add Document

```bash
lexum doc add <index> --file <file> [--id <id>]
```

Adds a document to an index from a JSON file.

**Examples**:
```bash
# Add from file
lexum doc add products --file product.json

# Add with specific ID
lexum doc add products --file product.json --id "prod_123"
```

#### Get Document

```bash
lexum doc get <index> <id> [--format json|table|pretty]
```

Retrieves a document by its ID.

#### Delete Document

```bash
lexum doc delete <index> <id>
```

Deletes a document by its ID.

#### Bulk Operations

```bash
lexum doc bulk <index> --file <file>
```

Performs bulk document operations from a JSON file.

**Bulk file format**:
```json
[
  {
    "title": "Product 1",
    "price": 99.99
  },
  {
    "title": "Product 2", 
    "price": 149.99
  }
]
```

### Search Operations

#### Basic Search

```bash
lexum search <index> <query> [options]
```

Searches documents in an index.

**Options:**
- `--limit <number>`: Maximum number of results (default: 10)
- `--offset <number>`: Number of results to skip for pagination (default: 0)
- `--sort <field:order>`: Sort results by field (asc/desc)
- `--fields <field1,field2>`: Return only specified fields
- `--highlight`: Highlight search terms in results
- `--explain`: Show query execution details and performance metrics
- `--min-score <number>`: Minimum score threshold for results

**Examples**:
```bash
# Basic text search
lexum search products "wireless headphones"

# Search with limit
lexum search products "gaming" --limit 20

# Search with sorting
lexum search products "electronics" --sort price:desc

# Search with field selection
lexum search products "keyboard" --fields title,price,category

# Search with pagination
lexum search products "electronics" --limit 10 --offset 20

# Search with highlighting
lexum search products "wireless headphones" --highlight

# Search with query explanation
lexum search products "gaming" --explain

# Search with minimum score
lexum search products "electronics" --min-score 0.5

# Combined advanced search
lexum search products "wireless" \
  --limit 5 \
  --offset 0 \
  --sort price:desc \
  --fields title,price,description \
  --highlight \
  --explain \
  --min-score 0.3
```

#### Advanced Query Syntax

```bash
# Field-specific search
lexum search products "title:gaming"

# Phrase search
lexum search products "description:\"wireless headphones\""

# Range search
lexum search products "price:[100,500]"

# Fuzzy search
lexum search products "title:~gaming"

# Boolean search
lexum search products "+category:electronics -status:discontinued"
```

#### File-based Queries

```bash
# Query from file
lexum search products @query.json --limit 100
```

### LQL (Lexum Query Language)

#### Basic LQL

```bash
lexum lql <index> <lql_query> [--limit N] [--sort field:asc/desc] [--fields field1,field2]
```

Executes LQL queries against an index.

**Examples**:
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

#### LQL Syntax

- **FROM queries**: `FROM <index> [WHERE <conditions>]`
- **SELECT queries**: `SELECT * FROM <index> [WHERE <conditions>]`
- **MATCH queries**: `MATCH <field>:<value>`

#### LQL Conditions

- **Exact match**: `field:value`
- **Range match**: `field:[min,max]`
- **Fuzzy match**: `field:~value`
- **Phrase match**: `field:"exact phrase"`
- **Boolean AND**: `+field:value`
- **Boolean NOT**: `-field:value`

### Server Management

#### Start Server

```bash
lexum server start [config_file] [--daemon]
```

Starts the Lexum server.

#### Stop Server

```bash
lexum server stop
```

Stops the Lexum server.

#### Server Status

```bash
lexum server status
```

Checks the server status.

#### Validate Configuration

```bash
lexum server config [file]
```

Validates a configuration file.

### Snapshot Management

#### List Repositories

```bash
lexum snapshot list-repos
```

Lists all snapshot repositories.

#### List Snapshots

```bash
lexum snapshot list <repository>
```

Lists snapshots in a repository.

#### Create Snapshot

```bash
lexum snapshot create <repository> <snapshot> [--indices INDEX1,INDEX2] [--wait]
```

Creates a snapshot.

#### Delete Snapshot

```bash
lexum snapshot delete <repository> <snapshot>
```

Deletes a snapshot.

## LQL Query Language

LQL (Lexum Query Language) provides a SQL-like syntax for complex queries.

### Basic Syntax

```sql
FROM <index> [WHERE <conditions>]
SELECT * FROM <index> [WHERE <conditions>]
MATCH <field>:<value>
```

### Query Types

#### FROM Queries

```sql
-- Match all documents
FROM products

-- Match with conditions
FROM products WHERE title:laptop

-- Multiple conditions
FROM products WHERE category:electronics AND price:[100,500]
```

#### SELECT Queries

```sql
-- Select all fields
SELECT * FROM products WHERE category:books

-- Select specific fields (planned feature)
SELECT title, price FROM products WHERE category:electronics
```

#### MATCH Queries

```sql
-- Simple field match
MATCH title:laptop

-- Phrase match
MATCH description:"wireless headphones"

-- Range match
MATCH price:[100,500]

-- Fuzzy match
MATCH title:~laptp
```

### Condition Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `:` | Exact match | `title:laptop` |
| `[min,max]` | Range match | `price:[100,500]` |
| `~` | Fuzzy match | `title:~laptp` |
| `"phrase"` | Phrase match | `description:"wireless headphones"` |
| `+` | Boolean AND | `+category:electronics` |
| `-` | Boolean NOT | `-status:discontinued` |
| `AND` | Logical AND | `title:laptop AND price:[100,500]` |
| `OR` | Logical OR | `category:electronics OR category:computers` |

### Examples

```sql
-- Find all electronics under $200
FROM products WHERE category:electronics AND price:[0,200]

-- Find laptops with wireless capability
FROM products WHERE title:laptop AND description:"wireless"

-- Find products with fuzzy title matching
FROM products WHERE title:~gaming

-- Find products excluding discontinued ones
FROM products WHERE +category:electronics -status:discontinued

-- Complex boolean query
FROM products WHERE (category:electronics OR category:computers) AND price:[100,1000] AND in_stock:true
```

## Configuration

### Server Configuration

The Lexum server can be configured using a YAML file:

```yaml
# config.yml
server:
  host: "0.0.0.0"
  port: 9200

storage:
  data_path: "./data"

logging:
  level: "info"
  format: "json"

search:
  cache:
    enabled: true
    max_size_mb: 100
    ttl_seconds: 3600
```

### CLI Configuration

The CLI can be configured using environment variables:

```bash
# Set default server URL
export LEXUM_URL="http://localhost:9200"

# Set default output format
export LEXUM_FORMAT="json"

# Set API key for authentication
export LEXUM_API_KEY="your-api-key"
```

## Examples

### E-commerce Product Search

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

### Content Management System

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

### Log Analysis

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

### File-based Operations

Create query files for complex searches:

**`price_range_query.lql`**:
```sql
FROM products 
WHERE price:[100,500] 
  AND category:electronics
  AND in_stock:true
```

**`search_products.json`**:
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

**Usage**:
```bash
# Execute LQL from file
lexum lql products "@price_range_query.lql"

# Execute JSON query from file
lexum search products "@search_products.json"
```

## Troubleshooting

### Common Issues

#### Index not found
```
Error: Index 'my_index' not found
```
**Solution**: Check if the index exists with `lexum index list`

#### No results returned
```
Found 0 results in 5ms
```
**Solution**: 
- Check your query syntax
- Verify field names exist in the index
- Try a broader search query like `*`

#### Connection refused
```
Error: Connection refused
```
**Solution**: 
- Ensure the Lexum server is running
- Check the server URL with `--url` option
- Verify the server is listening on the correct port

#### Invalid JSON
```
Error: Invalid JSON in document
```
**Solution**: 
- Validate your JSON before adding documents
- Use a JSON validator tool
- Check for trailing commas or missing quotes

#### Permission denied
```
Error: Permission denied
```
**Solution**: 
- Check file permissions
- Ensure you have write access to the data directory
- Run with appropriate user permissions

### Debug Mode

Enable debug logging for troubleshooting:

```bash
# Set debug level
export RUST_LOG=debug

# Run CLI with debug output
lexum search products "query"
```

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

### Performance Tips

1. **Use appropriate field types** in your schema for better search performance
2. **Index frequently searched fields** as keywords for exact matches
3. **Use text fields** for full-text search capabilities
4. **Leverage sorting** to control result ordering
5. **Use field selection** to reduce response size for large documents
6. **Test queries** in the interactive REPL before using in scripts
7. **Use file-based queries** for complex, reusable search patterns
8. **Monitor index statistics** to understand your data distribution

### Support

For additional help:

- **Documentation**: Check the [project documentation](https://github.com/hivellm/lexum/docs)
- **Issues**: Report bugs on [GitHub Issues](https://github.com/hivellm/lexum/issues)
- **Discussions**: Join discussions on [GitHub Discussions](https://github.com/hivellm/lexum/discussions)