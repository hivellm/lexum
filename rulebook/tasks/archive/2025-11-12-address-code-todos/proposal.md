## Why

There are several TODO comments throughout the codebase that represent incomplete work, known issues, or planned improvements. These TODOs span multiple areas including test fixes, performance enhancements, Tantivy compatibility issues, and feature improvements. Addressing these TODOs will improve code quality, test reliability, and system capabilities.

## What Changes

- Fix Tantivy compatibility issues in integration tests (4 occurrences)
- Fix hanging test in progress tracker
- Re-enable template tests with updated mockito API
- Implement memory profiling in load tests
- Implement CPU profiling in load tests
- Implement throughput tracking over time
- Implement response time distribution tracking
- Implement efficient Tantivy-based sorting
- Support regex patterns in index template matching

## Impact

- Affected specs: `testing-framework`, `performance`, `core-search`, `index-templates`
- Affected code:
  - `lexum-cli/tests/integration_test.rs` - Fix Tantivy errors
  - `lexum-core/src/progress/tracker.rs` - Fix hanging test
  - `lexum-cli/src/commands/template.rs` - Re-enable tests
  - `lexum-server/src/http_load_test.rs` - Add profiling capabilities
  - `lexum-core/src/search/executor.rs` - Improve sorting performance
  - `lexum-core/src/index/template.rs` - Add regex pattern support
- Dependencies: May require updates to mockito, Tantivy, or profiling libraries
- Testing: All fixes must include tests to prevent regressions

