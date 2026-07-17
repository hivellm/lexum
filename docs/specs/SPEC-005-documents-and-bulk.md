# SPEC-005 — Document CRUD & `_bulk`

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | phase3_bulk-and-document-crud · [tasks §1–§6](../../.rulebook/tasks/phase3_bulk-and-document-crud/tasks.md) (hard dependency: phase1 merged) |
| **Planning source** | [phase3 proposal](../../.rulebook/tasks/phase3_bulk-and-document-crud/proposal.md); [Elastic execution plan](../analysis/elastic/08-execution-plan.md) P0 #1 `_bulk` (F-015), P0 #3 CRUD/`_mget`/refresh/concurrency (F-014), seq-nos as replication primitive (F-039, F-054) |

Requirement IDs `DOC-xxx`. RFC 2119 keywords are normative. Every write here **rides the task queue** — enqueue contract, batching, and crash recovery are [SPEC-002](SPEC-002-write-path-task-queue.md); all error bodies and per-item error objects are [SPEC-003](SPEC-003-error-contract.md); layering is [SPEC-001](SPEC-001-architecture.md) §4. Compatibility target: Elasticsearch 7.10 wire behavior where this spec says "ES-shaped".

## 1. Endpoints

| Route | Semantics |
|---|---|
| `POST /_bulk`, `POST /{index}/_bulk`, `POST /api/v1/_bulk` | NDJSON bulk (§2–§4); `{index}` form supplies the default `_index` |
| `POST /api/v1/bulk` | legacy JSON-array bulk — kept working, marked deprecated in OpenAPI |
| `POST /api/v1/indices/{index}/documents` · `PUT .../documents/{id}` · `DELETE .../documents/{id}` | single-document add/update/delete → `202` task stub (SPEC-002 TSK-040) |
| `GET /api/v1/indices/{index}/documents/{id}` | read; returns `_seq_no`/`_primary_term`/`_version` (§6) |
| `POST /api/v1/_mget`, `POST /{index}/_mget` | multi-get (§7) |
| `POST /{index}/_update_by_query`, `POST /{index}/_delete_by_query` | queue tasks (§8) |

- **DOC-001** All mutating routes above enqueue tasks and return `202` stubs unless the caller opted into waiting (§5, DOC-042/DOC-043/DOC-045). None of them may touch a Tantivy writer directly.
- **DOC-002** Content type: `_bulk` routes accept `application/x-ndjson` (and `application/json` for compatibility with lenient clients); all other routes are JSON-only. `middleware/content_type.rs` allows NDJSON **only** on the bulk routes. Request size limits (`middleware/request_size.rs`) still apply; violation → `413 payload_too_large`.

## 2. `_bulk` request wire format (NDJSON)

Body = newline-delimited JSON: an **action line**, optionally followed by a **payload line**, repeated. Final newline RECOMMENDED (ES requires it; Lexum MUST accept a missing trailing newline after the last complete line).

```
{ "index":  { "_index": "products", "_id": "1" } }
{ "title": "Keyboard", "price": 29 }
{ "create": { "_index": "products", "_id": "2" } }
{ "title": "Mouse", "price": 19 }
{ "update": { "_index": "products", "_id": "1", "if_seq_no": 5, "if_primary_term": 1 } }
{ "doc": { "price": 27 }, "doc_as_upsert": false }
{ "delete": { "_index": "products", "_id": "9" } }
```

- **DOC-010** Action lines contain exactly one key ∈ `index | create | update | delete`. Metadata fields: `_index` (REQUIRED unless the URL supplies a default; explicit metadata overrides the URL default), `_id` (REQUIRED for `update`/`delete`; OPTIONAL for `index`/`create` — absent means server-generated id), `if_seq_no` + `if_primary_term` (both or neither, §6).
- **DOC-011** Payload lines: `index`/`create` take the document source; `update` takes `{ "doc": <partial>, "doc_as_upsert": <bool, default false> }`; `delete` has NO payload line.
- **DOC-012** Action semantics at apply time:
  - `index` — upsert: create or full replace.
  - `create` — fails the **item** with status `409`, error code `version_conflict`, when the id already exists (fixes the current behavior where create is silently treated as index).
  - `update` — shallow-merges `doc` into the stored source; missing document fails the item `404 document_not_found` unless `doc_as_upsert: true` (then `doc` becomes the new document).
  - `delete` — missing document is NOT an item error: item reports `status: 404`, `result: "not_found"`, and does not set the response `errors` flag by itself.
