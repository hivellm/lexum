# Lexum CLI

Command-line interface for the Lexum search engine.

## Features

- **Interactive REPL**: Start an interactive shell for exploring Lexum
- **Index Management**: Create, list, and delete indices
- **Document Operations**: Add and retrieve documents
- **Search**: Execute search queries from the command line
- **Colored Output**: Beautiful terminal output with syntax highlighting
- **Command History**: Navigate previous commands with arrow keys

## Installation

```bash
cargo install --path lexum-cli
```

## Usage

### Interactive REPL

Start the interactive shell:

```bash
lexum repl
```

Or simply:

```bash
lexum
```

### Index Management

```bash
# List all indices
lexum index list

# Create index from schema file
lexum index create my_index --schema schema.yml

# Get index info
lexum index get my_index

# Delete index
lexum index delete my_index
```

### Document Operations

```bash
# Add document from JSON file
lexum doc add my_index --file document.json

# Get document by ID
lexum doc get my_index doc_123
```

### Search

```bash
# Search documents
lexum search my_index "search query" --limit 20

# Match all documents
lexum search my_index "*"
```

## Schema File Format

Create a YAML file defining your index schema:

```yaml
- name: title
  type: text
  stored: true
  indexed: true

- name: content
  type: text
  stored: true
  indexed: true

- name: views
  type: i64
  stored: true
  fast: true
```

Supported field types:
- `text` - Full-text search
- `keyword` - Exact matching
- `i64` - 64-bit integer
- `f64` - 64-bit float
- `date` - Date/timestamp
- `boolean` - True/false

## Configuration

Use the `--url` flag to specify a different server:

```bash
lexum --url http://localhost:9200 index list
```

## License

Apache-2.0

