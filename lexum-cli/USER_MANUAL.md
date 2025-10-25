# Lexum CLI User Manual

Complete guide to using the Lexum command-line interface.

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
{
  "operations": [
    {
      "action": "add",
      "document": {"title": "Product 1", "price": 29.99}
    },
    {
      "action": "add",
      "document": {"title": "Product 2", "price": 39.99},
      "id": "custom_id"
    },
    {
      "action": "delete",
      "id": "old_product"
    }
  ]
}
```

### Search Operations

#### Basic Search

```bash
lexum search <index> <query> [options]
```

**Options**:
- `--limit <number>`: Limit number of results (default: 10)
- `--offset <number>`: Skip number of results (default: 0)
- `--sort <field:asc|desc>`: Sort results by field
- `--fields <field1,field2>`: Return only specified fields
- `--format <format>`: Output format

**Examples**:
```bash
# Basic search
lexum search products "laptop"

# Search with options
lexum search products "laptop" --limit 20 --sort price:asc

# Search from file
lexum search products @query.json
```

#### LQL Search

```bash
lexum lql <index> <query> [--format json|table|pretty]
```

Executes LQL (Lexum Query Language) queries.

**Examples**:
```bash
# LQL query
lexum lql products "FROM products WHERE price:[100,500] AND category:electronics"

# LQL from file
lexum lql products @complex_query.lql
```

### Server Management

#### Server Status

```bash
lexum server status
```

Shows server health and basic information.

#### Server Configuration

```bash
lexum server config [--file <config_file>]
```

Validates server configuration.

### Snapshot Management

#### List Repositories

```bash
lexum snapshot list-repos [--format json|table|pretty]
```

Lists all snapshot repositories.

#### List Snapshots

```bash
lexum snapshot list <repository> [--format json|table|pretty]
```

Lists snapshots in a repository.

#### Create Snapshot

```bash
lexum snapshot create <repository> <snapshot_name> [options]
```

**Options**:
- `--indices <index1,index2>`: Specific indices to snapshot
- `--wait`: Wait for completion
- `--format <format>`: Output format

#### Get Snapshot

```bash
lexum snapshot get <repository> <snapshot_name> [--format json|table|pretty]
```

Retrieves snapshot information.

#### Delete Snapshot

```bash
lexum snapshot delete <repository> <snapshot_name>
```

Deletes a snapshot.

#### Repository Management

```bash
# Get repository info
lexum snapshot repo <repository> [--format json|table|pretty]

# Create repository (from file)
lexum snapshot repo-create <repository> --file <config_file>
```

## LQL Query Language

LQL (Lexum Query Language) provides a SQL-like syntax for complex queries.

### Basic Syntax

```sql
FROM <index_name>
[| <operation>]*
```

### Query Types

#### FROM Clause

```sql
-- Single index
FROM products

-- Multiple indices
FROM products, reviews

-- Index pattern
FROM logs-*

-- With alias
FROM products AS p
```

#### WHERE Clause

```sql
-- Basic comparison
WHERE title:laptop

-- Phrase search
WHERE title:"gaming laptop"

-- Range queries
WHERE price:[100,500]
WHERE date:[2024-01-01,2024-12-31]

-- Fuzzy search
WHERE title:~laptop

-- Boolean operators
WHERE title:laptop AND price:[100,500]
WHERE category:electronics OR category:computers
WHERE title:laptop AND NOT price:[1000,9999]
```

#### SELECT Clause

```sql
-- Select specific fields
SELECT title, price FROM products WHERE category:electronics

-- Select all fields
SELECT * FROM products WHERE price:[100,500]
```

### Examples

```sql
-- Find expensive electronics
FROM products
WHERE category:electronics AND price:[500,9999]
ORDER BY price DESC
LIMIT 10

-- Search with fuzzy matching
FROM products
WHERE title:~laptop AND brand:apple

