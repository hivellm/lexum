# Proposal: phase1_write-path-task-queue

## Why

This is the architectural keystone of the whole re-plan (Meilisearch analysis
R-01/R-02, motivated by F-006, F-010, F-020, F-027, F-031; parity matrix rows
21–24 mark the async task pipeline as the single largest miss cluster, F-038).
Both analyses agree it must land BEFORE any distribution work: a persistent
task log is already the operation log that phase9 replication will ship, and
the retrofit cost grows with every write endpoint added (Lexum has 39+ today).

The current write path is measurably the worst part of the codebase:

- `crates/lexum-core/src/document/store.rs` creates a **fresh
  `index.writer(50_000_000)` and calls `commit()` for every single
  operation** — `add_document` (~lines 257–272), `delete_document`
  (~360–368), and the bulk path builds one writer per call (~422–681).
  A Tantivy `IndexWriter` spawns indexing threads and allocates a 50 MB
  arena on creation; commits are the expensive fsync point. Per-op
  writer+commit is exactly the anti-pattern R-01 exists to fix.
- `crates/lexum-server/src/handlers/document.rs` additionally calls
  `refresh_index` unconditionally after every add/update/delete (lines 71,
  199, 261) — every write pays commit + reader reload latency inline.
- Task-like surfaces are fragmented and non-durable: `handlers/reindex.rs`
  keeps its own in-memory `RwLock<HashMap>` registry behind `/_tasks`;
  `handlers/progress.rs` + `crates/lexum-core/src/progress/` is a second,
  separate progress system (pause/resume/cancel) covering only bulk/reindex.
  Neither survives a restart; neither covers plain document writes.
- The error contract is `ErrorResponse { error, details }`
  (`crates/lexum-server/src/error.rs`) — no machine-readable `code`, no
  `type`, no docs `link` (R-02, F-027). Handlers stringify errors ad hoc.

## What Changes

1. **Durable task model.** A `Task` record — `uid` (monotonic u64),
   `indexUid`, `type` (documentAdditionOrUpdate, documentDeletion,
   indexCreation/Deletion/Update, settingsUpdate, reindex,
   snapshotCreation, taskCancelation…), `status`
   (`enqueued/processing/succeeded/failed/canceled`), `details`, `error`
   (uniform error object), `enqueuedAt/startedAt/finishedAt/duration` —
   persisted to disk so the queue survives crash/restart and later serves
   as phase9's replication log.
2. **Single scheduler per index with a long-lived shared `IndexWriter`.**
   All mutations flow through the scheduler; `DocumentStore` stops creating
   writers. **Auto-batching**: consecutive compatible tasks (same index,
   document add/update/delete family) are drained into ONE Tantivy commit;
   incompatible tasks (settings, index lifecycle) close the batch. Batch
   membership is recorded on the task (`batchUid`).
3. **Every mutating endpoint returns `202 Accepted` with a task stub**
   (`{ taskUid, indexUid, status: "enqueued", type, enqueuedAt }`)
   immediately: document add/update/delete, `/api/v1/bulk`,
   `_update_by_query`/`_delete_by_query`, index create/delete/settings,
   reindex. Read endpoints are untouched.
4. **Task API**: `GET /tasks` with filters (`uids`, `statuses`, `types`,
   `indexUids`, `beforeEnqueuedAt`/`afterEnqueuedAt`, keyset pagination via
   `from`+`limit`), `GET /tasks/{uid}`, `POST /tasks/cancel?...` (atomic,
   itself a task), `DELETE /tasks?...` for finished-task pruning. The
   in-memory reindex `_tasks` registry and the progress store are folded
   into (or backed by) this single system.
5. **Bounded queue** (A-06): configurable cap on queue size/bytes; when
   full, enqueue fails FAST with an explicit `task_queue_full` error —
   never silent degradation, never unbounded growth.
6. **Uniform error object** `{ message, code, type, link }` on **every**
   endpoint (R-02): stable snake_case `code` registry (e.g.
   `index_not_found`, `invalid_document_id`, `task_queue_full`), `type` in
   `invalid_request | internal | auth | system`, `link` to docs. Replaces
   `ErrorResponse { error, details }` in `crates/lexum-server/src/error.rs`
   and propagates into task `error` fields and bulk per-item errors.

## Impact

- Affected specs: `specs/task-queue/spec.md`, `specs/error-contract/spec.md`
  (this task)
- Affected code: `crates/lexum-core/src/document/store.rs` (writer
  ownership removed), new `crates/lexum-core/src/tasks/` (queue, scheduler,
  batcher, persistence), `crates/lexum-server/src/handlers/document.rs`,
  `handlers/index.rs`, `handlers/query_ops.rs`, `handlers/reindex.rs`,
  `handlers/progress.rs` (fold-in/deprecate), new `handlers/tasks.rs`,
  `crates/lexum-server/src/error.rs`, `src/router.rs`, `src/openapi.rs`
- Breaking change: YES — write endpoints move from synchronous 200/201 to
  202 + taskUid, and the error body shape changes on all endpoints. Lexum
  is pre-1.0/alpha; both analyses say this exact break gets more expensive
  every release it is deferred. CHANGELOG + migration notes included.
- User benefit: writes stop paying commit+refresh latency inline; bulk
  ingestion becomes batched (one commit per batch instead of per document);
  crash-safe write pipeline; a single observable task surface; a
  machine-readable error contract for every SDK to build on; and the
  operation log that distribution (phase9), dumps, and webhooks all reuse.

## Dependencies / sequencing

- **Blocks phase3** (`_bulk`, refresh semantics, by-query ops ride this
  queue) and **phase9** (task log = replication log). No dependency on
  phase2 (read path). Must land first among the three.

## Success criteria (gates)

- 100% of mutating REST endpoints return 202 + task stub and perform the
  write asynchronously; verified by an integration test that enumerates the
  router's write routes.
- `GET /tasks` filters (statuses, types, indexUids, uids, time windows) and
  `GET /tasks/{uid}` status transitions `enqueued→processing→succeeded|failed`
  covered by tests; cancelation of an enqueued task yields `canceled`.
- Auto-batching benchmark gate: a new write-path benchmark (N=100k docs
  submitted as individual add-document tasks) shows **≥10x throughput vs
  the current per-op writer+commit baseline**, and a commit counter proves
  ≥100 tasks per commit on average during the run.
- Kill -9 during processing: on restart, enqueued tasks resume; no task is
  lost or duplicated (idempotent replay), proven by a crash-recovery test.
- Queue cap: filling the queue past the configured bound returns the
  `task_queue_full` error object with HTTP 429/507-class status, tested.
- Error contract: an endpoint-walking contract test asserts every error
  response body deserializes to `{ message, code, type, link }`; zero
  endpoints emit the legacy `{ error, details }` shape.
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, full test
  suite green.
