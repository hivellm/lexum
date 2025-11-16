# Testing Guide

## Running Tests

### Basic Tests (Default)

Run all tests excluding slow tests:

```bash
cargo test --package lexum-core --lib
```

### Slow Tests

Some tests are marked as "slow tests" because they:
- Process large datasets (1000+ documents)
- Perform complex merge operations
- Test performance characteristics

To run slow tests, enable the `slow-tests` feature:

```bash
# Run all tests including slow tests
cargo test --package lexum-core --lib --features slow-tests

# Run only slow tests
cargo test --package lexum-core --lib --features slow-tests -- --test-threads=1
```

### Server-to-Server Tests (s2s)

Tests that require a running server are marked with the `s2s` feature:

```bash
cargo test --package lexum-server --test protocol_tests --features s2s
```

## Test Categories

### Fast Tests (Default)
- Basic functionality tests
- Edge case tests
- Merge tests with small datasets (< 100 documents)
- Unit tests for individual components

### Slow Tests (`slow-tests` feature)
- Large dataset tests (1000+ documents)
- Very large dataset tests (10000+ documents)
- Performance tests
- Stress tests

### Integration Tests (`s2s` feature)
- Protocol tests (StreamableHTTP, MCP, UMICP)
- End-to-end server tests
- Network communication tests

## CI/CD Configuration

### Recommended CI Test Commands

```bash
# Fast tests (run in every commit/push) - DEFAULT
cargo test --workspace --no-default-features

# Slow tests (run in nightly builds or before releases)
cargo test --workspace --features slow-tests

# Full test suite including slow tests (run before releases)
cargo test --workspace --features slow-tests

# Server-to-server tests (run in integration test environment)
cargo test --workspace --features s2s
```

### Git Hooks Configuration

The Git hooks are configured to run fast tests only:

- **pre-commit**: Runs fast tests (`--no-default-features`)
- **pre-push**: Runs fast tests (`--no-default-features`)

To run slow tests before committing/pushing, manually run:
```bash
cargo test --workspace --features slow-tests
```

## Test Timeouts

Rust's test framework doesn't have built-in timeout support, but you can:

1. Use `--test-threads=1` to run tests sequentially and identify slow ones
2. Use external tools like `timeout` on Linux/Mac:
   ```bash
   timeout 30s cargo test --package lexum-core --lib
   ```
3. Use `cargo-nextest` which has built-in timeout support:
   ```bash
   cargo nextest run --test-timeout 30s
   ```

## Performance Testing

For performance benchmarks, use Criterion:

```bash
cargo bench --package lexum-core
```

## Test Organization

Tests are organized by module:
- `lexum-core/src/aggregation/*.rs` - Aggregation tests
- `lexum-server/tests/*.rs` - Integration tests
- `tests/*/` - End-to-end tests

## Best Practices

1. **Keep fast tests fast**: Tests should complete in < 1 second
2. **Mark slow tests**: Use `#[cfg(feature = "slow-tests")]` for tests > 1 second
3. **Use appropriate datasets**: Small datasets (< 100) for fast tests, large for slow tests
4. **Test in isolation**: Each test should be independent
5. **Document test purpose**: Add comments explaining what each test validates