- **DOC-013** Request-level parse failures fail the WHOLE request with `400 malformed_payload` (SPEC-003 object; `message` MUST name the 1-based offending line number): malformed JSON on any line, unknown action key, action line with ≠1 keys, missing payload line, `delete` followed by an orphan payload, missing `_index` with no URL default, `_id` on `update`/`delete` absent. Per-document failures (DOC-012) are item-level — this is the ES boundary: parse errors are request-level, apply errors are item-level.
- **DOC-014** Limits, configurable: `documents.bulk_max_actions` (default 10 000 actions/request), `documents.bulk_max_bytes` (default 100 MiB body), `documents.max_document_bytes` (default 16 MiB per document source). Action/body-limit violations → request-level `413 payload_too_large`; a single oversized document → item-level `413 document_too_large` (greedy continue).

## 3. `_bulk` queue integration

- **DOC-020** One `_bulk` request = one `documentAdditionOrUpdate` task (SPEC-002 §3), fully parsed and persisted at enqueue (SPEC-002 TSK-003). The task is batchable per TSK-060: at most one Tantivy commit per bulk request, possibly fewer (several bulks per commit) — never one per item.
- **DOC-021** Item ordering: operations apply in request order within the task; the response `items` array order MUST match request order exactly.
- **DOC-022** Greedy continue: an item failure never aborts the remaining items; every item gets exactly one result entry (acceptance: SPEC-002 TSK-062 failure isolation).

## 4. `_bulk` response envelope

Returned inline when the caller waited (§5), and stored verbatim in task `details.items` otherwise:

```json
{
  "took": 30,
  "errors": true,
  "items": [
    { "index":  { "_index": "products", "_id": "1", "_version": 3,
                  "_seq_no": 17, "_primary_term": 1, "status": 200, "result": "updated" } },
    { "create": { "_index": "products", "_id": "2", "status": 409,
                  "error": { "message": "Document `2` already exists.", "code": "version_conflict",
                             "type": "invalid_request", "link": "https://docs.lexum.dev/errors#version_conflict" } } },
    { "delete": { "_index": "products", "_id": "9", "_version": 1,
                  "_seq_no": 18, "_primary_term": 1, "status": 404, "result": "not_found" } }
  ]
}
```

- **DOC-030** `took` — apply duration in ms. `errors` — `true` iff ≥1 item carries an `error` object (a `delete`→`not_found` alone keeps it `false`, per DOC-012).
- **DOC-031** Each item is an object with exactly one key = its action name. Success fields: `_index`, `_id`, `_version`, `_seq_no`, `_primary_term`, `status` (`201` created, `200` updated/deleted-existing, `404` delete-missing), `result` ∈ `created | updated | deleted | not_found`.
- **DOC-032** Failed items replace `result` with `error` = the SPEC-003 object, plus the canonical `status` for the code (SPEC-003 ERR-042). For ES-client compatibility the fixture suite (phase3 gate 6.1) maps `error.code` values onto expected ES `error.type` strings (e.g. `version_conflict` ↔ `version_conflict_engine_exception`); the body Lexum emits is the SPEC-003 shape.
- **DOC-033** The legacy `/api/v1/bulk` JSON endpoint keeps its existing per-item shape for compatibility but MUST ride the same task type and MUST use SPEC-003 objects for its per-item errors.

## 5. `refresh` semantics

Applies to single-document add/update/delete, `_bulk`, and the legacy bulk route, as query parameter `refresh`.

