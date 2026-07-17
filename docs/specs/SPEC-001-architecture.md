# SPEC-001 — System Architecture

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Cross-phase baseline — normative frame for `phase1_write-path-task-queue`, `phase3_bulk-and-document-crud`, and the phase9 distribution track |
| **Planning source** | [Meilisearch execution plan](../analysis/meilisearch/08-execution-plan.md) (R-01, R-02, R-07, A-01, A-04, A-06); [Elasticsearch execution plan](../analysis/elastic/08-execution-plan.md) (F-051, F-054, F-055); [.rulebook phase1 proposal](../../.rulebook/tasks/phase1_write-path-task-queue/proposal.md); [.rulebook phase3 proposal](../../.rulebook/tasks/phase3_bulk-and-document-crud/proposal.md) |

Requirement IDs `ARC-xxx`. RFC 2119 keywords are normative. Related specs: task queue = [SPEC-002](SPEC-002-write-path-task-queue.md), error contract = [SPEC-003](SPEC-003-error-contract.md), documents/bulk = [SPEC-005](SPEC-005-documents-and-bulk.md).

## 1. Scope and model

Lexum is a Rust full-text search engine built on Tantivy 0.25, exposed over an axum REST API plus MCP/UMICP protocol adapters, with LQL as the power-user query language. This spec defines the component model every other spec assumes: crate responsibilities, the layering, the write/read subsystems, the async model, the configuration model, and the target architecture after the re-planning (task queue as the single write door; federation as the query scatter-gather; WAL/replication under the distribution track).

