# SPEC-002 — Write Path: Durable Task Queue

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | phase1_write-path-task-queue · [tasks §1–§4, §6](../../.rulebook/tasks/phase1_write-path-task-queue/tasks.md) |
| **Planning source** | [phase1 proposal](../../.rulebook/tasks/phase1_write-path-task-queue/proposal.md); [Meilisearch execution plan](../analysis/meilisearch/08-execution-plan.md) R-01, A-06 (motivating findings F-006, F-010, F-020, F-027, F-031, F-038); [Elastic execution plan](../analysis/elastic/08-execution-plan.md) F-039, F-054 (task log = future replication log) |

Requirement IDs `TSK-xxx`. All timestamps are RFC 3339 UTC with millisecond precision (`2026-07-17T12:00:00.123Z`). JSON field names are camelCase on the wire. Error bodies follow [SPEC-003](SPEC-003-error-contract.md); the layering context is [SPEC-001](SPEC-001-architecture.md) §4; document/bulk semantics that ride this queue are [SPEC-005](SPEC-005-documents-and-bulk.md).

## 1. Model

Every mutation is a **task**: accepted with `202`, persisted durably at enqueue time, applied asynchronously by a per-index scheduler that owns the index's single long-lived Tantivy `IndexWriter` and drains compatible consecutive tasks into one commit (auto-batching). The persisted task log survives restarts and is the operation log phase9 replication will ship.

- **TSK-001** The task queue is the single write door (SPEC-001 ARC-020): 100% of mutating REST endpoints MUST enqueue a task and return `202 Accepted` + task stub. No handler may mutate an index synchronously.
- **TSK-002** Read endpoints (search, get document, `_mget`, aggregations, task/batch introspection) are unaffected and MUST NOT return task stubs.
- **TSK-003** A task MUST be self-contained: the full payload (documents, delete filters, settings diff) is persisted at enqueue time; processing MUST NOT depend on the original HTTP request or any in-memory state that a restart would lose.

## 2. Task object

```json
{
  "uid": 42,
  "batchUid": 7,
  "indexUid": "products",
  "type": "documentAdditionOrUpdate",
  "status": "succeeded",
  "canceledBy": null,
  "details": { "receivedDocuments": 5000, "indexedDocuments": 5000 },
  "error": null,
  "duration": "PT0.523S",
  "enqueuedAt": "2026-07-17T12:00:00.000Z",
  "startedAt": "2026-07-17T12:00:00.100Z",
  "finishedAt": "2026-07-17T12:00:00.623Z"
}
```

- **TSK-010** `uid` — `u64`, strictly monotonic across the whole instance (not per index), assigned at enqueue, never reused. The counter is persisted and MUST survive restart (no uid is ever assigned twice, even across crashes).
- **TSK-011** `indexUid` — target index name; `null` for global tasks (`taskCancelation`, `taskDeletion`, `snapshotCreation`).
- **TSK-012** `type` — one of the registry in §3. Clients MUST tolerate unknown `type` values (the enum is append-only).
- **TSK-013** `status` — exactly one of `enqueued | processing | succeeded | failed | canceled` (state machine in §4).
- **TSK-014** `details` — per-type object summarizing the request and (once finished) the outcome (§3 table). Fields not yet known are `null`, never omitted-then-added with different meaning.
- **TSK-015** `error` — `null` unless `status = failed` (or `canceled` per TSK-033); when present it is exactly the SPEC-003 error object `{ message, code, type, link }`.
- **TSK-016** `batchUid` — `u64` batch identifier (§7), `null` until the task is drained into a batch.
- **TSK-017** Timestamps: `enqueuedAt` set at enqueue; `startedAt` when the scheduler begins the task's batch; `finishedAt` when the terminal status is recorded; `duration` = `finishedAt − startedAt` as ISO 8601 duration, `null` until finished.
- **TSK-018** `canceledBy` — `uid` of the `taskCancelation` task that canceled this one; `null` otherwise.

## 3. Task types

| `type` | Enqueued by | `details` keys (request → outcome) |
|---|---|---|
| `documentAdditionOrUpdate` | POST/PUT documents, `_bulk`, legacy `/api/v1/bulk` | `receivedDocuments` → `indexedDocuments`; `_bulk` tasks also store per-item results (SPEC-005 DOC-032) |
| `documentDeletion` | DELETE document(s), `_delete_by_query` | `providedIds` and/or `originalQuery` → `deletedDocuments`, `versionConflicts` |
| `documentUpdateByQuery` | `_update_by_query` | `originalQuery`, `script/doc` → `total`, `updated`, `versionConflicts`, `batches` |
| `indexCreation` | PUT/POST index | `primaryKey`/settings summary |
| `indexDeletion` | DELETE index | `deletedDocuments` |
| `indexUpdate` | index open/close/settings-adjacent lifecycle | changed attributes |
| `settingsUpdate` | PUT/PATCH index settings | the settings diff |
| `reindex` | POST reindex | `sourceIndex`, `destIndex` → `processedDocuments`, `totalDocuments` (progress, TSK-064) |
| `snapshotCreation` | POST snapshot | snapshot name/target |
| `taskCancelation` | POST `/tasks/cancel` | `matchedTasks`, `canceledTasks`, `originalFilter` |
| `taskDeletion` | DELETE `/tasks` | `matchedTasks`, `deletedTasks`, `originalFilter` |

