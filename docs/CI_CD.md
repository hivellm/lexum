# CI/CD Pipeline

Continuous Integration and Continuous Deployment setup for Lexum.

## Overview

Lexum uses GitHub Actions for automated building, testing, and deployment.

**Pipeline Stages:**
1. **Lint**: Code formatting and linting
2. **Test**: Unit and integration tests
3. **Security**: Dependency audit and security checks
4. **Build**: Multi-platform builds
5. **Package**: Create release artifacts
6. **Publish**: Publish to registries
7. **Deploy**: Deploy to production

## GitHub Actions Workflows

### Workflow Structure

```
.github/
└── workflows/
    ├── ci.yml                 # Main CI pipeline
    ├── release.yml            # Release automation
    ├── docker.yml             # Docker image builds
    ├── security.yml           # Security scans
    └── deploy.yml             # Deployment workflows
```

## Main CI Pipeline

### `.github/workflows/ci.yml`

```yaml
name: CI

on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main, develop]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  # Format check
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust nightly
        uses: dtolnay/rust-toolchain@nightly
        with:
          components: rustfmt
      
      - name: Check formatting
        run: cargo +nightly fmt --all -- --check

  # Linting
  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
        with:
          components: clippy
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Run clippy
        run: |
          cargo clippy --workspace --all-targets --all-features -- -D warnings

  # Tests
  test:
    name: Test
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        rust: [nightly]
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Install nextest
        uses: taiki-e/install-action@nextest
      
      - name: Run tests
        run: cargo nextest run --workspace --all-features
      
      - name: Run doc tests
        run: cargo test --doc --workspace

  # Coverage
  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
        with:
          components: llvm-tools-preview
      
      - name: Install cargo-llvm-cov
        uses: taiki-e/install-action@cargo-llvm-cov
      
      - name: Generate coverage
        run: |
          cargo llvm-cov --all-features --workspace --lcov --output-path lcov.info
      
      - name: Upload to Codecov
        uses: codecov/codecov-action@v3
        with:
          files: lcov.info
          fail_ci_if_error: true

  # Build
  build:
    name: Build
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
          - os: windows-latest
            target: x86_64-pc-windows-msvc
          - os: macos-latest
            target: x86_64-apple-darwin
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
        with:
          targets: ${{ matrix.target }}
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Build
        run: cargo build --release --target ${{ matrix.target }}
      
      - name: Upload artifact
        uses: actions/upload-artifact@v3
        with:
          name: lexum-${{ matrix.target }}
          path: |
            target/${{ matrix.target }}/release/lexum${{ matrix.os == 'windows-latest' && '.exe' || '' }}
```

## Release Workflow

### `.github/workflows/release.yml`

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

permissions:
  contents: write

