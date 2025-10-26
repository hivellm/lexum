# Development Guide

Complete guide for developing and contributing to Lexum.

## Prerequisites

### Required

- **Rust**: 1.85+ (nightly)
- **Cargo**: Latest version
- **Git**: 2.30+

### Optional

- **Docker**: 20.10+ (for containerized development)
- **Kubernetes**: 1.25+ (for K8s testing)
- **Node.js**: 18+ (for GUI development)

## Quick Start

### Clone Repository

```bash
git clone https://github.com/your-org/lexum
cd lexum
```

### Setup Rust Toolchain

```bash
# Install Rust nightly
rustup install nightly
rustup default nightly

# Install components
rustup component add rustfmt clippy llvm-tools-preview

# Install additional tools
cargo install cargo-nextest cargo-llvm-cov cargo-watch
```

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# With all features
cargo build --all-features
```

### Run

```bash
# Start server
cargo run -- serve --config config.example.yml

# With custom config
cargo run -- serve --config my-config.yml --data-dir ./data
```

### Test

```bash
# Run all tests
cargo test

# Run with nextest (faster)
cargo nextest run

# Run specific test
cargo test test_search_query

# Run with coverage
cargo llvm-cov --all --html
```

## Project Structure

```
lexum/
├── Cargo.toml                 # Workspace manifest
├── Cargo.lock                 # Dependency lock file
├── rust-toolchain.toml        # Rust toolchain config
├── config.example.yml         # Example configuration
├── README.md
├── CHANGELOG.md
├── LICENSE
├── .github/
│   └── workflows/             # CI/CD workflows
├── docs/                      # Documentation
├── lexum-core/                # Core library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── index/             # Indexing engine
│       ├── search/            # Search engine
│       ├── query/             # Query parsing
│       ├── storage/           # Storage layer
│       └── cluster/           # Cluster management
├── lexum-server/              # Server binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── api/               # API handlers
│       ├── config/            # Configuration
│       └── telemetry/         # Observability
├── lexum-cli/                 # CLI tool
│   ├── Cargo.toml
│   └── src/
│       └── main.rs
├── lexum-gui/                 # Electron GUI
│   ├── package.json
│   └── src/
├── tests/                     # Integration tests
├── benchmark/                 # Benchmarks
└── scripts/                   # Utility scripts
```

## Development Workflow

### 1. Create Feature Branch

```bash
git checkout -b feature/my-feature
```

### 2. Make Changes

Edit code following the [coding standards](#coding-standards).

### 3. Format Code

```bash
# Format all code
cargo +nightly fmt --all

# Check formatting
cargo +nightly fmt --all -- --check
```

### 4. Lint

```bash
# Run clippy
cargo clippy --workspace -- -D warnings

# All targets and features
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

### 5. Test

```bash
# Run tests
cargo test --workspace

# With nextest
cargo nextest run --workspace

# Check coverage
cargo llvm-cov --all --html
open target/llvm-cov/html/index.html
```

### 6. Commit

```bash
# Stage changes
git add .

# Commit with conventional commit message
git commit -m "feat: add new feature

- Implemented X
- Added tests for Y
- Updated documentation"
```

### 7. Push and Create PR

```bash
# Push to your fork
git push origin feature/my-feature

# Create PR on GitHub
```

## Coding Standards

### Rust Code Style

**Follow Rust 2024 edition best practices:**

```rust
// Use explicit types when not obvious
let count: usize = items.len();

// Prefer iterators over loops
let sum: i32 = numbers.iter().sum();

// Use Result for errors
pub fn process(input: &str) -> Result<String, Error> {
    // Implementation
}

// Document public APIs
/// Processes the input and returns a result.
///
/// # Arguments
///
/// * `input` - The input string to process
///
/// # Examples
///
/// ```
/// use lexum::process;
/// let result = process("hello").unwrap();
/// assert_eq!(result, "HELLO");
/// ```
pub fn process(input: &str) -> Result<String, Error> {
    Ok(input.to_uppercase())
}

// Use strong typing
pub struct DocumentId(String);
pub struct Score(f32);

// Prefer owned types in APIs
pub fn index_document(doc: Document) -> Result<DocumentId, Error> {
    // Implementation
}
```

### Error Handling

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LexumError {
    #[error("Index not found: {0}")]
    IndexNotFound(String),
    
    #[error("Query parsing error: {0}")]
    QueryParseError(String),
    
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, LexumError>;
```

### Async Code

