## Why

Users need a command-line interface to manage Lexum servers, indices, and perform operations without writing code. The CLI provides essential operations for DevOps, testing, and administration.

## What Changes

- Create lexum-cli binary crate
- Implement server start/stop commands
- Add index management commands (create, delete, list, info)
- Implement document operations (index, get, delete)
- Add query execution command
- Implement configuration validation command
- Add interactive mode with readline
- Implement output formatting (JSON, table, pretty)

## Impact

- Affected specs: `cli-tool`
- Affected code: Creates `lexum-cli/` crate
- Dependencies: clap, tokio, serde_json, prettytable
- Provides user-friendly interface to all core functionality

