# Contributing to Lexum

Thank you for your interest in contributing to Lexum! This document provides guidelines and instructions for contributing.

## Code of Conduct

This project adheres to a Code of Conduct. By participating, you are expected to uphold this code. Please report unacceptable behavior to conduct@lexum.io.

## How to Contribute

### Reporting Bugs

Before creating bug reports, please check the issue tracker to avoid duplicates. When creating a bug report, include:

- **Clear title and description**
- **Steps to reproduce**
- **Expected behavior**
- **Actual behavior**
- **Environment** (OS, Rust version, Lexum version)
- **Logs and error messages**
- **Minimal reproduction** (if possible)

### Suggesting Enhancements

Enhancement suggestions are tracked as GitHub issues. When creating an enhancement suggestion, include:

- **Clear title and description**
- **Use case and motivation**
- **Proposed solution**
- **Alternatives considered**
- **Additional context**

### Pull Requests

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Make your changes**
4. **Add tests** for new functionality
5. **Update documentation**
6. **Run quality checks** (format, lint, test)
7. **Commit your changes** with conventional commits
8. **Push to your fork**
9. **Open a Pull Request**

#### Pull Request Guidelines

- Follow the Rust style guidelines
- Write clear commit messages
- Include tests for new features
- Update documentation
- Ensure all tests pass
- Keep PRs focused and atomic
- Reference related issues

## Development Setup

See [DEVELOPMENT.md](docs/DEVELOPMENT.md) for detailed development setup instructions.

### Quick Setup

```bash
# Clone repository
git clone https://github.com/your-org/lexum
cd lexum

# Install Rust nightly
rustup install nightly
rustup default nightly

# Install tools
cargo install cargo-nextest cargo-llvm-cov cargo-watch

# Build
cargo build

# Run tests
cargo test
```

## Coding Standards

### Rust Style

- Use Rust 2024 edition
- Follow `rustfmt` formatting (run `cargo +nightly fmt`)
- Fix all `clippy` warnings (run `cargo clippy -- -D warnings`)
- Use meaningful variable names
- Add doc comments for public APIs
- Write idiomatic Rust code

### Testing

- Write tests for all new functionality
- Aim for >95% code coverage
- Include unit tests and integration tests
- Test error cases
- Use property-based testing where appropriate

### Documentation

- Document all public APIs with `///` comments
- Include examples in doc comments
- Update relevant documentation in `docs/`
- Add changelog entries

### Commits

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `style`: Code style (formatting, missing semicolons, etc.)
- `refactor`: Code refactoring
- `perf`: Performance improvement
- `test`: Adding tests
- `build`: Build system changes
- `ci`: CI/CD changes
- `chore`: Maintenance tasks

**Examples:**
```
feat(search): add fuzzy matching support

Implement fuzzy matching using Levenshtein distance with
configurable maximum edit distance.

Closes #123

fix(api): handle null response in search endpoint

- Add null check before deserializing
- Return proper error message
- Add test for null response case

Fixes #456
```

## Quality Checklist

Before submitting a PR, ensure:

- [ ] Code is formatted (`cargo +nightly fmt --all`)
- [ ] No clippy warnings (`cargo clippy --workspace -- -D warnings`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Coverage meets threshold (`cargo llvm-cov --all`)
- [ ] Documentation updated
- [ ] CHANGELOG.md updated
- [ ] Commit messages follow conventions
- [ ] PR description is clear and complete

## Project Structure

```
lexum/
├── lexum-core/          # Core library
├── lexum-server/        # Server binary
├── lexum-cli/           # CLI tool
├── lexum-gui/           # GUI application
├── tests/               # Integration tests
├── benchmark/           # Benchmarks
└── docs/                # Documentation
```

## Testing

### Run Tests

```bash
# All tests
cargo test --workspace

# Specific test
cargo test test_search

# With nextest (faster)
cargo nextest run --workspace

# Coverage
cargo llvm-cov --all --html
open target/llvm-cov/html/index.html
```

### Integration Tests

```bash
# Run integration tests
cargo test --test integration_test

# With specific feature
cargo test --test integration_test --features full
```

### Benchmarks

```bash
# Run all benchmarks
cargo bench

# Specific benchmark
cargo bench search

# Save baseline
cargo bench -- --save-baseline main
```

## Documentation

### Build Documentation

```bash
# Generate docs
cargo doc --no-deps --all-features

# Open in browser
cargo doc --no-deps --all-features --open
```

### Doc Tests

```bash
# Run doc tests
cargo test --doc --workspace
```

## Release Process

Maintainers only:

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Commit: `git commit -m "chore: release v0.2.0"`
4. Tag: `git tag -a v0.2.0 -m "Release v0.2.0"`
5. Push: `git push --tags`
6. GitHub Actions will build and publish

## Community

- **GitHub Discussions**: Ask questions and discuss ideas
- **Discord**: Join our Discord server for real-time chat
- **Twitter**: Follow [@LexumSearch](https://twitter.com/lexumsearch) for updates

## License

By contributing to Lexum, you agree that your contributions will be licensed under the Apache License 2.0.

## Questions?

Feel free to:
- Open an issue
- Ask in GitHub Discussions
- Join our Discord
- Email: dev@lexum.io

Thank you for contributing to Lexum! 🚀

