# Multi-stage Dockerfile for Lexum Search Engine Server
#
# USAGE:
#   Build image:
#     docker build -t lexum:latest .
#     docker build -t lexum:0.1.0-alpha --build-arg VERSION=0.1.0-alpha .
#
#   Run container:
#     docker run -d -p 9200:9200 \
#       -v lexum-data:/data \
#       -v lexum-snapshots:/snapshots \
#       -e LEXUM_NETWORK_HOST=0.0.0.0 \
#       -e LEXUM_NETWORK_HTTP_PORT=9200 \
#       --name lexum-server \
#       lexum:latest
#
#   Run with custom config:
#     docker run -d -p 9200:9200 \
#       -v lexum-data:/data \
#       -v lexum-snapshots:/snapshots \
#       -v /path/to/config.yml:/app/config.yml:ro \
#       -e LEXUM_CONFIG_FILE=/app/config.yml \
#       --name lexum-server \
#       lexum:latest
#
#   Access logs:
#     docker logs lexum-server
#     docker logs -f lexum-server  # follow logs
#
#   Health check:
#     curl http://localhost:9200/_cluster/health
#
#   Stop container:
#     docker stop lexum-server
#     docker rm lexum-server
#
#   Remove volumes:
#     docker volume rm lexum-data lexum-snapshots

# Stage 1: Build stage
FROM rust:1.85-slim AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy workspace members
COPY lexum-core ./lexum-core
COPY lexum-macros ./lexum-macros
COPY lexum-server ./lexum-server

# Build arguments
# VERSION: Application version (default: 0.1.0-alpha)
# BUILD_DATE: Build timestamp (optional, for labels)
# GIT_COMMIT: Git commit hash (optional, for labels)
ARG VERSION=0.1.0-alpha
ARG BUILD_DATE
ARG GIT_COMMIT

# Build the application in release mode
# Uses BuildKit cache mounts for faster subsequent builds:
# - Cargo registry cache: speeds up dependency downloads
# - Target directory cache: speeds up compilation
# To enable BuildKit: export DOCKER_BUILDKIT=1
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/build/target \
    cargo build --release --bin lexum-server && \
    cp target/release/lexum-server /lexum-server

# Stage 2: Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
# - ca-certificates: for HTTPS/TLS connections
# - libssl3: SSL/TLS library required by Rust binaries
# - curl: for health check endpoint
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
# Running as non-root reduces attack surface
RUN groupadd -r lexum && useradd -r -g lexum -u 1000 lexum

# Create data directories with proper permissions
# /data: index data storage (should be mounted as volume)
# /snapshots: snapshot storage (should be mounted as volume)
RUN mkdir -p /data /snapshots && \
    chown -R lexum:lexum /data /snapshots

# Set working directory
WORKDIR /app

# Copy binary from builder stage
COPY --from=builder /lexum-server /app/lexum-server

# Copy example config file (can be overridden via volume mount)
COPY config.example.yml /app/config.example.yml

# Set ownership to non-root user
RUN chown -R lexum:lexum /app

# Switch to non-root user for security
USER lexum

# Expose default HTTP port
# Override with -p flag: docker run -p 8080:9200 ...
EXPOSE 9200

# Health check configuration
# Checks /_cluster/health endpoint every 30 seconds
# Container starts as "starting" for 40 seconds before health checks begin
# After 3 consecutive failures, container is marked unhealthy
HEALTHCHECK --interval=30s --timeout=10s --start-period=40s --retries=3 \
    CMD curl -f http://localhost:9200/_cluster/health || exit 1

# Default command
# Server will bind to 0.0.0.0:9200 by default (configurable via env vars)
# Environment variables:
#   LEXUM_NETWORK_HOST: bind address (default: 0.0.0.0)
#   LEXUM_NETWORK_HTTP_PORT: HTTP port (default: 9200)
#   LEXUM_DATA_DIR: data directory path (default: /data)
#   LEXUM_CONFIG_FILE: path to config file (optional)
#   RUST_LOG: logging level (trace, debug, info, warn, error)
CMD ["/app/lexum-server"]

# OCI labels for image metadata
# These labels follow the OCI Image Specification
# Useful for image management and tooling
LABEL org.opencontainers.image.title="Lexum Search Engine" \
      org.opencontainers.image.description="High-performance Elasticsearch-compatible search engine" \
      org.opencontainers.image.version="${VERSION}" \
      org.opencontainers.image.created="${BUILD_DATE}" \
      org.opencontainers.image.revision="${GIT_COMMIT}" \
      org.opencontainers.image.source="https://github.com/hivellm/lexum"