- **TSK-020** New task types MAY be added; existing names and their `details` keys MUST NOT be renamed or repurposed.

## 4. Status state machine

```
             ┌────────────► canceled  (terminal)
             │                  ▲
 enqueued ───┴──► processing ───┤
                     │          │  (cancelation reached it in time)
                     ├──► succeeded  (terminal)
                     └──► failed     (terminal)
```

- **TSK-030** Legal transitions are exactly: `enqueued→processing`, `enqueued→canceled`, `processing→succeeded`, `processing→failed`, `processing→canceled`. Terminal states never transition again. Status transitions MUST be atomic in the task store (a crash never leaves a half-written status).
- **TSK-031** A task in a batch whose Tantivy commit has returned MUST be marked `succeeded`/`failed` per its own outcome — the Tantivy commit is the durability point: a batch counts as applied if and only if its commit returned.
- **TSK-032** Cancelation of a `processing` task is best-effort at interruption points (batch boundaries, per-chunk checkpoints inside by-query/reindex tasks). If the task's batch already committed, the commit wins and the task ends `succeeded`.
- **TSK-033** A `canceled` task carries `canceledBy` (TSK-018) and MAY carry an `error` object with code `task_canceled` explaining partial progress in `details`.

## 5. Enqueue contract (every mutating endpoint)

- **TSK-040** Response: HTTP `202 Accepted`, body is the **task stub**:

```json
{ "taskUid": 42, "indexUid": "products", "status": "enqueued",
  "type": "documentAdditionOrUpdate", "enqueuedAt": "2026-07-17T12:00:00.000Z" }
```

- **TSK-041** The stub MUST be returned only after the task record + payload are durably persisted (fsync of the task log append). A `202` is a promise the task survives kill -9.
- **TSK-042** Endpoints that MUST enqueue (initial registry; any future mutating route joins it): document add/update/delete (`/api/v1/indices/{index}/documents[/{id}]`), `_bulk` + legacy `/api/v1/bulk`, `_update_by_query`, `_delete_by_query`, index create/delete/open/close, index settings update, reindex, snapshot creation, `/tasks/cancel`, `DELETE /tasks`.
- **TSK-043** Request validation that does not require index contents (malformed JSON/NDJSON, invalid index name, missing required fields, payload over size limits) MUST fail synchronously with a SPEC-003 error (4xx) — invalid requests are rejected, not enqueued. Failures that depend on index state (missing document, version conflict) surface asynchronously on the task (or per bulk item).
- **TSK-044** SPEC-005 defines the only sanctioned synchronous-looking behavior: `refresh=true|wait_for` and `wait_for_completion=true` park the *response* on task completion; the write still flows through the queue.

## 6. Scheduler and shared writer

- **TSK-050** Exactly one scheduler per open index, owning that index's single long-lived `tantivy::IndexWriter` (heap = `task_queue.writer_heap_bytes`, default 128 MiB). `DocumentStore` MUST NOT create writers or call `commit()` (removes the per-op `index.writer(50_000_000)` + `commit()` pattern in `crates/lexum-core/src/document/store.rs`).
- **TSK-051** The scheduler processes tasks in `uid` order per index. Tasks for different indexes MAY proceed concurrently (one scheduler each); global tasks (`taskCancelation`, `taskDeletion`, `snapshotCreation`) are serialized by a global scheduler slot.
- **TSK-052** The apply loop runs off the Tokio core threads (SPEC-001 ARC-041); `commit()` never executes on an async runtime thread.

## 7. Auto-batching

- **TSK-060** The scheduler drains **consecutive compatible** tasks into one batch → exactly one Tantivy `commit()` per batch. Compatible = same `indexUid` AND `type` ∈ { `documentAdditionOrUpdate`, `documentDeletion` (id-based) }. Any other type — `settingsUpdate`, index lifecycle, `reindex`, by-query tasks (they need a read-consistent snapshot), global tasks — closes the current batch and runs as a batch of one.
- **TSK-061** Batch limits, configurable: `task_queue.batch_max_docs` (default 100 000) and `task_queue.batch_max_bytes` (default 256 MiB payload). Hitting a limit closes the batch; the next task starts a new one.
- **TSK-062** Every processed task gets `batchUid` (monotonic `u64`). Failure isolation: a document that fails conversion fails **its** task (or its bulk items) with a SPEC-003 error while the rest of the batch commits; a failed `commit()` fails **all** tasks in the batch, each carrying the commit error.
- **TSK-063** Introspection: `GET /batches` (same filter/pagination style as §10) and `GET /batches/{uid}` MUST return batch composition (task uids, types with counts), timings (`startedAt`, `finishedAt`, `duration`), and commit stats (docs written, commit duration).
- **TSK-064** Long-running single-task batches (`reindex`, by-query) MUST update progress counters in task `details` at checkpoints, replacing the separate progress system (`handlers/progress.rs`, `lexum-core/src/progress/`) — one source of truth (SPEC-001 ARC-024).
- **TSK-065** Throughput gate (normative acceptance): 100k single-document add tasks achieve ≥10x the pre-queue per-op writer+commit baseline, with ≥100 tasks per commit on average.