```rust
use tokio::time::{timeout, Duration};

// Always set timeouts for network operations
pub async fn fetch_data(url: &str) -> Result<Data> {
    let request = reqwest::get(url);
    
    let response = timeout(Duration::from_secs(30), request)
        .await
        .map_err(|_| LexumError::Timeout)?
        .map_err(LexumError::from)?;
    
    Ok(response.json().await?)
}

// Use spawn_blocking for CPU-intensive tasks
pub async fn process_large_file(path: &Path) -> Result<ProcessedData> {
    let path = path.to_owned();
    
    tokio::task::spawn_blocking(move || {
        // CPU-intensive processing
        process_file_sync(&path)
    })
    .await?
}
```

### Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let result = process("input");
        assert_eq!(result, "expected");
    }

    #[tokio::test]
    async fn test_async_operation() {
        let result = async_operation().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_error_handling() {
        let result = failing_operation();
        assert!(matches!(result, Err(LexumError::InvalidInput(_))));
    }
}
```

### Benchmarks

```rust
// benchmark/search_bench.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use lexum::search::search_index;

fn search_benchmark(c: &mut Criterion) {
    let index = setup_test_index();
    
    c.bench_function("search basic query", |b| {
        b.iter(|| {
            search_index(black_box(&index), black_box("query"))
        })
    });
}

criterion_group!(benches, search_benchmark);
criterion_main!(benches);
```

## Hot Reload Development

```bash
# Watch for changes and rebuild
cargo watch -x 'run -- serve --config config.example.yml'

# Run tests on change
cargo watch -x test