jobs:
  # Build release artifacts
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            name: linux-x86_64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            name: linux-aarch64
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            name: windows-x86_64
          - os: macos-latest
            target: x86_64-apple-darwin
            name: darwin-x86_64
          - os: macos-latest
            target: aarch64-apple-darwin
            name: darwin-aarch64
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
        with:
          targets: ${{ matrix.target }}
      
      - name: Install cross (for cross-compilation)
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: cargo install cross
      
      - name: Build
        run: |
          if [ "${{ matrix.target }}" = "aarch64-unknown-linux-gnu" ]; then
            cross build --release --target ${{ matrix.target }}
          else
            cargo build --release --target ${{ matrix.target }}
          fi
        shell: bash
      
      - name: Create archive
        shell: bash
        run: |
          VERSION=${GITHUB_REF#refs/tags/v}
          if [ "${{ matrix.os }}" = "windows-latest" ]; then
            ARCHIVE="lexum-v$VERSION-${{ matrix.name }}.zip"
            7z a $ARCHIVE target/${{ matrix.target }}/release/lexum.exe
          else
            ARCHIVE="lexum-v$VERSION-${{ matrix.name }}.tar.gz"
            tar -czf $ARCHIVE -C target/${{ matrix.target }}/release lexum
          fi
          echo "ARCHIVE=$ARCHIVE" >> $GITHUB_ENV
      
      - name: Upload release asset
        uses: softprops/action-gh-release@v1
        with:
          files: ${{ env.ARCHIVE }}
          draft: false
          prerelease: false

  # Publish to crates.io
  publish-crates:
    name: Publish to crates.io
    runs-on: ubuntu-latest
    needs: [build]
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Publish
        run: cargo publish --token ${{ secrets.CARGO_TOKEN }}

  # Build and push Docker images
  publish-docker:
    name: Publish Docker images
    runs-on: ubuntu-latest
    needs: [build]
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      
      - name: Login to Docker Hub
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKER_USERNAME }}
          password: ${{ secrets.DOCKER_PASSWORD }}
      
      - name: Extract version
        id: version
        run: echo "VERSION=${GITHUB_REF#refs/tags/v}" >> $GITHUB_OUTPUT
      
      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: |
            lexum/lexum:latest
            lexum/lexum:${{ steps.version.outputs.VERSION }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

## Docker Build Workflow

### `.github/workflows/docker.yml`

```yaml
name: Docker

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  build:
    name: Build Docker image
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3
      
      - name: Build
        uses: docker/build-push-action@v5
        with:
          context: .
          push: false
          tags: lexum/lexum:test
          cache-from: type=gha
          cache-to: type=gha,mode=max
      
      - name: Test image
        run: |
          docker run --rm -d --name lexum-test lexum/lexum:test
          sleep 10
          curl -f http://localhost:9200/_health || exit 1
          docker stop lexum-test
```

## Security Workflow

### `.github/workflows/security.yml`

```yaml
name: Security

on:
  schedule:
    - cron: '0 0 * * *'  # Daily at midnight
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  # Dependency audit
  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Install cargo-audit
        run: cargo install cargo-audit
      
      - name: Run audit
        run: cargo audit --deny warnings

  # Dependency review (PRs only)
  dependency-review:
    name: Dependency Review
    runs-on: ubuntu-latest
    if: github.event_name == 'pull_request'
    steps:
      - uses: actions/checkout@v4
      
      - name: Dependency Review
        uses: actions/dependency-review-action@v3

  # CodeQL analysis
  codeql:
    name: CodeQL Analysis
    runs-on: ubuntu-latest
    permissions:
      security-events: write
    steps:
      - uses: actions/checkout@v4
      
      - name: Initialize CodeQL
        uses: github/codeql-action/init@v2
        with:
          languages: 'rust'
      
      - name: Build
        run: cargo build
      
      - name: Perform CodeQL Analysis
        uses: github/codeql-action/analyze@v2
```

## Deployment Workflow

### `.github/workflows/deploy.yml`

```yaml
name: Deploy

on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Environment to deploy'
        required: true
        type: choice
        options:
          - staging
          - production

jobs:
  deploy:
    name: Deploy to ${{ github.event.inputs.environment }}
    runs-on: ubuntu-latest
    environment: ${{ github.event.inputs.environment }}
    steps:
      - uses: actions/checkout@v4
      
      - name: Configure kubectl
        uses: azure/setup-kubectl@v3
      
      - name: Set kubeconfig
        run: |
          echo "${{ secrets.KUBECONFIG }}" | base64 -d > kubeconfig
          export KUBECONFIG=kubeconfig
      
      - name: Deploy with Helm
        run: |
          helm upgrade --install lexum ./helm/lexum \
            --namespace lexum-${{ github.event.inputs.environment }} \
            --create-namespace \
            --values values.${{ github.event.inputs.environment }}.yml \
            --wait
      
      - name: Verify deployment
        run: |
          kubectl rollout status deployment/lexum \
            -n lexum-${{ github.event.inputs.environment }}
```

## Benchmarking Workflow

### `.github/workflows/bench.yml`

```yaml
name: Benchmark

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  benchmark:
    name: Run benchmarks
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      
      - name: Install Rust
        uses: dtolnay/rust-toolchain@nightly
      
      - name: Cache dependencies
        uses: Swatinem/rust-cache@v2
      
      - name: Run benchmarks
        run: cargo bench --workspace -- --output-format bencher | tee output.txt
      
      - name: Store benchmark result
        uses: benchmark-action/github-action-benchmark@v1
        with:
          tool: 'cargo'
          output-file-path: output.txt
          github-token: ${{ secrets.GITHUB_TOKEN }}
          auto-push: true
```

## Status Badges

Add to README.md:

```markdown
[![CI](https://github.com/your-org/lexum/workflows/CI/badge.svg)](https://github.com/your-org/lexum/actions/workflows/ci.yml)
[![Security](https://github.com/your-org/lexum/workflows/Security/badge.svg)](https://github.com/your-org/lexum/actions/workflows/security.yml)
[![codecov](https://codecov.io/gh/your-org/lexum/branch/main/graph/badge.svg)](https://codecov.io/gh/your-org/lexum)
[![Crates.io](https://img.shields.io/crates/v/lexum.svg)](https://crates.io/crates/lexum)
[![Docker](https://img.shields.io/docker/v/lexum/lexum?label=docker)](https://hub.docker.com/r/lexum/lexum)
```

## Secrets Configuration

Required GitHub secrets:

```
CARGO_TOKEN              # crates.io API token
DOCKER_USERNAME          # Docker Hub username
DOCKER_PASSWORD          # Docker Hub password/token
KUBECONFIG               # Base64 encoded kubeconfig
CODECOV_TOKEN           # Codecov.io token (optional)
```

Add secrets in GitHub repository:
Settings → Secrets and variables → Actions → New repository secret

## Local CI Testing

### Act (GitHub Actions locally)

```bash
# Install act
brew install act  # macOS
# or
curl https://raw.githubusercontent.com/nektos/act/master/install.sh | sudo bash

# Run CI locally
act -j test

# Run specific workflow
act -W .github/workflows/ci.yml

# Run with secrets
act -s CARGO_TOKEN=your-token
```

## Pre-commit Hooks

### Install pre-commit

```bash
# .git/hooks/pre-commit
#!/bin/bash
set -e

echo "Running pre-commit checks..."

# Format check
cargo +nightly fmt --all -- --check

# Clippy
cargo clippy --workspace --all-targets -- -D warnings

# Tests
cargo test --workspace

echo "All checks passed!"
```

```bash
chmod +x .git/hooks/pre-commit
```

## Continuous Deployment

### GitOps with ArgoCD

```yaml
# argocd/application.yml
apiVersion: argoproj.io/v1alpha1
kind: Application
metadata:
  name: lexum
  namespace: argocd
spec:
  project: default
  source:
    repoURL: https://github.com/your-org/lexum
    targetRevision: HEAD
    path: helm/lexum
    helm:
      valueFiles:
        - values.production.yml
  destination:
    server: https://kubernetes.default.svc
    namespace: lexum
  syncPolicy:
    automated:
      prune: true
      selfHeal: true
```

## Monitoring CI/CD

### Prometheus Metrics

```yaml
# .github/workflows/metrics.yml
name: Metrics

on:
  workflow_run:
    workflows: ["CI"]
    types: [completed]

jobs:
  metrics:
    runs-on: ubuntu-latest
    steps:
      - name: Report metrics
        run: |
          curl -X POST https://metrics.example.com/api/v1/ci \
            -H "Content-Type: application/json" \
            -d '{
              "workflow": "${{ github.workflow }}",
              "status": "${{ github.event.workflow_run.conclusion }}",
              "duration": "${{ github.event.workflow_run.duration }}",
              "timestamp": "${{ github.event.workflow_run.created_at }}"
            }'
```

## Best Practices

1. **Cache Dependencies**: Use `Swatinem/rust-cache` to speed up builds
2. **Matrix Builds**: Test on multiple platforms
3. **Fail Fast**: Set `fail-fast: false` to see all test results
4. **Secrets Management**: Use GitHub secrets, never commit credentials
5. **Branch Protection**: Require CI to pass before merging
6. **Version Tagging**: Use semantic versioning
7. **Automated Testing**: Test every commit
8. **Security Scanning**: Run regular security audits
9. **Documentation**: Keep CI/CD configuration documented
10. **Monitoring**: Track CI/CD metrics

## Troubleshooting

### CI Failures

**Tests fail on specific platform:**
```yaml
# Add platform-specific exclusions
- name: Run tests
  if: matrix.os != 'windows-latest'
  run: cargo test
```

**Slow builds:**
```yaml
# Increase cache effectiveness
- uses: Swatinem/rust-cache@v2
  with:
    shared-key: "ci-cache"
```

**Flaky tests:**
```yaml
# Retry failed tests
- name: Run tests
  uses: nick-invision/retry@v2
  with:
    timeout_minutes: 30
    max_attempts: 3
    command: cargo nextest run
```

## Release Checklist

- [ ] All CI checks pass
- [ ] Version bumped in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] Documentation updated
- [ ] Security audit clean
- [ ] Benchmarks run
- [ ] Tag created
- [ ] Release notes written
- [ ] Docker images built
- [ ] Crates published
- [ ] Deployment verified

## See Also

- [Development](./DEVELOPMENT.md)
- [Deployment](./DEPLOYMENT.md)
- [Architecture](./ARCHITECTURE.md)

