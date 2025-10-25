## Why

Developers need official SDKs in multiple languages to integrate Lexum into their applications. SDKs provide idiomatic, type-safe clients that handle connection management, retries, and error handling.

## What Changes

- Create Rust SDK (native client library)
- Develop Python SDK with async support
- Build JavaScript/TypeScript SDK for Node.js and browsers
- Create Go SDK
- Implement Java SDK
- Add connection pooling in all SDKs
- Implement automatic retry with exponential backoff
- Add comprehensive documentation and examples for each SDK

## Impact

- Affected specs: `sdk-rust`, `sdk-python`, `sdk-javascript`, `sdk-go`, `sdk-java`
- Affected code: Creates `sdks/` directory:
  - `rust/` - Rust client library
  - `python/` - Python package
  - `javascript/` - NPM package
  - `go/` - Go module
  - `java/` - Maven package
- Each SDK must support all API operations
- Must follow language-specific best practices