- **DOC-040** Values: `false` (DEFAULT), `true`, `wait_for`. Any other value → `400 invalid_query_parameter`. The previous behavior (unconditional inline refresh after every write) is removed.
- **DOC-041** `refresh=false` — respond `202` + task stub after durable enqueue; the write becomes searchable whenever its batch commits and readers reload. No visibility guarantee at response time.
- **DOC-042** `refresh=true` — the response waits for: task applied → batch committed → forced reader reload. Response is the completed result (`200`, or the §4 envelope for `_bulk`) and the write IS searchable in the same response turn.
- **DOC-043** `refresh=wait_for` — the response parks on the scheduler's per-task visibility notification (SPEC-001 ARC-031) and returns after the write's batch is committed AND visible, without forcing an extra reload. Bounded by the configured refresh interval + `documents.wait_for_timeout` (default 30 s); expiry → `504 timeout` (the task itself continues — the timeout is on the wait, not the write).
- **DOC-044** Read-your-writes guarantee: after a `refresh=true` or `wait_for` response returns success, a subsequent `GET` of that document and a search matching it MUST observe the write (phase3 gate: tests in tasks.md 2.4).
- **DOC-045** `wait_for_completion=true` (by-query ops, §8) waits on task completion and returns the task summary; it does NOT imply a refresh.

## 6. Optimistic concurrency: `_seq_no` / `_primary_term`

Designed from day 1 as the phase9 replication primitive (F-039).

- **DOC-050** `_seq_no` — per-index monotonic `u64`, assigned by the SPEC-002 scheduler **at apply time** (one increment per applied document operation), persisted with the document and recoverable from the task log. `_primary_term` — `u64`, constant `1` on single-node; reserved to increment on primary failover under phase9.
- **DOC-051** Every write response/item and every document `GET`/`_mget` entry MUST return `_seq_no`, `_primary_term`, and `_version` (the existing external-version field is preserved; `_version_type=external` behavior unchanged).
- **DOC-052** Conditional writes: `if_seq_no` + `if_primary_term` (query params on single-doc update/delete; action-line metadata in `_bulk`). Both MUST be supplied together, else `400 invalid_query_parameter`. At apply time, mismatch with the document's current values → `409 version_conflict` (request-level for single-doc, item-level in `_bulk`). The stale writer never silently overwrites.
- **DOC-053** Atomicity: the compare and the apply execute atomically inside the scheduler's apply loop (single writer per index makes this a local check). Race acceptance: N concurrent conditional updates on one doc → exactly one winner per round, all losers `409`, no lost update (phase3 gate 3.4).

## 7. `_mget`

- **DOC-060** Request shapes (both MUST be accepted): `{ "docs": [ { "_index", "_id", "_source": bool | [fields] | {"includes": [], "excludes": []} } ] }` on `/api/v1/_mget`, and `{ "ids": [ ... ] }` on the index-scoped route (where `_index` defaults from the URL).
- **DOC-061** Response: `{ "docs": [ ... ] }` with exactly one entry per requested doc, in request order: `{ "_index", "_id", "found": bool, "_source"?, "_version"?, "_seq_no"?, "_primary_term"? }`. A missing document is `found: false` — never a request-level error. Per-doc `_source` filtering is applied server-side.
- **DOC-062** `_mget` is a read: never a task, never `202`.

## 8. `_update_by_query` / `_delete_by_query` as tasks

- **DOC-070** Both enqueue (`documentUpdateByQuery` / `documentDeletion` — SPEC-002 §3) and return a `202` stub; the current synchronous inline execution (with forced refresh) is removed. `wait_for_completion=true` parks the response on task completion and returns the summary counters.
- **DOC-071** Apply semantics: snapshot the matching document set (their `_seq_no`s) when the task starts; re-check each document's `_seq_no` at apply. `conflicts=abort` (DEFAULT) fails the task on the first conflict (task `error` = `version_conflict`); `conflicts=proceed` counts the conflict and continues.
- **DOC-072** Task `details` counters: `total`, `updated`/`deleted`, `versionConflicts`, `batches` — and they MUST equal the actual mutation counts (phase3 gate 5.4). Progress updates at checkpoints per SPEC-002 TSK-064; cancelation interrupts at checkpoints per TSK-032.
- **DOC-073** Query bodies: native Lexum `Query`/LQL accepted always; ES-DSL `query` bodies accepted once the phase2 `es_dsl` adapter exists (until then an ES-DSL body → `400 invalid_search_query`).

## 9. Migration notes

- **DOC-080** Behavioral break (accepted by the phase3 proposal): default write visibility changes from "always refreshed inline" to `refresh=false`. Read-your-writes callers MUST send `refresh=wait_for` (or `true`). No route is removed; the legacy JSON bulk stays as a deprecated alias. CHANGELOG MUST document both.
