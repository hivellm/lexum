# Proposal: phase12_client-sdks

> **Re-validate scope when picked up — this phase was planned ahead of implementation.**
> The API surface this phase wraps (task queue, multi-search, settings resource,
> tenant tokens) is being built in phases 1–8; re-check endpoint shapes, the
> OpenAPI document, and the sibling SDK conventions before starting.

## Why

Lexum only speaks raw REST today. Every consumer (including the phase13 GUI and
MCP integrations) must hand-roll HTTP calls, task-polling loops, and error
handling. The HiveLLM family already has a proven SDK model — Nexus and Synap
each ship a `sdks/` tree of handwritten, idiomatic clients with a shared wire
contract, consistent client-construction shape, and per-language test suites —
and downstream family projects consume those SDKs directly (e.g. Vectorizer's
GUI depends on `@hivehub/vectorizer-sdk`). Lexum needs the same so that:

- async writes are usable without every caller reimplementing `taskUid`
  poll/wait logic (the phase1 task-queue model returns `taskUid` for every
  mutating call — an SDK without wait helpers makes that model painful);
- search, multi-search/federation, and index+settings management get type-safe,
  documented entry points;
- the phase13 GUI has a first-party TypeScript client to build on.

This supersedes the archived `2026-07-17-add-sdk-development` task, which
listed five languages flat with no priority; this re-scope orders them by
adoption impact and defers Go/Java/PHP/C# to demand.

## What Changes

1. **Create the `sdks/` tree following the family convention** (same layout and
   naming as `e:\HiveLLM\Nexus\sdks` / `e:\HiveLLM\Synap\sdks`): `sdks/README.md`
   with the SDK/coverage matrix, one folder per language, per-SDK README and
   tests folder, and a cross-SDK test runner script.
2. **Handwritten clients, OpenAPI as the contract check — per family
   convention.** Nexus and Synap SDKs are handwritten idiomatic clients, not
   generator output; Lexum follows suit. Lexum's OpenAPI document
   (lexum-server already serves it) is used as the source of truth for request/
   response types and as a CI drift check (SDK model definitions validated
   against the served spec), not as a code generator.
3. **TypeScript/JavaScript SDK first** — `sdks/typescript/`, npm package
   `@hivehub/lexum-sdk` (scoped like the siblings). Node + browser (fetch-based),
   ESM+CJS builds, full typings.
4. **Python SDK second** — `sdks/python/`, PyPI package `lexum-sdk`,
   async-first (`httpx`) with a sync facade, Python 3.11+.
5. **Rust native client third** — `sdks/rust/`, crates.io `lexum-sdk`,
   reusing `lexum-core` DTO types where the dependency does not drag the whole
   engine in (otherwise mirror the types).
6. **Go/Java/PHP/C# deferred** — folders are NOT created in this phase; add
   only on concrete demand, following the same shape.
7. **Common capability set (all shipped SDKs, feature-parity matrix in
   `sdks/README.md`):**
   - client construction from a base URL + API key (family shape:
     `LexumClient::new("http://host:port")` / `new LexumClient({ baseUrl })`);
   - **task-queue model**: every mutating call returns a task handle
     (`taskUid`); `waitForTask(uid, {timeout, interval})` and
     `waitForTasks([...])` helpers; task listing with filters
     (status/type/index) and cancel/delete where the server supports it;
   - **search**: simple search (`q` + `filter` + `sort` flat params), LQL
     query passthrough, response-shaping options (highlight/crop/retrieve,
     both pagination styles), and **multi-search/federation**
     (`POST /multi-search`, per-query error objects with partial results);
   - **index + settings management**: index CRUD, the phase4 settings
     resource (get/patch/reset per key), document CRUD + bulk/NDJSON;
   - uniform error type mapping the server error object
     (`{message, code, type, link}`), automatic retry with exponential
     backoff + jitter for idempotent calls, configurable timeouts.
8. **Per-SDK tests + cross-SDK integration matrix**: unit/mock suites per
   language plus an integration run against a real `lexum-server` (the family
   pattern: `run-all-comprehensive-tests` script at `sdks/` root).
9. **CI + publishing scaffolding**: GitHub Actions jobs per SDK (build, test,
   OpenAPI drift check); publish workflows wired but manual-trigger until 1.0.

## Impact

- Affected specs: `.rulebook/tasks/phase12_client-sdks/specs/` (SDK contract:
  client shape, task-wait semantics, error mapping, coverage matrix)
- Affected code: new `sdks/typescript/`, `sdks/python/`, `sdks/rust/`,
  `sdks/README.md`, cross-SDK test script; `.github/workflows/` (SDK CI);
  no changes to `crates/lexum-core` / `crates/lexum-server` beyond OpenAPI
  fixes discovered by the drift check
- Breaking change: NO (purely additive; server API untouched)
- User benefit: type-safe, idiomatic integration in the two highest-adoption
  ecosystems plus native Rust; task-queue ergonomics (`waitForTask`) instead
  of hand-rolled polling; a first-party client for the phase13 GUI

## Dependencies

- **phase1_write-path-task-queue** (hard): the `taskUid` contract and
  `GET /tasks` filters are the SDK's write-path surface.
- **phase3_bulk-and-document-crud**, **phase4_settings-mappings-analyze**
  (hard): document/bulk and settings-resource endpoints must be stable.
- **phase8_multisearch-federation** (soft): multi-search support can land in
  the SDKs behind a capability flag if phase8 is still in flight.
- **phase7_security-rbac-tenant-tokens** (soft): tenant-token minting helpers
  (backend-signed JWT search rules) are in scope only if phase7 has shipped.

## Success criteria

- `sdks/typescript` and `sdks/python` cover 100% of the stable REST surface
  (task queue, search, multi-search, documents/bulk, indexes, settings, keys);
  `sdks/rust` covers at minimum task queue + search + index/settings.
- `waitForTask` semantics verified by integration tests in every shipped SDK:
  an indexing task observed through `enqueued → processing → succeeded`, and
  timeout/failure paths covered.
- OpenAPI drift check green in CI: SDK request/response models validate
  against the spec served by `lexum-server`.
- Cross-SDK integration matrix passes against a real server started from the
  repo (script at `sdks/` root, runnable in CI and locally).
- `sdks/README.md` matrix accurately reflects per-SDK coverage, and each SDK
  README has installation + quick-start that works as written.