-- Complex boolean query
FROM products
WHERE (title:laptop OR title:notebook) AND price:[500,2000] AND NOT category:accessories
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LEXUM_URL` | Server URL | `http://localhost:9200` |
| `LEXUM_FORMAT` | Default output format | `pretty` |
| `LEXUM_CONFIG` | Config file path | - |

### Config File

Create a `~/.lexum/config.yml` file:

```yaml
server:
  url: "http://localhost:9200"
  timeout: 30

cli:
  default_format: "pretty"
  history_size: 1000
  auto_complete: true

logging:
  level: "info"
  format: "json"
```

## Examples

### E-commerce Product Search

1. **Create product index**:
   ```bash
   # schema.yml
   - name: title
     type: text
     stored: true
     indexed: true
   - name: description
     type: text
     stored: true
     indexed: true
   - name: price
     type: f64
     stored: true
     fast: true
   - name: category
     type: keyword
     stored: true
     fast: true
   - name: brand
     type: keyword
     stored: true
     fast: true
   - name: in_stock
     type: boolean
     stored: true
     fast: true
   ```

   ```bash
   lexum index create products --schema schema.yml
   ```

2. **Add products**:
   ```bash
   # products.json
   [
     {
       "title": "Gaming Laptop",
       "description": "High-performance gaming laptop with RTX 4080",
       "price": 1999.99,
       "category": "electronics",
       "brand": "GamingCorp",
       "in_stock": true
     },
     {
       "title": "Wireless Mouse",
       "description": "Ergonomic wireless mouse for productivity",
       "price": 49.99,
       "category": "accessories",
       "brand": "TechBrand",
       "in_stock": true
     }
   ]
   ```

   ```bash
   lexum doc bulk products --file products.json
   ```

3. **Search products**:
   ```bash
   # Find gaming laptops
   lexum search products "gaming laptop" --limit 10

   # Find products by price range
   lexum lql products "FROM products WHERE price:[100,500] AND category:electronics"

   # Find products by brand
   lexum search products "brand:GamingCorp"
   ```

### Log Analysis

1. **Create log index**:
   ```bash
   # log_schema.yml
   - name: timestamp
     type: date
     stored: true
     fast: true
   - name: level
     type: keyword
     stored: true
     fast: true
   - name: message
     type: text
     stored: true
     indexed: true
   - name: service
     type: keyword
     stored: true
     fast: true
   - name: user_id
     type: keyword
     stored: true
     fast: true
   ```

2. **Search logs**:
   ```bash
   # Find error logs
   lexum search logs "level:ERROR" --limit 50

   # Find logs by service
   lexum search logs "service:api" --sort timestamp:desc

   # Complex log query
   lexum lql logs "FROM logs WHERE level:ERROR AND service:api AND timestamp:[2024-01-01,2024-01-31]"
   ```

## Troubleshooting

### Common Issues

#### Connection Refused

```
Error: Connection refused
```

**Solution**: Ensure the Lexum server is running:
```bash
lexum-server
```

#### Index Not Found

```
Error: Index 'my_index' not found
```

**Solution**: Create the index first:
```bash
lexum index create my_index --schema schema.yml
```

#### Invalid Schema

```
Error: Invalid schema format
```

**Solution**: Check your schema file format and field types.

#### Permission Denied

```
Error: Permission denied
```

**Solution**: Check file permissions and server access rights.

### Debug Mode

Enable debug logging:

```bash
RUST_LOG=debug lexum search my_index "query"
```

### Getting Help

1. **Command help**:
   ```bash
   lexum --help
   lexum search --help
   ```

2. **Interactive help**:
   ```bash
   lexum
   help
   ```

3. **Server logs**:
   Check the server logs for detailed error information.

### Performance Tips

1. **Use appropriate field types** for your data
2. **Enable fast fields** for sorting and filtering
3. **Use batch operations** for bulk data loading
4. **Limit result sets** with `--limit` option
5. **Use LQL** for complex queries

---

*This manual covers Lexum CLI v0.1.0-alpha. For the latest updates, visit the [project repository](https://github.com/hivellm/lexum).*