# Run specific command
cargo watch -x 'clippy --all-targets'
```

## Debugging

### VS Code

`.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Lexum Server",
      "cargo": {
        "args": [
          "build",
          "--bin=lexum-server",
          "--package=lexum-server"
        ],
        "filter": {
          "name": "lexum-server",
          "kind": "bin"
        }
      },
      "args": ["serve", "--config", "config.example.yml"],
      "cwd": "${workspaceFolder}"
    },
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug Tests",
      "cargo": {
        "args": [
          "test",
          "--no-run",
          "--lib",
          "--package=lexum-core"
        ],
        "filter": {
          "name": "lexum-core",
          "kind": "lib"
        }
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Logging

```rust
use tracing::{info, debug, warn, error, instrument};

#[instrument(skip(index))]
pub async fn search(index: &Index, query: &str) -> Result<SearchResults> {
    debug!("Starting search with query: {}", query);
    
    let start = Instant::now();
    let results = perform_search(index, query).await?;
    
    info!(
        duration_ms = start.elapsed().as_millis(),
        hits = results.total,
        "Search completed"
    );
    
    Ok(results)
}

// Set log level
RUST_LOG=lexum=debug cargo run
```

## Performance Profiling

### CPU Profiling

```bash
# Install flamegraph
cargo install flamegraph

# Profile
cargo flamegraph --bin lexum-server

# View
open flamegraph.svg
```

### Memory Profiling

```bash
# With valgrind
valgrind --tool=massif target/release/lexum-server

# Analyze
ms_print massif.out.*
```

### Benchmarking

```bash
# Run benchmarks
cargo bench

# Specific benchmark
cargo bench search

# Save baseline
cargo bench --bench search_bench -- --save-baseline main

# Compare
git checkout feature-branch
cargo bench --bench search_bench -- --baseline main
```

## Docker Development

```bash
# Build image
docker build -t lexum-dev -f Dockerfile.dev .

# Run container
docker run -it --rm \
  -v $(pwd):/workspace \
  -w /workspace \
  -p 9200:9200 \
  lexum-dev

# Docker Compose
docker-compose -f docker-compose.dev.yml up
```

## Integration Tests

```rust
// tests/integration_test.rs
use lexum::prelude::*;

#[tokio::test]
async fn test_full_workflow() {
    // Start test server
    let server = TestServer::start().await;
    
    // Create index
    let index = server.create_index("test_index").await.unwrap();
    
    // Index documents
    let doc_id = server.index_document(&index, json!({
        "title": "Test Document",
        "content": "This is a test"
    })).await.unwrap();
    
    // Search
    let results = server.search(&index, "test").await.unwrap();
    assert_eq!(results.total, 1);
    assert_eq!(results.hits[0].id, doc_id);
    
    // Cleanup
    server.delete_index(&index).await.unwrap();
    server.shutdown().await;
}
```

## GUI Development

```bash
cd lexum-gui

# Install dependencies
npm install

# Development mode
npm run dev

# Build
npm run build

# Run tests
npm test

# Lint
npm run lint
```

## Documentation

### Generate Docs

```bash
# Generate API docs
cargo doc --no-deps --all-features

# Open in browser
cargo doc --no-deps --all-features --open
```

### Doc Tests

```rust
/// Searches the index with the given query.
///
/// # Examples
///
/// ```
/// use lexum::search;
/// 
/// let index = setup_test_index();
/// let results = search(&index, "rust").unwrap();
/// assert!(results.total > 0);
/// ```
pub fn search(index: &Index, query: &str) -> Result<SearchResults> {
    // Implementation
}
```

## Dependency Management

### Update Dependencies

```bash
# Check for outdated dependencies
cargo outdated

# Update Cargo.lock
cargo update

# Update specific dependency
cargo update -p tokio
```

### Audit Dependencies

```bash
# Install cargo-audit
cargo install cargo-audit

# Run audit
cargo audit

# Fix vulnerabilities
cargo audit fix
```

## Release Process

### 1. Update Version

```toml
# Cargo.toml
[package]
version = "0.2.0"
```

### 2. Update CHANGELOG

```markdown
## [0.2.0] - 2024-10-25

### Added
- New feature X
- Support for Y

### Fixed
- Bug in Z

### Changed
- Improved performance of A
```

### 3. Run Quality Checks

```bash
# Format
cargo +nightly fmt --all

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Test
cargo test --workspace --all-features

# Build
cargo build --release

# Documentation
cargo doc --no-deps --all-features

# Audit
cargo audit
```

### 4. Create Tag

```bash
# Commit changes
git add .
git commit -m "chore: release version 0.2.0"

# Create annotated tag
git tag -a v0.2.0 -m "Release version 0.2.0

Features:
- Feature X
- Feature Y

Fixes:
- Bug Z"
```

### 5. Build Release Artifacts

```bash
# Build for multiple targets
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-apple-darwin
cargo build --release --target x86_64-pc-windows-msvc

# Create archives
tar -czf lexum-v0.2.0-linux-x86_64.tar.gz -C target/x86_64-unknown-linux-gnu/release lexum
tar -czf lexum-v0.2.0-darwin-x86_64.tar.gz -C target/x86_64-apple-darwin/release lexum
zip lexum-v0.2.0-windows-x86_64.zip target/x86_64-pc-windows-msvc/release/lexum.exe
```

### 6. Publish

```bash
# Dry run
cargo publish --dry-run

# Publish to crates.io
cargo publish
```

## Troubleshooting

### Common Issues

**Build fails with "could not compile":**
```bash
# Clean and rebuild
cargo clean
cargo build
```

**Tests fail intermittently:**
```bash
# Run tests serially
cargo test -- --test-threads=1
```

**Clippy warnings:**
```bash
# Fix automatically when possible
cargo clippy --fix --workspace --all-targets
```

## Environment Variables

```bash
# Rust flags
export RUSTFLAGS="-C target-cpu=native"

# Log level
export RUST_LOG=lexum=debug,tantivy=info

# Backtrace
export RUST_BACKTRACE=1

# Build threads
export CARGO_BUILD_JOBS=8
```

## Tools

### Recommended Tools

```bash
# Code formatting
cargo install cargo-fmt

# Linting
cargo install cargo-clippy

# Testing
cargo install cargo-nextest

# Coverage
cargo install cargo-llvm-cov

# Benchmarking
cargo install cargo-criterion

# Watch for changes
cargo install cargo-watch

# Dependency management
cargo install cargo-outdated cargo-audit cargo-edit

# Profiling
cargo install flamegraph

# Cross-compilation
cargo install cross
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Update documentation
6. Run quality checks
7. Submit a pull request

See [CONTRIBUTING.md](../CONTRIBUTING.md) for detailed guidelines.

## Code Review Checklist

- [ ] Code follows Rust style guidelines
- [ ] All tests pass
- [ ] New tests added for new features
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] No clippy warnings
- [ ] Code formatted with rustfmt
- [ ] No unwrap() or expect() without justification
- [ ] Error handling is appropriate
- [ ] Performance considered
- [ ] Security implications reviewed

## Resources

- [Rust Documentation](https://doc.rust-lang.org/)
- [Tokio Documentation](https://tokio.rs)
- [Tantivy Documentation](https://docs.rs/tantivy)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Rust Performance Book](https://nnethercote.github.io/perf-book/)

## See Also

- [Architecture](./ARCHITECTURE.md)
- [API Reference](./API_REFERENCE.md)
- [CI/CD](./CI_CD.md)

