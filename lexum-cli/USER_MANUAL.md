# Lexum CLI User Manual

**Version**: 0.1.0-alpha  
**Last Updated**: 2025-10-25

## Overview

The Lexum CLI is a command-line interface for managing and interacting with the Lexum search engine. It provides both interactive (REPL) and non-interactive modes for all Lexum operations.

## Installation

### From Source

```bash
git clone https://github.com/your-org/lexum.git
cd lexum
cargo build --release --bin lexum
```

### Binary Installation

```bash
# Download the latest release binary
wget https://github.com/your-org/lexum/releases/latest/download/lexum-linux-x86_64
chmod +x lexum-linux-x86_64
sudo mv lexum-linux-x86_64 /usr/local/bin/lexum
```

## Quick Start

### Interactive Mode (REPL)

Start an interactive session:

```bash
lexum repl
```

This opens a REPL where you can type commands interactively:

```
lexum> help
lexum> index list
lexum> search my_index "hello world"
lexum> exit
```

### Non-Interactive Mode

Execute single commands:

```bash
lexum index list
lexum search my_index "hello world"
lexum doc add my_index document.json
```

## Global Options

| Option | Short | Description | Default |
|--------|-------|-------------|---------|
| `--url` | `-u` | Server URL | `http://localhost:9200` |
| `--format` | `-f` | Output format | `table` |
| `--help` | `-h` | Show help | - |
| `--version` | `-V` | Show version | - |

### Output Formats

- `table` - Human-readable table format
- `json` - Raw JSON output
- `json-pretty` - Pretty-printed JSON

## Commands

### Server Management

#### Start Server

```bash
lexum server start [--config <file>] [--daemon]
```

**Options:**
- `--config, -c` - Configuration file path (default: `config.yml`)
- `--daemon, -d` - Run as daemon process

**Examples:**
```bash
lexum server start
lexum server start --config /etc/lexum/config.yml --daemon
```

#### Stop Server

```bash
lexum server stop
```

#### Server Status

```bash
lexum server status
```

#### Validate Configuration

```bash
lexum server config [--file <path>]
```

**Options:**
- `--file, -f` - Configuration file path (default: `config.yml`)

### Index Management

#### List Indices

```bash
lexum index list
```

**Output:**
```
Name        Documents  Size    Health  Status
my_index    1000      2.5MB   green   open
logs        5000      15.2MB  yellow  open
```

#### Create Index

```bash
lexum index create <name> [--schema <file>]
```

**Options:**
- `--schema, -s` - Schema definition file (YAML)

**Examples:**
```bash
lexum index create my_index
lexum index create products --schema product_schema.yml
```

#### Get Index Information

```bash
lexum index get <name>
```

#### Get Index Statistics

```bash
lexum index stats <name>
```

#### Delete Index

```bash
lexum index delete <name>
```

**Warning:** This permanently deletes the index and all its data.

### Document Operations

#### Add Document

```bash
lexum doc add <index> <file>
```

**Examples:**
```bash
lexum doc add my_index document.json
lexum doc add products product_123.json
```

**Document Format:**
```json
{
  "title": "Sample Document",
  "content": "This is the document content",
  "tags": ["tag1", "tag2"],
  "timestamp": "2025-10-25T10:00:00Z"
}
```

#### Get Document

```bash
lexum doc get <index> <id>
```

**Examples:**
```bash
lexum doc get my_index doc_123
lexum doc get products product_456
```

#### Delete Document

```bash
lexum doc delete <index> <id>
```

#### Bulk Operations

```bash
lexum doc bulk <index> <file>
```

**Bulk File Format:**
```json
{"index": {"_index": "my_index", "_id": "1"}}
{"title": "Document 1", "content": "Content 1"}
{"index": {"_index": "my_index", "_id": "2"}}
{"title": "Document 2", "content": "Content 2"}
```

### Search Operations

#### Basic Search

```bash
lexum search <index> <query> [options]
```

**Options:**
- `--limit, -l` - Maximum number of results (default: 10)
- `--offset, -o` - Number of results to skip (default: 0)
- `--sort` - Sort field and direction (e.g., `score:desc`, `title:asc`)
- `--fields` - Fields to return (comma-separated)
- `--format` - Output format override

