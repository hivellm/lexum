# Lexum CLI

Command-line interface for the Lexum search engine.

## Features

- **Interactive REPL**: Start an interactive shell for exploring your data
- **Index Management**: Create, list, and delete indices
- **Document Operations**: Add, retrieve, and delete documents
- **Search**: Execute search queries from the command line
- **LQL Support**: Advanced query language with SQL-like syntax
- **Snapshot Management**: Create, list, and manage snapshots
- **Colored Output**: Beautiful terminal output with syntax highlighting
- **Command History**: Navigate previous commands with arrow keys
- **Tab Completion**: Smart autocomplete for commands and options
- **File Operations**: Load data and queries from filesrow keys

## Installation

```bash
cargo install --path lexum-cli## Quick Start

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

# Get index statistics
lexum index stats my_index

# Delete index
lexum index delete my_index
```

### Document Operations

```bash
# Add document from JSON file
lexum doc add my_index --file document.json

# Add document with specific ID
lexum doc add my_index --file document.json --id "doc_123"

# Get document by ID
lexum doc get my_index doc_123

# Delete document by ID
lexum doc delete my_index doc_123

# Bulk operations
lexum doc bulk my_index --file documents.json
```

### Search Operations

```bash
# Basic search
lexum search my_index "search query" --limit 20

# Search with sorting
lexum search my_index "laptop" --sort price:asc

# Search from file
lexum search my_index @query.json

# LQL query language
lexum lql my_index "FROM my_index WHERE title:laptop AND price:[100,500]"
```

### Snapshot Management

```bash
# List snapshot repositories
lexum snapshot list-repos

# List snapshots in a repository
lexum snapshot list my_repo

# Get snapshot information
lexum snapshot get my_repo snapshot_1

# Create a snapshot
lexum snapshot create my_repo backup_2024 --indices index1,index2 --wait

# Delete a snapshot
lexum snapshot delete my_repo old_backup

# Get repository information
lexum snapshot repo my_repo
```atch all documents
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
lexum -## Documentation

For complete documentation, see the [User Manual](USER_MANUAL.md).

## License

Apache-2.09200 index list
```

## License

Apache-2.0