## 8. Persistence and crash recovery

- **TSK-070** The task store lives under the data dir (`path.data`/tasks/): append-oriented records addressable by `uid`, scannable by status/type/index, with atomic status updates and a persisted `uid`/`batchUid` counter. Storage format is an implementation detail; the durability and ordering guarantees here are not.
- **TSK-071** On startup the scheduler MUST: (a) re-enqueue every task found in `processing` (its batch never reached commit — see TSK-031), (b) resume `enqueued` tasks in `uid` order. No task is lost; no task is applied twice.
- **TSK-072** Replay idempotency: re-applying a recovered task after a crash-before-commit MUST yield the same final index state (document operations are keyed by document id; a batch is only durable when its commit returns).
- **TSK-073** The task log is the future replication log (F-039/F-054): records MUST be replayable in `uid` order onto an empty index to reproduce state, and the format MUST NOT bake in node-local absolute paths or ephemeral identifiers.
- **TSK-074** Acceptance: kill -9 during processing → on restart, all previously enqueued tasks eventually reach a terminal state exactly once, verified by the phase1 crash-recovery test (tasks.md 6.2).

## 9. Bounded queue (A-06)

- **TSK-080** Caps, configurable in `config.yml`: `task_queue.max_pending_tasks` (default 10 000 tasks in non-terminal states) and `task_queue.max_payload_bytes` (default 10 GiB of pending payloads).
- **TSK-081** Enqueue past either cap MUST fail fast with HTTP `429` and error code `task_queue_full` (SPEC-003 registry) — never silent drop, never blocking the client, never unbounded growth. The error `message` MUST state which cap was hit and its configured value.
- **TSK-082** Once the queue drains below the cap, enqueue MUST succeed again with no operator intervention (acceptance: phase1 tasks.md 6.3).

## 10. Task API

### 10.1 `GET /tasks`

- **TSK-090** Filters (all optional, comma-separated multi-values, AND-combined across parameters, OR within one): `uids`, `statuses`, `types`, `indexUids`, `batchUids`, `canceledBy`, and time windows `beforeEnqueuedAt`/`afterEnqueuedAt`, `beforeStartedAt`/`afterStartedAt`, `beforeFinishedAt`/`afterFinishedAt` (RFC 3339, exclusive bounds). Unknown filter values (e.g. a bogus status) → `400 invalid_task_filter`.
- **TSK-091** Pagination is keyset: `from` (start uid, inclusive, descending) + `limit` (default 20, max 1000). Response:

```json
{ "results": [ /* full task objects, uid descending */ ],
  "total": 130, "limit": 20, "from": 42, "next": 22 }
```

`next` is the `from` for the next page, `null` when exhausted.

### 10.2 `GET /tasks/{uid}`

- **TSK-092** Returns the full task object (§2) or `404 task_not_found`.

### 10.3 `POST /tasks/cancel`

- **TSK-093** Accepts the same filters as TSK-090 (at least one filter is REQUIRED — canceling everything unfiltered is `400 missing_task_filters`). Only `enqueued` and `processing` tasks match. The cancelation is atomic over the matched set at evaluation time, is itself enqueued as a `taskCancelation` task (global slot, priority: it jumps to the front of scheduling), and returns its own `202` stub. Matched tasks end `canceled` with `canceledBy` set (subject to TSK-032).

### 10.4 `DELETE /tasks` (retention)

- **TSK-094** Accepts the same filters (at least one REQUIRED); only tasks in **terminal** states match — deleting non-finished tasks is impossible by construction. Recorded as a `taskDeletion` task; returns a `202` stub.
- **TSK-095** Automatic retention: finished tasks are kept indefinitely by default up to `task_queue.max_finished_tasks` (default 1 000 000); past that, the oldest finished tasks are pruned first (recorded as system-initiated `taskDeletion`). `enqueued`/`processing` tasks are NEVER auto-pruned.
- **TSK-096** Task routes are wired through the standard auth middleware; reading and canceling tasks require the same scope class as other admin operations.

## 11. Migration notes

- **TSK-100** Breaking change (pre-1.0, accepted by the phase1 proposal): write endpoints move from synchronous `200/201` bodies to `202` + stub; the in-memory `/_tasks` reindex registry routes are re-pointed (or `301`-aliased) to `GET /tasks`. CHANGELOG + migration notes are part of the phase1 gate; error-shape migration is SPEC-003 §6.
