## 1. Task model and durable queue
- [ ] 1.1 Define `Task` in a new `crates/lexum-core/src/tasks/` module: `uid: u64` (monotonic, persisted counter), `index_uid: Option<String>`, `task_type` enum (documentAdditionOrUpdate, documentDeletion, indexCreation, indexDeletion, indexUpdate, settingsUpdate, reindex, snapshotCreation, taskCancelation, taskDeletion), `status` enum (enqueued/processing/succeeded/failed/canceled), `details` (per-type payload summary, e.g. receivedDocuments/indexedDocuments), `error: Option<TaskError>` (uniform object, see §5), `batch_uid: Option<u64>`, timestamps enqueuedAt/startedAt/finishedAt + duration
- [ ] 1.2 Implement persistent task storage under the data dir (append/update by uid, scan by status/type/index, atomic status transitions); tasks and the uid counter survive process restart
- [ ] 1.3 Store the task PAYLOAD (documents to index, delete filters, settings diff) durably at enqueue time so processing never depends on the original HTTP request
- [ ] 1.4 Bounded queue (A-06): configurable caps (`task_queue.max_pending_tasks`, `task_queue.max_payload_bytes` in `config.example.yml`); enqueue past the cap fails immediately with error code `task_queue_full` — no silent drop, no unbounded growth
- [ ] 1.5 Retention: `DELETE /tasks?statuses=succeeded,failed,canceled&beforeFinishedAt=...` prunes finished tasks (itself recorded as a `taskDeletion` task)

## 2. Scheduler with shared writer and auto-batching
- [ ] 2.1 Implement a per-index scheduler owning ONE long-lived `tantivy::IndexWriter` (configurable heap), replacing the per-operation `index.writer(50_000_000)` + `commit()` pattern in `crates/lexum-core/src/document/store.rs` (add ~257–272, delete ~360–368, bulk ~422–681); `DocumentStore` write methods are refactored to apply operations through a provided writer handle and no longer create writers or commit
- [ ] 2.2 Auto-batching: the scheduler drains consecutive compatible tasks (same index; document add/update/delete family) into a single batch → one `commit()`; incompatible task types (settingsUpdate, index lifecycle) close the current batch and run alone; assign `batch_uid` to every processed task
- [ ] 2.3 Batch limits (max docs / max bytes per batch) to bound memory and commit latency, configurable with sane defaults
- [ ] 2.4 Failure isolation inside a batch: a document that fails conversion fails ITS task (or its items) with a task error while the rest of the batch commits; a poisoned commit fails all tasks in the batch with the commit error recorded on each
- [ ] 2.5 Expose `GET /batches` + `GET /batches/{uid}` introspection (parity matrix row 23), listing batch composition, timings, and commit stats
- [ ] 2.6 Crash recovery: on startup the scheduler re-enqueues tasks left in `processing` (idempotent replay — a batch is only marked durable after the Tantivy commit returns), and resumes `enqueued` ones in uid order

## 3. Convert all mutating endpoints to enqueue (202 + taskUid)
- [ ] 3.1 Document writes: `POST/PUT/DELETE /api/v1/indices/{index}/documents[/{id}]` in `crates/lexum-server/src/handlers/document.rs` enqueue and return `202 { taskUid, indexUid, status: "enqueued", type, enqueuedAt }`; delete the unconditional `refresh_index` calls (lines 71, 199, 261)
- [ ] 3.2 `/api/v1/bulk` (JSON body) enqueues one `documentAdditionOrUpdate`/mixed task per request; the whole request becomes a single batchable unit (per-item results are stored in task `details` — full ES `_bulk` NDJSON wire parity is phase3, which rides this)
- [ ] 3.3 `_update_by_query` / `_delete_by_query` in `handlers/query_ops.rs` enqueue and return a taskUid instead of executing synchronously
- [ ] 3.4 Index lifecycle: create/delete/settings-update in `handlers/index.rs` become tasks (indexCreation/indexDeletion/indexUpdate/settingsUpdate)
- [ ] 3.5 Reindex: replace the in-memory `RwLock<HashMap>` registry in `handlers/reindex.rs` with queue-backed `reindex` tasks; `/_tasks` routes in `src/router.rs` are re-pointed (or 301-aliased) to the new task API
- [ ] 3.6 Fold `handlers/progress.rs` + `crates/lexum-core/src/progress/` into task `details` progress fields (or back the progress API by the task store); no second source of truth for operation state
- [ ] 3.7 Update `src/openapi.rs` schemas: all write operations documented as 202 + task stub

## 4. Task API
- [ ] 4.1 `GET /tasks` with filters `uids`, `statuses`, `types`, `indexUids`, `beforeEnqueuedAt`/`afterEnqueuedAt`, `beforeFinishedAt`/`afterFinishedAt`; keyset pagination via `from` + `limit` returning `next`
- [ ] 4.2 `GET /tasks/{uid}` returns the full task including `details`, `error`, `batchUid`, timings
- [ ] 4.3 `POST /tasks/cancel?uids=...|statuses=...|indexUids=...` cancels matching enqueued/processing tasks atomically; cancelation is itself a `taskCancelation` task; canceled tasks carry `canceledBy`
- [ ] 4.4 Wire the task routes into `src/router.rs` and auth middleware (task read/cancel scoped like other admin operations)

## 5. Uniform error object on every endpoint (R-02)
- [ ] 5.1 Replace `ErrorResponse { error, details }` in `crates/lexum-server/src/error.rs` with `{ message, code, type, link }`; `type` ∈ `invalid_request | internal | auth | system`; `link` points to `https://docs.lexum.dev/errors#<code>` (stable anchor per code)
- [ ] 5.2 Create the error-code registry (snake_case, stable): map every `ApiError` variant and every ad-hoc `ApiError::Internal(format!(...))` call site in `crates/lexum-server/src/handlers/*` to a registered code (e.g. `index_not_found`, `document_not_found`, `invalid_document_id`, `task_not_found`, `task_queue_full`, `invalid_search_query`)
- [ ] 5.3 Task `error` field and bulk per-item errors reuse the same object shape
- [ ] 5.4 Update `src/openapi.rs` error schemas and regenerate the OpenAPI document
- [ ] 5.5 Endpoint-walking contract test: for every registered route, trigger at least one error (unknown index / bad payload / unknown id) and assert the body deserializes exactly to `{ message, code, type, link }` — gate: 100% of endpoints, zero legacy `{ error, details }` bodies

## 6. Benchmarks and hardening gates
- [ ] 6.1 New write-path benchmark (e.g. `crates/lexum-core/benchmark/write_path_bench.rs` + a server-level run in `benchmark/`): baseline the pre-change per-op writer+commit throughput on 100k single-document adds, then measure through the queue — gate: ≥10x throughput AND a commit-counter assertion of ≥100 tasks/commit average during the run
- [ ] 6.2 Crash-recovery test: enqueue N tasks, kill the process mid-processing, restart, assert all N eventually reach `succeeded` exactly once (no loss, no duplicate application)
- [ ] 6.3 Queue-bound test: fill past `max_pending_tasks`, assert fast failure with `task_queue_full` error object and correct HTTP status; assert the queue drains and accepts again
- [ ] 6.4 Latency sanity: enqueue (202 response) p99 under bulk load stays < 50 ms while batches process in background
- [ ] 6.5 `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, full `cargo test --all-features` green; CHANGELOG + migration notes for the 202/error-shape break

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