- **ARC-001** The workspace MUST consist of exactly three published crates under `crates/`: `lexum-core` (engine), `lexum-server` (API + protocols), `lexum-macros` (procedural macros). New functionality MUST be placed by the responsibility rules of §2; new crates MAY be added only for the distribution layer (phase9).
- **ARC-002** `lexum-core` MUST NOT depend on `lexum-server`, axum, or any HTTP/protocol type. The engine is embeddable: everything reachable from `lexum_core::` MUST be usable without a running server.
- **ARC-003** `lexum-server` MUST NOT talk to Tantivy directly. All index, document, and search operations go through `lexum-core` public APIs. (Tantivy types MUST NOT appear in `lexum-server`'s public signatures.)
- **ARC-004** `lexum-macros` is build-time only (currently `tokio_test`, a timeout-enforcing `tokio::test` wrapper). It MUST NOT contain runtime logic.

## 2. Crate responsibilities

| Crate | Responsibility | Key modules (current) |
|---|---|---|
| `lexum-core` | Engine: index lifecycle, document storage, search execution, query model, aggregations, schema, snapshots, config | `index/` (manager, settings, template, alias, rollover, datastream), `document/` (store, multi_get, query_operations), `search/` (executor, multi_search, PIT, scroll, search_after, caches, highlighter, suggester), `query/` (LQL builder + types), `aggregation/`, `schema/`, `snapshot/`, `config.rs`, `error.rs`; **planned:** `tasks/` (queue, scheduler, batcher, persistence — SPEC-002) |
| `lexum-server` | Protocol surface: REST routing, protocol adapters, middleware, OpenAPI, error mapping | `router.rs`, `handlers/` (document, index, search, query_ops, bulk, mapping, snapshot, cluster, admin, …; **planned:** `tasks.rs`, `bulk_ndjson.rs`), `middleware/` (auth, rate_limit, content_type, request_size, query_complexity, ip_filter, metrics, …), `protocols/` (streamable_http, mcp/, umicp, detection), `error.rs`, `openapi.rs`, `services/` |
| `lexum-macros` | Proc macros for the workspace | `lib.rs` |

## 3. Layering

```
┌────────────────────────────────────────────────────────────────┐
│ Clients: REST / SDKs / ES shippers / MCP / UMICP / GUI         │
└──────────────────────────────┬─────────────────────────────────┘
                               ▼
┌────────────────────────────────────────────────────────────────┐
│ PROTOCOL LAYER              (lexum-server/src/protocols, bin)  │
│   HTTP(S) via axum · StreamableHTTP · MCP · UMICP · detection  │
└──────────────────────────────┬─────────────────────────────────┘
                               ▼
┌────────────────────────────────────────────────────────────────┐
│ GATEWAY / MIDDLEWARE        (lexum-server/src/middleware)      │
│   auth · rate_limit · request_size · content_type ·           │
│   query_complexity · ip_filter · metrics · serialization      │
└──────────────────────────────┬─────────────────────────────────┘
                               ▼
┌────────────────────────────────────────────────────────────────┐
│ HANDLERS + ERROR CONTRACT   (lexum-server/src/handlers,        │
│   router.rs, error.rs)   uniform error object → SPEC-003      │
├───────────────────┬────────────────────────────────────────────┤
│  WRITE DOOR       │  READ PATH                                 │
│  task queue +     │  search / _mget / aggregations /           │
│  scheduler        │  multi-search federation (scatter-gather)  │
│  → SPEC-002       │                                            │
└─────────┬─────────┴──────────────────┬─────────────────────────┘
          ▼                            ▼
┌────────────────────────────────────────────────────────────────┐
│ ENGINE                      (lexum-core)                       │
│   index manager · document store · search executor ·          │
│   query builder (LQL) · aggregation · schema · snapshots      │
└──────────────────────────────┬─────────────────────────────────┘
                               ▼
┌────────────────────────────────────────────────────────────────┐
│ TANTIVY 0.25   one long-lived IndexWriter per index ·         │
│                segment store · reader reload = "refresh"       │
└──────────────────────────────┬─────────────────────────────────┘
                               ▼
┌────────────────────────────────────────────────────────────────┐
│ STORAGE  data dir: segments · task log (SPEC-002 §8) ·        │
│          settings/metadata · snapshots                         │
└────────────────────────────────────────────────────────────────┘
```

- **ARC-010** Request flow MUST be strictly downward: protocol → middleware → handler → engine → Tantivy. A layer MUST NOT reach around the layer below it (e.g., a protocol adapter invoking `lexum-core` directly, or a handler opening Tantivy segments).
- **ARC-011** All protocol adapters (REST, MCP, UMICP) MUST converge on the same handler/service layer, so behavior (task semantics, error contract, auth) is identical regardless of protocol.
- **ARC-012** Every error that crosses the handler boundary MUST be expressed as the uniform error object of SPEC-003, on every protocol.

## 4. Write path (target: single write door)

The current write path — each operation creating a fresh `index.writer(50_000_000)` and committing (`crates/lexum-core/src/document/store.rs`), with handlers forcing `refresh_index` inline (`handlers/document.rs`) — is superseded.

- **ARC-020** The task queue (SPEC-002) is the **single write door**: every mutation of index contents or index configuration MUST be enqueued as a task and applied by the per-index scheduler. No other code path MAY obtain a Tantivy `IndexWriter` or call `commit()`.
- **ARC-021** Exactly one long-lived `IndexWriter` per open index, owned by that index's scheduler. `DocumentStore` write methods MUST be refactored to apply operations through a writer handle supplied by the scheduler and MUST NOT create writers or commit.
- **ARC-022** Mutating REST endpoints MUST return `202 Accepted` with a task stub (SPEC-002 §5); write visibility is controlled by the `refresh` parameter (SPEC-005 §5), never by unconditional inline refresh.
- **ARC-023** The persisted task log is the designated operation log for phase9 replication (Meilisearch R-01; Elastic F-039/F-054). Design decisions in the write path (task payload durability, monotonic `uid`, per-index `_seq_no` at apply time — SPEC-005 §6) MUST preserve its replayability as a replication primitive.
- **ARC-024** The legacy in-memory task registry (`handlers/reindex.rs`) and the separate progress system (`handlers/progress.rs`, `lexum-core/src/progress/`) MUST be folded into the task queue; after phase1 there MUST be exactly one source of truth for operation state.

## 5. Read path (target: federation as scatter-gather)

- **ARC-030** Search reads go through `lexum-core/src/search/` executors against Tantivy readers. Reads MUST NOT block on the write path; a search never waits for a commit (except a caller who opted into `refresh=wait_for`, SPEC-005 §5).
- **ARC-031** "Refresh" is defined as a Tantivy reader reload after a commit. The scheduler exposes visibility notifications (batch committed AND readers reloaded) that `refresh=true|wait_for` consume.
- **ARC-032** Multi-index and (later) multi-node querying MUST be built as **federation**: `POST /multi-search` executes N queries and merges ranked lists; the same scatter-gather code path is reused by the phase9 distributed search layer (Meilisearch R-07/F-023). Distribution MUST NOT introduce a second, separate query-fanout mechanism.
- **ARC-033** Per A-03: federated/multi-search MUST support per-query error objects with partial results rather than failing the whole request on one bad sub-query.
- **ARC-034** Per A-04: the simple search endpoint (`q` + `filter` + `sort` flat parameters) MUST remain a first-class door; LQL is the power-user layer and MUST NOT become the only query interface.

## 6. Threading and async model

- **ARC-040** `lexum-server` runs on a multi-threaded Tokio runtime; handlers are `async` and MUST NOT block the runtime. CPU-bound or blocking engine work invoked from a handler MUST go through `tokio::task::spawn_blocking` or a dedicated thread.
- **ARC-041** Each per-index scheduler (SPEC-002 §6) runs its apply loop on a dedicated worker (dedicated thread or `spawn_blocking` scope) — Tantivy `commit()` is blocking and MUST NOT execute on a Tokio core thread. Tantivy's own indexing thread pool is owned by the single writer (ARC-021).
- **ARC-042** Enqueue (the 202 response path) MUST be non-blocking apart from the durable append of the task record + payload; it MUST NOT wait for indexing, commit, or refresh. Gate: enqueue p99 < 50 ms under bulk load (phase1 gate 6.4).
- **ARC-043** Shared engine state uses interior mutability with clear ownership: index registry behind the index manager; readers are cheaply cloneable Tantivy `Searcher` handles; the only writer mutex is the scheduler's (one per index — no global write lock across indexes).

## 7. Configuration model

- **ARC-050** Configuration is a single YAML file (`config.example.yml` is the documented template) loaded at startup by `lexum_core::config::Config`, with top-level sections `cluster`, `node`, `network`, `path`, `logging`, `snapshots`, and (new) `task_queue`. Every key MUST have a documented default; an absent file MUST yield a fully working default configuration.
- **ARC-051** New subsystems MUST surface their tunables in this file (e.g., `task_queue.max_pending_tasks`, `task_queue.max_payload_bytes`, `task_queue.writer_heap_bytes`, batch limits — SPEC-002 §9) rather than environment-only or hardcoded values. Environment variables MAY override individual keys.
- **ARC-052** Config keys are stable once released: renames require a deprecation release where the old key still works and logs a warning.
- **ARC-053** Per-index runtime settings (analyzers, field options, search tuning) are NOT server config: they live in the index settings resource (`index/settings.rs`), are mutated via `settingsUpdate` tasks (SPEC-002), and are stamped by index templates.

## 8. Target architecture summary (post re-planning)

| Concern | Mechanism | Spec |
|---|---|---|
| All writes | Durable task queue, per-index scheduler, one writer, auto-batched commits | SPEC-002 |
| Write visibility | `refresh=true\|false\|wait_for` against scheduler notifications | SPEC-005 §5 |
| All errors | Uniform `{ message, code, type, link }` object | SPEC-003 |
| Bulk ingestion | ES-compatible `_bulk` NDJSON riding the queue | SPEC-005 |
| Concurrency control | Per-index `_seq_no` / `_primary_term` assigned at apply time | SPEC-005 §6 |
| Distribution (phase9) | Replication ships the task log; queries fan out through federation; WAL/translog-class durability work lives under the distribution track and MUST reuse the task log rather than invent a parallel journal | future SPEC (phase9) |

- **ARC-060** Sequencing is normative: SPEC-002 (write door + SPEC-003 errors) MUST land before SPEC-005 semantics, and both before any distribution work — retrofit cost grows with every endpoint added (R-01; F-055).
- **ARC-061** Per A-01: Lexum adopts Meilisearch's *coordination* layer (task queue, batching, uniform errors), not its storage model. Tantivy's segment model remains the storage foundation; nothing in these specs may assume a single global write transaction.
