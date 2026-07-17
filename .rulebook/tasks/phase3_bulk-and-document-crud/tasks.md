## 0. Precondition
- [ ] 0.1 Verify phase1_write-path-task-queue is merged: scheduler + shared writer live, mutating endpoints return 202 task stubs, uniform error object `{message, code, type, link}` in place — this task's write endpoints ride that queue

## 1. `_bulk` NDJSON endpoint (gap-closing: JSON `/api/v1/bulk` exists, ES wire format does not)
- [ ] 1.1 NDJSON parser (new `crates/lexum-server/src/handlers/bulk_ndjson.rs`): streaming line-pair parser for action lines `index`/`create`/`update`/`delete` with metadata `_index`, `_id`, `if_seq_no`, `if_primary_term`; `update` supports `{"doc": ...}` + `doc_as_upsert`; malformed NDJSON or unknown action fails the WHOLE request 400 with the uniform error object naming the offending line number (ES semantics: parse errors are request-level, doc failures are item-level)
- [ ] 1.2 Allow `application/x-ndjson` in `src/middleware/content_type.rs` for the bulk routes only; request size limits still enforced by `middleware/request_size.rs`
- [ ] 1.3 Routes in `src/router.rs`: `POST /_bulk`, `POST /{index}/_bulk` (default index for metadata-less lines), and `/api/v1/_bulk` alias; keep the legacy JSON `/api/v1/bulk` working but marked deprecated in `src/openapi.rs`
- [ ] 1.4 Correct action semantics in the apply path (`crates/lexum-core/src/document/store.rs`): `create` fails with `status: 409, error.type: "version_conflict_engine_exception"`-equivalent when the id exists (today document.rs ~line 423 treats create as index); `update` merges partial `doc` into the stored source and fails 404-per-item on missing doc unless `doc_as_upsert`; `delete` on missing doc reports `result: "not_found"` with status 404, not an error flag
- [ ] 1.5 ES response envelope: `{ took, errors, items: [...] }` where each item is keyed by its action and carries `_index`, `_id`, `_version`, `_seq_no`, `_primary_term`, `status`, and `result` (`created/updated/deleted/not_found`) or `error: { type, reason }`; item order strictly matches request order
- [ ] 1.6 Bulk rides the queue: one `_bulk` request = one queued task (single Tantivy commit per batch, per phase1 batcher); per-item results computed at apply time and stored in task `details`; the endpoint returns completed per-item results when the caller passes `refresh=true|wait_for` (waits on the task) and a 202 task stub otherwise

## 2. `refresh=true|false|wait_for` semantics (greenfield: today refresh is unconditional)
- [ ] 2.1 Add the `refresh` query param to document add/update/delete and `_bulk` in `handlers/document.rs`/`bulk_ndjson.rs`; delete the unconditional `refresh_index` calls (document.rs lines 71, 199, 261) — default becomes `false`
- [ ] 2.2 `refresh=true`: after the task's batch commits, force a reader reload before responding; `refresh=wait_for`: park the response on a scheduler notification that fires when the write's batch is committed AND visible; bound `wait_for` by the configured refresh interval + timeout with a `timeout` error code
- [ ] 2.3 Scheduler support in `crates/lexum-core/src/tasks/`: per-task visibility notification (task applied → batch committed → readers reloaded)
- [ ] 2.4 Integration tests: write with `refresh=false` not yet searchable; `refresh=true` searchable in the same response turn; `wait_for` blocks until searchable and GET-after-write always reads its own write

## 3. Optimistic concurrency: seq_no / primary_term (gap-closing: `_version`/`_version_type` plumbing exists in document/store.rs, no seq_no contract)
- [ ] 3.1 Monotonic per-index `_seq_no` counter assigned by the phase1 scheduler at apply time and persisted with the task log; `_primary_term` fixed at 1 for single-node (field reserved for phase9 replication, per F-039 "design seq-nos from day 1")
- [ ] 3.2 Store `_seq_no`/`_primary_term` (and keep `_version`) per document; return them on every write response item and on `GET /api/v1/indices/{index}/documents/{id}`
- [ ] 3.3 `if_seq_no` + `if_primary_term` params on update/delete (single-doc and bulk items): mismatch → HTTP 409 with `version_conflict` uniform error (or per-item 409 in bulk); existing `_version`/`_version_type=external` behavior retained and tested
- [ ] 3.4 Race test: 100 concurrent conditional updates on one document — exactly one winner per round, all losers get 409, final doc state consistent with the winning sequence; no lost updates

## 4. `_mget` parity (gap-closing: multi_get exists in handlers/query_ops.rs + core document/multi_get.rs)
- [ ] 4.1 Accept both ES body shapes on `POST /api/v1/_mget` and index-scoped `POST /{index}/_mget`: `{docs: [{_index, _id, _source: bool|list|{includes,excludes}}]}` and `{ids: [...]}`
- [ ] 4.2 Response: one entry per requested doc, in request order, each `{_index, _id, found, _source?, _version?, _seq_no?, _primary_term?}`; missing docs are `found: false`, never a request-level error
- [ ] 4.3 Per-doc `_source` filtering implemented in `crates/lexum-core/src/document/multi_get.rs`; tests cover mixed found/missing sets and per-doc source filters

## 5. `_update_by_query` / `_delete_by_query` as queue tasks (gap-closing: synchronous versions exist in handlers/query_ops.rs)
- [ ] 5.1 Convert both handlers to enqueue a task and return the phase1 task stub (202); remove the inline `refresh: true` execution path; optional `wait_for_completion=true` waits on the task and returns the summary
- [ ] 5.2 Apply-time semantics in `crates/lexum-core/src/document/query_operations.rs`: snapshot the matching doc set (seq_nos) at task start; re-check seq_no per doc at apply; `conflicts=abort` (default, fail task on first conflict) vs `conflicts=proceed` (count and continue)
- [ ] 5.3 Task `details` counters: `total`, `updated`/`deleted`, `version_conflicts`, `batches`; accept ES-DSL `query` bodies once phase2's `es_dsl` adapter is available (native `Query` accepted regardless)
- [ ] 5.4 Tests: by-query task lifecycle via `GET /tasks/{uid}`, counter accuracy against a seeded corpus, conflict handling under concurrent writes for both `conflicts` modes

## 6. Fidelity, ecosystem, and throughput gates
- [ ] 6.1 Wire-fidelity fixture suite (new `crates/lexum-server/tests/es_bulk_parity_test.rs`): recorded ES 7.10 `_bulk` exchanges — success, mixed actions across indexes, create-conflict, update-missing, delete-missing, malformed line, oversized doc — asserting `errors` flag, item order, envelope keys, `status`, and `error.type` values
- [ ] 6.2 Ecosystem smoke test: an `elasticsearch-py` 7.x `helpers.bulk`-equivalent recorded exchange ingests ≥100k documents with zero client errors and all docs searchable after refresh
- [ ] 6.3 Throughput gate on the phase1 write benchmark harness: `_bulk` at 5k docs/request sustains ≥10x the pre-phase1 per-document endpoint baseline docs/sec, with ≤1 commit per bulk request on average (commit-counter assertion)
- [ ] 6.4 `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, `cargo test --all-features` green; CHANGELOG entry for the `refresh=false` default-visibility change and the deprecated JSON bulk alias

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
