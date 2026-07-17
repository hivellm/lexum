## 1. Re-validate scope and pin the contract
- [ ] 1.1 Re-validate this phase against the actual shipped API (phases 1–8): confirm task-queue statuses/filters, multi-search shape (per-query errors, partial results), settings-resource keys, and the error object — update proposal/specs where reality diverged
- [ ] 1.2 Re-check the family convention against current `Nexus/sdks` and `Synap/sdks` (layout, package naming under `@hivehub/`/PyPI/crates.io, README matrix, cross-SDK test runner) and write the Lexum SDK contract spec (client shape, task-wait semantics, retry/error mapping, coverage matrix) into `specs/`
- [ ] 1.3 Verify the served OpenAPI document is complete and accurate for the surfaces the SDKs wrap; fix gaps in `crates/lexum-server` annotations; add the CI drift-check harness that SDK model tests will reuse

## 2. TypeScript/JavaScript SDK (`sdks/typescript`, `@hivehub/lexum-sdk`)
- [ ] 2.1 Scaffold package (ESM+CJS builds, typings, Node + browser fetch transport) with client construction, API-key auth, timeouts, retry with exponential backoff + jitter, uniform `LexumError` mapping `{message, code, type, link}`
- [ ] 2.2 Implement index management, document CRUD + bulk/NDJSON, and the settings resource (get/patch/reset)
- [ ] 2.3 Implement the task-queue surface: task handles on every mutating call, `getTask`/`getTasks` with filters, `waitForTask`/`waitForTasks` with timeout/interval, cancel/delete where supported
- [ ] 2.4 Implement search: simple search params, LQL passthrough, response shaping (highlight/crop/retrieve, both pagination styles), and multi-search/federation with per-query error objects
- [ ] 2.5 Unit/mock suite + integration suite against a real `lexum-server`; README with install + quick-start

## 3. Python SDK (`sdks/python`, `lexum-sdk` on PyPI)
- [ ] 3.1 Scaffold package (Python 3.11+, `httpx` async client + sync facade, typed models) with the same construction/auth/retry/error contract as §2.1
- [ ] 3.2 Implement index management, documents/bulk, settings resource
- [ ] 3.3 Implement task-queue surface incl. `wait_for_task(s)` helpers
- [ ] 3.4 Implement search, LQL passthrough, response shaping, multi-search
- [ ] 3.5 Unit/mock suite + integration suite against a real server; README with install + quick-start

## 4. Rust SDK (`sdks/rust`, `lexum-sdk` on crates.io)
- [ ] 4.1 Scaffold crate (reqwest-based, tokio async), sharing or mirroring `lexum-core` DTO types without pulling in the engine; same construction/auth/retry/error contract
- [ ] 4.2 Implement task queue (incl. `wait_for_task`), search + multi-search, index/settings/document management (minimum contract per proposal; full surface if cheap)
- [ ] 4.3 Unit + integration tests against a real server; README + docs.rs metadata

## 5. Family plumbing, CI, and publishing
- [ ] 5.1 Write `sdks/README.md` with the SDK/coverage matrix (family format) and note Go/Java/PHP/C# as deferred-on-demand
- [ ] 5.2 Add the cross-SDK integration test runner script at `sdks/` root (starts `lexum-server`, runs each SDK suite, prints a summary)
- [ ] 5.3 Wire GitHub Actions: per-SDK build+test jobs, OpenAPI drift check, and manual-trigger publish workflows (npm/PyPI/crates.io) held until 1.0

## 6. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 6.1 Update or create documentation covering the implementation
- [ ] 6.2 Write tests covering the new behavior
- [ ] 6.3 Run tests and confirm they pass
