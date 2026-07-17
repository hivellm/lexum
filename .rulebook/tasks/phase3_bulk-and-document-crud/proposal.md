# Proposal: phase3_bulk-and-document-crud

## Why

`_bulk` with exact NDJSON semantics is P0 item #1 in the Elasticsearch
execution plan (F-015): the single highest-value compatibility endpoint,
because it is what every shipper, ETL tool, and client library actually
calls. Document CRUD parity — `_mget`, `?refresh=true|wait_for`,
optimistic concurrency via seq_no/primary_term — is P0 #3 (F-014), and
seq-nos are also the primitive phase9's replication model is built on
(F-039/F-054: "design seq-nos from day 1").

Lexum already has most of the surface, but not the semantics:

- **Bulk exists but is not `_bulk`.** `POST /api/v1/bulk`
  (`crates/lexum-server/src/handlers/document.rs`) takes a JSON body
  `{ "operations": [...] }` — not the NDJSON action-line wire format —
  so no ES client, Logstash, Filebeat, or `elasticsearch-py` helper can
  use it. Its per-item result shape (`{success, action, _index, _id,
  error}`) is not ES's `items` envelope, `create` is silently treated as
  `index` ("we don't check if exists", document.rs ~line 423), `update`
  with partial `doc` semantics is absent, and the loop builds a new
  `DocumentStore` per operation — inheriting the per-op writer+commit
  pathology phase1 removes.
- **Refresh semantics are hardcoded, not selectable.** Handlers call
  `refresh_index` unconditionally after every write (document.rs lines 71,
  199, 261); there is no `?refresh=true|false|wait_for` parameter, so
  clients can neither opt out of the cost nor reliably read-their-writes
  once phase1 makes writes async.
- **Optimistic concurrency is partial.** `crates/lexum-core/src/document/
  store.rs` threads `_version`/`_version_type` options through its op
  enums, but there is no monotonic per-index `_seq_no`/`_primary_term`,
  no `if_seq_no`/`if_primary_term` request params, and no 409
  version-conflict contract on the REST surface.
- **`_mget` and by-query ops exist but need parity + queue integration.**
  `handlers/query_ops.rs` has `multi_get` (backed by
  `crates/lexum-core/src/document/multi_get.rs`), `update_by_query`, and
  `delete_by_query` — the latter two run synchronously inline
  (`refresh: true` default) and must become tasks per phase1's contract.

## What Changes

1. **ES-compatible `_bulk` endpoint** (`POST /_bulk` and
   `POST /{index}/_bulk` compatibility aliases, plus the `/api/v1` form):
   `application/x-ndjson` body; action lines `index`, `create` (fails on
   existing id), `update` (partial `doc` + `doc_as_upsert`), `delete`;
   metadata `_index`, `_id`, `if_seq_no`/`if_primary_term`; malformed
   NDJSON or action line fails the whole request with 400 (ES semantics),
   while per-document failures are reported per item. Response: `{ took,
   errors, items: [{ index|create|update|delete: { _index, _id, _version,
   _seq_no, _primary_term, status, result | error: { type, reason } } }] }`.
   The legacy JSON `/api/v1/bulk` shape stays as a deprecated alias.
2. **Bulk rides the phase1 queue**: one bulk request enqueues one task
   (202 or, with `refresh`/`wait_for` or `wait_for_completion=true`,
   returns the completed per-item results). Batching means one Tantivy
   commit per bulk request or larger — never per item.
3. **`?refresh=true|false|wait_for`** on document add/update/delete and
   `_bulk`: `false` (new default) returns after durable enqueue+apply;
   `true` forces a refresh before responding; `wait_for` parks the
   response until the next scheduled refresh makes the write visible.
4. **Optimistic concurrency, ES-shaped**: monotonic per-index `_seq_no`
   assigned by the phase1 scheduler at apply time, `_primary_term`
   (constant 1 single-node, real once phase9 lands) — returned on every
   write response and on GET; `if_seq_no` + `if_primary_term` conditional
   writes fail with a 409 `version_conflict` error (uniform error object);
   existing external `_version`/`_version_type` behavior preserved.