**Examples:**
```bash
lexum search my_index "hello world"
lexum search products "laptop" --limit 20 --sort "price:asc"
lexum search logs "error" --fields "timestamp,message" --limit 5
```

#### Advanced Search with File

```bash
lexum search <index> @query.json
```

**Query File Format:**
```json
{
  "query": {
    "bool": {
      "must": [
        {"match": {"title": "laptop"}},
        {"range": {"price": {"gte": 500, "lte": 1000}}}
      ]
    }
  },
  "sort": [{"price": {"order": "asc"}}],
  "size": 20
}
```

### LQL (Lexum Query Language)

#### Execute LQL Query

```bash
lexum lql <query> [options]
```

**Options:**
- `--limit, -l` - Maximum number of results
- `--sort` - Sort specification
- `--fields` - Fields to return

**Examples:**
```bash
lexum lql "FROM products WHERE price > 100"
lexum lql "SELECT title, price FROM products ORDER BY price DESC LIMIT 10"
lexum lql "MATCH 'laptop' IN title,description"
```

#### LQL Query from File

```bash
lexum lql @query.lql
```

**LQL File Example:**
```sql
-- Find expensive laptops
SELECT title, price, brand
FROM products
WHERE category = 'laptop' AND price > 1000
ORDER BY price DESC
LIMIT 20;
```

### Snapshot Management

#### List Snapshots

```bash
lexum snapshot list
```

#### Create Snapshot

```bash
lexum snapshot create <name> [--indices <indices>] [--wait]
```

**Options:**
- `--indices` - Comma-separated list of indices (default: all)
- `--wait` - Wait for completion

**Examples:**
```bash
lexum snapshot create backup_2025_10_25
lexum snapshot create products_backup --indices "products,products_v2" --wait
```

#### Get Snapshot Information

```bash
lexum snapshot get <name>
```

#### Delete Snapshot

```bash
lexum snapshot delete <name>
```

#### List Repositories

```bash
lexum snapshot list-repos
```

### Template Management

#### List Templates

```bash
lexum template list
```

#### Create Template

```bash
lexum template create <name> [--pattern <pattern>] [--priority <priority>]
```

**Options:**
- `--pattern` - Index pattern (default: `*`)
- `--priority` - Template priority (default: 0)

#### Get Template

```bash
lexum template get <name>
```

#### Delete Template

```bash
lexum template delete <name>
```

## Interactive Mode (REPL)

### Starting REPL

```bash
lexum repl
```

### REPL Features

- **Tab Completion**: Press Tab to complete commands and parameters
- **Command History**: Use Up/Down arrows to navigate history
- **Syntax Highlighting**: Commands are color-coded for better readability
- **Error Suggestions**: Get helpful suggestions when commands fail
- **Multi-line Support**: Use `\` at end of line for multi-line commands

### REPL Commands

| Command | Description |
|---------|-------------|
| `help` | Show available commands |
| `clear` | Clear the screen |
| `exit`, `quit` | Exit the REPL |
| `index <subcommand>` | Index management |
| `doc <subcommand>` | Document operations |
| `search <index> <query>` | Search documents |
| `lql <query>` | Execute LQL query |
| `server <subcommand>` | Server management |
| `snapshot <subcommand>` | Snapshot operations |
| `template <subcommand>` | Template management |

### REPL Examples

```
lexum> help
Available commands:
Index Management:
  index list                    - List all indices
  index create <name>           - Create a new index
  ...

lexum> index list
Name        Documents  Size    Health  Status
my_index    1000      2.5MB   green   open

lexum> search my_index "hello world"
{
  "hits": {
    "total": {"value": 5},
    "hits": [
      {
        "_id": "1",
        "_score": 1.0,
        "_source": {
          "title": "Hello World",
          "content": "This is a hello world document"
        }
      }
    ]
  }
}

lexum> exit
Goodbye!
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `LEXUM_URL` | Server URL | `http://localhost:9200` |
| `LEXUM_CONFIG` | Config file path | `config.yml` |
| `LEXUM_FORMAT` | Default output format | `table` |

### Configuration File

Create a `config.yml` file:

