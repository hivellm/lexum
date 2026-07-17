# SPEC-016 — Testing & Conformance

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Cross-cutting · verification contract for SPEC-002…SPEC-013; fixture harness from phase3_bulk-and-document-crud (success criteria) ([tasks](../../.rulebook/tasks/phase3_bulk-and-document-crud/proposal.md)) |
| **Planning source** | [Elastic plan F-054 (fault-injection as first-class deliverable), F-055](../analysis/elastic/08-execution-plan.md) · [Meilisearch plan R-02 (uniform errors), A-06](../analysis/meilisearch/08-execution-plan.md) · phase3 wire-fidelity fixture gate · `.rulebook/rulebook.json` (`coverageThreshold: 95`) · [docs/development/WSL_TANTIVY_CONFLICT.md](../development/WSL_TANTIVY_CONFLICT.md) |

Requirement IDs `TST-xxx`. RFC 2119 keywords are normative. This spec defines *how* every other spec's acceptance criteria are verified: the test tiers, the ES-fixture fidelity harness, the error-contract walk (SPEC-003), CI gates, benchmark policy, and the future fault-injection harness whose invariants live in SPEC-011.

## 1. Test tiers

- **TST-001** The suite is organized in four tiers; every tier MUST run in CI (stress MAY be a scheduled/nightly job rather than per-PR):

| Tier | Location | Scope | Budget |
|---|---|---|---|
| Unit | `#[cfg(test)] mod tests` in-file (per `.claude/rules/rust.md`) | One function/type; no I/O beyond tempdirs | ms each |
| Integration | `crates/*/tests/` and workspace `tests/*.rs` (e.g. `integration_test.rs`, `snapshot_restore_workflow_tests.rs`), one file per public behavior | One subsystem against a real Tantivy index / running router | < 10 s each |
| E2E | `tests/e2e/` | Full server over HTTP: real process, real ports, multi-request workflows | < 60 s each |
| Stress | `tests/stress/` | Concurrency, volume, and soak (100+ concurrent writers, 100k+ docs) | scheduled |

- **TST-002** Every normative requirement ID in SPEC-002…SPEC-013 MUST be covered by at least one test that names the requirement ID in its test name or a comment, so conformance is greppable (`rg "SET-041" crates tests`).
- **TST-003** Tests MUST run with `cargo test --all-features` as the baseline. New behavior lands with its tests in the same change (rulebook tail policy); a spec acceptance criterion without a corresponding test is an open conformance gap, not a done item.
- **TST-004** Tests MUST be hermetic: temp directories per test, no shared global state, no ordering dependencies (`--test-threads=1` MUST NOT be required for correctness), no reliance on external network services — webhook/receiver tests use local listeners.

## 2. Platform constraint — the WSL prohibition

- **TST-010** All test runs, benchmarks, and CI-reproduction runs on developer machines MUST execute on **native Windows** (or native Linux in CI), never under WSL with Windows-mounted paths. Tantivy's `mmap`/locking/fsync usage fails under WSL's 9p translation layer with `Invalid argument (os error 22)` — see [WSL_TANTIVY_CONFLICT.md](../development/WSL_TANTIVY_CONFLICT.md). A test failure reproduced only under WSL is an environment artifact, not a bug report.
- **TST-011** CI MUST include a native `windows-latest` job for the integration tiers (the existing `test-routes.yml` matrix already does; this is normative, not incidental).

## 3. ES-fixture fidelity harness

The compatibility promise ("drop-in for the 20% of the ES 7.10 API that 95% of clients use", F-047/F-055) is verified by replaying **recorded ES 7.10 exchanges**, not by hand-written assertions of what ES "probably" returns.

- **TST-020** The harness stores fixtures as request/response pairs recorded against a real Elasticsearch 7.10 instance: `tests/fixtures/es710/<area>/<case>.json` containing `{ "request": {method, path, headers, body}, "response": {status, body} }`. Fixtures are committed and never regenerated silently — regeneration is a reviewed change.
- **TST-021** Replay: each fixture's request is sent to a Lexum test server and the response compared per a **match policy** declared in the fixture: exact-value keys (e.g. `errors` flag, item order, `status` codes, `error.type` strings, bucket keys/doc_counts), shape-only keys (present with correct JSON type — e.g. `took`, `_seq_no`), and ignored keys (ES-internal noise). A fixture failure names the first diverging JSON pointer.
- **TST-022** Minimum fixture coverage: `_bulk` (success, per-item failure, create-conflict, update-missing-doc, malformed NDJSON line, mixed actions across indices — the phase3 gate), document CRUD + `_mget`, `_search` core DSL, `_mapping` GET/PUT + `_analyze` (SPEC-006), aggregations wire shapes (SPEC-007, behind the `es_aggregations_dsl` flag), and the ops surface (`_cluster/health`, `_cat/*`, `_stats`, `_nodes` — SPEC-008). Each new ES-compatible endpoint MUST land with fixtures.
- **TST-023** An ecosystem smoke gate MUST run a real ES client library (e.g. `elasticsearch-py` 7.x `helpers.bulk`, or an equivalent recorded client exchange) ingesting ≥ 100k documents against Lexum with zero client-side errors.