5. **`_mget` parity**: ES body shapes (`{docs: [{_index, _id, _source
   filter}]}` and index-scoped `{ids: [...]}`), per-doc `found` flags and
   `_source` filtering, one response entry per requested doc in order.
6. **`_update_by_query` / `_delete_by_query` as tasks**: enqueue on the
   phase1 queue and return a task stub; snapshot the matching doc set at
   start; `conflicts=abort|proceed` honored via seq_no checks at apply
   time; progress/counters (`updated`, `deleted`, `version_conflicts`)
   reported in task `details`.

## Impact

- Affected specs: `specs/bulk-ndjson/spec.md`,
  `specs/document-crud-parity/spec.md` (this task)
- Affected code: `crates/lexum-server/src/handlers/document.rs` (bulk +
  CRUD + refresh param), new `handlers/bulk_ndjson.rs` (NDJSON parser +
  item envelope), `handlers/query_ops.rs` (mget parity, by-query → tasks),
  `crates/lexum-core/src/document/store.rs` (+ seq_no assignment),
  `document/multi_get.rs`, `document/query_operations.rs`,
  `crates/lexum-core/src/tasks/` (bulk task type, wait_for hooks),
  `src/router.rs`, `src/openapi.rs`,
  `src/middleware/content_type.rs` (allow `application/x-ndjson`)
- Breaking change: YES (behavioral) — the default write visibility changes
  from "always refreshed inline" to `refresh=false` (async, read-your-
  writes via `wait_for`), consistent with phase1's 202 contract. The
  legacy JSON bulk endpoint remains as a deprecated alias; no route is
  removed.
- User benefit: the existing ES ecosystem (shippers, SDK bulk helpers,
  migration scripts) can pump data into Lexum unchanged — the highest-
  leverage compatibility win in the entire ES analysis — with safe
  concurrent updates (409 instead of lost writes) and explicit,
  ES-standard control over write visibility.

## Dependencies / sequencing

- **Hard dependency on phase1_write-path-task-queue**: `_bulk`, by-query
  ops, and `refresh`/`wait_for` semantics all ride the task queue, shared
  writer, and uniform error object. Do not start §2+ before phase1's
  scheduler merges.
- Independent of phase2 (search kernel), except `_update_by_query`/
  `_delete_by_query` accept ES-DSL `query` bodies once phase2's
  `es_dsl` adapter exists (use native `Query` until then).
- Feeds phase9: `_seq_no`/`_primary_term` here are the exact primitives
  the replication design consumes (F-039).

## Success criteria (gates)

- Wire-fidelity fixture suite: recorded ES 7.10 `_bulk` request/response
  pairs (success, per-item failure, create-conflict, update-missing-doc,
  malformed-line, mixed actions across indexes) replay against Lexum and
  match on `errors` flag, item order, item envelope keys, `status` codes,
  and `error.type` values.
- Ecosystem smoke gate: `elasticsearch-py` (7.x) `helpers.bulk` or an
  equivalent recorded client exchange ingests ≥100k docs against Lexum
  with zero client-side errors.
- Throughput gate (reuses the phase1 harness): `_bulk` at 5k docs/request
  sustains ≥10x the documents/sec of the pre-phase1 per-document endpoint
  baseline, with commits/request ≤ 1 on average.
- `refresh` semantics: `refresh=false` write is not yet searchable,
  `refresh=true` is searchable in the response turn, `wait_for` blocks
  until searchable and never longer than the refresh interval — each
  proven by integration tests; `GET` after `wait_for` write always
  read-your-writes.
- Concurrency: two conditional updates racing on the same doc — exactly
  one succeeds, the other gets 409 with `version_conflict` error object;
  a stale `if_seq_no` never silently overwrites (test with 100 concurrent
  writers on one doc).
- `_mget` returns per-doc `found:false` (not an error) for missing docs
  and honors per-doc `_source` filters; order preserved.
- `_update_by_query`/`_delete_by_query` return task stubs; task `details`
  counters match the actual mutation count; `conflicts=proceed` records
  `version_conflicts` without failing the task.
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, full test
  suite green.