```yaml
server:
  host: "0.0.0.0"
  port: 9200
  workers: 4

logging:
  level: "info"
  format: "json"

indices:
  default_shards: 1
  default_replicas: 0

search:
  default_size: 10
  max_size: 1000
  timeout: "30s"
```

## Examples

### Complete Workflow

```bash
# 1. Start server
lexum server start --daemon

# 2. Create an index
lexum index create products --schema product_schema.yml

# 3. Add documents
lexum doc add products product1.json
lexum doc add products product2.json

# 4. Search documents
lexum search products "laptop" --limit 10

# 5. Create snapshot
lexum snapshot create backup_$(date +%Y%m%d) --wait

# 6. Stop server
lexum server stop
```

### Batch Processing

```bash
# Process multiple files
for file in documents/*.json; do
    lexum doc add my_index "$file"
done

# Search with different parameters
lexum search my_index "error" --limit 100 --sort "timestamp:desc"
lexum search my_index "warning" --fields "timestamp,level,message"
```

### LQL Queries

```bash
# Simple queries
lexum lql "FROM products WHERE price > 100"
lexum lql "SELECT title, price FROM products ORDER BY price DESC LIMIT 10"

# Complex queries
lexum lql "SELECT COUNT(*) FROM logs WHERE level = 'ERROR' AND timestamp > '2025-10-01'"
lexum lql "SELECT category, AVG(price) FROM products GROUP BY category"
```

## Troubleshooting

### Common Issues

#### Connection Refused

```
Error: Failed to connect to server
```

**Solution:**
- Check if server is running: `lexum server status`
- Verify URL: `lexum --url http://localhost:9200 index list`
- Check firewall settings

#### Index Not Found

```
Error: Index 'my_index' not found
```

**Solution:**
- List indices: `lexum index list`
- Create index: `lexum index create my_index`

#### Permission Denied

```
Error: Permission denied
```

**Solution:**
- Check file permissions
- Run with appropriate user privileges
- Verify server configuration

#### Invalid JSON

```
Error: Invalid JSON in document
```

**Solution:**
- Validate JSON: `cat document.json | jq .`
- Check file encoding (should be UTF-8)
- Fix JSON syntax errors

### Debug Mode

Enable debug logging:

```bash
export RUST_LOG=debug
lexum index list
```

### Verbose Output

Use verbose mode for detailed information:

```bash
lexum --format json index list
```

## Advanced Usage

### Scripting

Create shell scripts for automation:

```bash
#!/bin/bash
# backup_indices.sh

DATE=$(date +%Y%m%d_%H%M%S)
SNAPSHOT_NAME="backup_$DATE"

echo "Creating snapshot: $SNAPSHOT_NAME"
lexum snapshot create "$SNAPSHOT_NAME" --wait

if [ $? -eq 0 ]; then
    echo "Snapshot created successfully"
else
    echo "Failed to create snapshot"
    exit 1
fi
```

### Integration with CI/CD

```yaml
# .github/workflows/search-test.yml
name: Search Test
on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - name: Start Lexum
        run: |
          lexum server start --daemon
          sleep 10
      - name: Test Search
        run: |
          lexum index create test_index
          lexum doc add test_index test_document.json
          lexum search test_index "test query"
      - name: Stop Lexum
        run: lexum server stop
```

## API Reference

### Command Structure

```
lexum [GLOBAL_OPTIONS] <COMMAND> [COMMAND_OPTIONS] [ARGUMENTS]
```

### Exit Codes

| Code | Description |
|------|-------------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Server connection error |
| 4 | File I/O error |
| 5 | JSON parsing error |

### Output Formats

#### Table Format
```
Name        Documents  Size    Health  Status
my_index    1000      2.5MB   green   open
```

#### JSON Format
```json
{
  "indices": [
    {
      "name": "my_index",
      "documents": 1000,
      "size": "2.5MB",
      "health": "green",
      "status": "open"
    }
  ]
}
```

## Support

- **Documentation**: [https://docs.lexum.dev](https://docs.lexum.dev)
- **Issues**: [https://github.com/your-org/lexum/issues](https://github.com/your-org/lexum/issues)
- **Discussions**: [https://github.com/your-org/lexum/discussions](https://github.com/your-org/lexum/discussions)

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.