## 4. Error-contract walking test (SPEC-003 shape)

- **TST-030** A single integration test MUST walk **100% of registered routes** and provoke at least one error per route (unknown index, malformed body, wrong content type, missing auth as applicable), asserting every error response body is the SPEC-003 uniform error object — correct keys, a documented `code`, a `type` from the closed set, and a resolvable `link`.
- **TST-031** The walk MUST enumerate routes programmatically from the router/OpenAPI registration (not a hand-maintained list), so an endpoint added without error-contract coverage fails this test rather than being forgotten. Routes intentionally exempt (e.g. `/metrics` plain-text) live in an explicit, commented allowlist in the test.
- **TST-032** The walk additionally asserts every route is documented in `openapi.rs` (route ↔ OpenAPI parity in both directions).

## 5. CI gates

- **TST-040** **Route-coverage gate**: the route integration suite (`crates/lexum-server/tests/route_integration_test.rs`, wired via `.github/workflows/test-routes.yml`) MUST exercise every registered route at least once (success path) and MUST fail CI on any failure, on both `ubuntu-latest` and `windows-latest`. A route registered in `router.rs` with no test is a CI failure via TST-031's enumeration.
- **TST-041** **Coverage threshold**: line coverage (cargo-llvm-cov) MUST be ≥ **95%** per `.rulebook/rulebook.json` `coverageThreshold`. The gate applies to lexum-core and lexum-server library code; generated code and `main.rs` bootstrap MAY be excluded via explicit, committed exclusion rules.
- **TST-042** **Lint gates**: `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check` MUST pass on every PR (per `.claude/rules/rust.md` — warnings are errors).
- **TST-043** A `todo!()`/`unimplemented!()` reachable from any routed handler is a CI failure (the phase5 audit found runtime-reachable panics; the gate is a grep-plus-allowlist check over `crates/`).

## 6. Benchmark gates

- **TST-050** Benchmarks use **criterion** and live in `benchmark/` (workspace: `benchmark/search_benchmarks.rs`) and `lexum-core/benchmark/`. Each benchmark declares the fixture it runs against (doc count, schema) so numbers are comparable across runs.
- **TST-051** CI MUST run the benchmark suite against the base branch and fail on regression: > 10% slowdown on any gated benchmark's median, measured by criterion's built-in comparison on the same runner class. Gated benchmarks (minimum set): default search latency, `_bulk` ingest throughput (docs/s), aggregation over the 10k-doc fixture, snapshot/dump write throughput.
- **TST-052** Throughput acceptance gates from phases (e.g. phase3: `_bulk` at 5k docs/request sustains ≥ 10× the per-document baseline with ≤ 1 commit/request average) are expressed as criterion benchmarks with asserted floors, not one-off scripts.
- **TST-053** Benchmarks MUST run on native platforms only (TST-010) and MUST NOT run under coverage instrumentation.

## 7. Fault-injection harness (future — distribution)

Distribution is the largest engineering risk (F-054: "budget for correctness testing (Jepsen-style fault injection) as a first-class deliverable, not an afterthought"). The harness lands with SPEC-011 (distributed clustering); this section reserves its contract.

- **TST-060** The harness MUST drive a multi-node Lexum cluster through injected faults — process kill (-9), network partition, message delay/reorder, disk-full, clock skew — while a generator issues concurrent reads/writes, then check invariants against the recorded history.
- **TST-061** The **four distribution invariants** the harness verifies are normatively defined in **SPEC-011** — SPEC-011 owns their statement and semantics; this spec owns the harness mechanics and CI wiring. Any change to the invariants is a SPEC-011 change; the harness MUST reference them by their SPEC-011 requirement IDs.
- **TST-062** Harness runs are scheduled (nightly/weekly), seed-reproducible (a failing run's fault schedule replays from its seed), and produce a machine-readable history artifact for post-mortem checking.
- **TST-063** Single-node precursors MUST NOT wait for SPEC-011: kill-9 crash-recovery tests of the SPEC-002 task queue and dump/import atomicity (SPEC-013 LCM-054) belong to the integration tier now and are reused as harness checkers later.

## 8. Acceptance criteria

1. **Tier hygiene**: `cargo test --all-features` green on native Windows and Linux; no test requires `--test-threads=1`; e2e and stress tiers runnable by name (TST-001/004).
2. **Fixture harness**: the TST-022 fixture set replays green; a deliberately mutated response field makes the harness fail naming the JSON pointer (TST-021); the 100k-doc client smoke gate passes (TST-023).
3. **Error walk**: TST-030 walk covers 100% of routes (enumerated from the router), all error bodies match SPEC-003; adding an unregistered-in-OpenAPI route breaks TST-032.
4. **Gates fire**: a PR dropping coverage below 95%, introducing a clippy warning, regressing a gated benchmark > 10%, or adding a reachable `todo!()` fails CI (TST-040–043, TST-051).
5. **Requirement traceability**: for every requirement ID in SPEC-006/008/013, `rg` finds at least one referencing test (TST-002).
6. **Fault-injection readiness**: the kill-9 single-node precursors exist and pass (TST-063); the harness skeleton references SPEC-011 invariant IDs, not local copies (TST-061).
