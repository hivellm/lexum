## Why

Lexum needs robust configuration management and structured logging to enable proper operation, debugging, and monitoring. Configuration must support multiple sources (YAML, env vars) and logging must be structured for easy parsing and analysis.

## What Changes

- Implement YAML configuration parsing
- Add environment variable support
- Implement configuration validation
- Add structured logging with tracing
- Implement log levels and filtering
- Add log output configuration (stdout, file)
- Implement correlation IDs for request tracking
- Add configuration hot-reload support

## Impact

- Affected specs: `configuration`, `logging`
- Affected code: Creates `lexum-core/src/config/` and `lexum-core/src/logging/`
- Dependencies: serde, serde_yaml, tracing, tracing-subscriber
- Must be implemented before all other components

