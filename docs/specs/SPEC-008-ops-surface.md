# SPEC-008 — Ops & Observability Surface

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Phase 6 · phase6_ops-observability-surface §1–§7 ([tasks](../../.rulebook/tasks/phase6_ops-observability-surface/tasks.md)) |
| **Planning source** | [Elastic plan P0 #7 (F-055)](../analysis/elastic/08-execution-plan.md) · [Meilisearch plan R-12 (F-022), A-07](../analysis/meilisearch/08-execution-plan.md) · phase6 code audit (fabricated stats `+=100`/`+=1024` in `admin.rs:234-235`, fake UUIDs, fake JVM framing, dead `handlers/cluster.rs`, unwired metrics middleware, hardcoded `lexum_process_cpu_percent 0.0`) |

Requirement IDs `OPS-xxx`. RFC 2119 keywords are normative. Errors use SPEC-003. Shard/replica dimensions become real with SPEC-011 (distributed clustering); this spec fixes the *shapes* so SPEC-011 only changes values. Gated features interact with SPEC-007 (ES aggregations DSL flag) and SPEC-013 tasks appear in `number_of_pending_tasks`.

## 1. Model — the honesty rule

- **OPS-001** Every numeric value on every ops endpoint MUST be a real measurement of the running system. Fabricated placeholders (per-index `+= 100` docs / `+= 1024` bytes, hardcoded UUIDs, sys_info numbers framed as JVM heap) are forbidden. A metric that cannot yet be measured MUST be omitted or reported under an honest name — never invented.
- **OPS-002** Fields whose meaning only exists in a multi-node cluster (replica shards, relocating shards) MUST be present with their honest single-node value (usually `0`) so ES clients parse the shape today and SPEC-011 fills in the values later.
- **OPS-003** `IndexStats` in lexum-core MUST expose at least: doc count, deleted-doc count, `size_in_bytes` (sum of segment file sizes via the Tantivy directory), segment count, and per-segment detail. All read endpoints below MUST be backed by it — no endpoint computes its own estimates.

## 2. `_cluster/health`

`GET /_cluster/health` — ES 7.x wire shape:

```json
{
  "cluster_name": "lexum",
  "status": "green",
  "timed_out": false,
  "number_of_nodes": 1,
  "number_of_data_nodes": 1,
  "active_primary_shards": 3,
  "active_shards": 3,
  "relocating_shards": 0,
  "initializing_shards": 0,
  "unassigned_shards": 0,
  "delayed_unassigned_shards": 0,
  "number_of_pending_tasks": 0,
  "number_of_in_flight_fetch": 0,
  "task_max_waiting_in_queue_millis": 0,
  "active_shards_percent_as_number": 100.0
}
```

- **OPS-010** Status semantics are normative:

| Status | Single-node meaning | Distributed meaning (SPEC-011) |
|---|---|---|
| `green` | All indices open and loadable; no index failed to load at startup; task queue below its capacity limit. | All primary and replica shards active. |
| `yellow` | Degraded but serving: at least one non-fatal condition (e.g. an index in read-only fallback, task queue above high-water mark). Also the steady state once replicas are *configured* but unassignable on one node. | All primaries active, ≥ 1 replica unassigned. |
| `red` | At least one index failed to load / is unservable — some data is unreachable. | ≥ 1 primary shard not active. |

- **OPS-011** `?wait_for_status=green|yellow|red&timeout=<dur>` MUST be accepted. Single-node, the check is immediate: return at once when the status is reached, or wait up to `timeout` (default 30s) and return the current state with `timed_out: true`. `number_of_pending_tasks` reflects the real SPEC-002 queue depth.
- **OPS-012** `active_shards_percent_as_number` = active/expected shards × 100, as a float; single-node with all indices loaded it is `100.0`.

## 3. `_cat` API

- **OPS-020** `GET /_cat/indices` and `GET /_cat/shards` MUST implement the `_cat` contract: aligned plain-text columns by default (no header row), `?v` adds the header row, `?h=<c1,c2,...>` selects/orders columns, `?format=json` returns an array of objects keyed by column name, `?bytes=b|kb|mb|gb` fixes byte units (default: human-readable).
- **OPS-021** `_cat/indices` columns (default set and order):

| Column | Source |
|---|---|
| `health` | Per-index status per OPS-010 |
| `status` | `open` / `close` |
| `index` | Index name |
| `uuid` | Real persisted index UUID (assigned at creation, survives restart) |
| `pri` | `number_of_shards` from settings |
| `rep` | `number_of_replicas` from settings |
| `docs.count` | Live docs (OPS-003) |
| `docs.deleted` | Deleted docs (OPS-003) |
| `store.size` | Total size on disk |
| `pri.store.size` | Primary size (single-node: equals `store.size`) |

- **OPS-022** `_cat/shards` columns: `index`, `shard`, `prirep`, `state`, `docs`, `store`, `ip`, `node`. Single-node output is one row per index: `shard=0`, `prirep=p`, `state=STARTED`. SPEC-011 adds rows; the schema MUST NOT change.
- **OPS-023** Unknown column names in `?h=` MUST return a SPEC-003 400 error naming the column and listing valid ones.

## 4. `_stats` and `_nodes`

- **OPS-030** `GET /_stats` and `GET /{index}/_stats` MUST return the ES tree: top-level `_all.{primaries,total}` and `indices.{name}.{primaries,total}`, each containing at least `docs: {count, deleted}`, `store: {size_in_bytes}`, and `segments: {count}`. Single-node, `primaries == total`. All values from OPS-003 — an integration test indexing N docs MUST read back exactly N.
- **OPS-031** `GET /_nodes` MUST return node identity: `nodes.{node_id}.{name, transport_address, http: {publish_address}, version, roles}`. `node_id` is a real persisted identifier (generated once, stable across restarts); `version` is the Lexum version; `roles` single-node is `["master","data","ingest"]`.
- **OPS-032** `GET /_nodes/stats` MUST nest under `nodes.{node_id}` and report process truth under honest names: `process: {cpu: {percent}, mem: {resident_bytes}}`, `fs: {total: {total_in_bytes, available_in_bytes}}`, plus `indices` rollups from OPS-003. There is no JVM; `jvm` sections MUST NOT be fabricated.
- **OPS-033** Exactly one implementation of the cluster/stats handlers may exist and it MUST be routed. Unrouted handler modules (today's dead `handlers/cluster.rs`) are non-conformant — revive or delete.

## 5. Prometheus metrics

- **OPS-040** `GET /metrics` serves Prometheus text exposition format, produced by a real metrics registry (the `prometheus` or `metrics` crate — not a hand-rolled formatter). `GET /_metrics` is kept as an alias returning identical output.
- **OPS-041** Metric naming: prefix `lexum_`, snake_case, base units in the name (`_seconds`, `_bytes`), counters end `_total`. Core inventory (normative names):

| Metric | Type | Labels |
|---|---|---|
| `lexum_http_requests_total` | counter | `method`, `route`, `status` |
| `lexum_http_request_duration_seconds` | histogram | `method`, `route` |
| `lexum_search_queries_total` | counter | `index` |
| `lexum_search_duration_seconds` | histogram | `index` |
| `lexum_indexing_operations_total` | counter | `index`, `op` |
| `lexum_index_docs` | gauge | `index` |
| `lexum_index_store_bytes` | gauge | `index` |
| `lexum_tasks_pending` | gauge | — (SPEC-002 queue depth) |
| `lexum_process_cpu_percent` | gauge | — (real measurement; the hardcoded `0.0` is non-conformant) |
| `lexum_process_resident_memory_bytes` | gauge | — |

- **OPS-042** Histograms MUST emit real `_bucket`/`_sum`/`_count` series. The HTTP metrics middleware MUST be wired into the router layer stack — after traffic, `lexum_http_requests_total > 0` (the audit found it never recorded).
- **OPS-043** Route labels use the route *template* (`/api/v1/indices/{index}/settings`), never raw paths — bounded cardinality.
- **OPS-044** Documentation (TELEMETRY.md, API_REFERENCE.md) MUST list only metrics and endpoints that exist; anything aspirational is marked "planned", not documented as present.

## 6. Experimental-features gate (R-12)

- **OPS-050** `GET /experimental-features` returns a flat `{ "<flag>": bool, ... }` object of **all** known flags. `PATCH /experimental-features` performs a partial update; unknown flags MUST be rejected 400 with a SPEC-003 error naming the flag and listing valid ones. No other keys, no nesting.
- **OPS-051** The flag registry is a typed struct in lexum-core config (initial flags: `mcp_protocol`, `umicp_protocol`, `vector_search`, `es_aggregations_dsl`), every flag defaulting to `false`, `deny_unknown_fields` on deserialization.
- **OPS-052** Flag values MUST be persisted synchronously on PATCH (small JSON file in the data dir) and reloaded at startup — a toggled flag survives restart. Toggling takes effect at runtime without restart.
- **OPS-053** Calling an endpoint gated behind a disabled flag MUST return a SPEC-003 `feature_not_enabled` error naming the flag and the PATCH route — never a 404 (discoverability) and never a silent no-op.
- **OPS-054** Mid-flight semantics: flipping a flag **off** MUST NOT silently change the behavior of operations already in flight. Requests admitted while the flag was on run to completion under the old behavior; queued SPEC-002 tasks created while the flag was on either run to completion or fail with `feature_not_enabled` — they MUST NOT be reinterpreted under different semantics. New requests after the flip are rejected per OPS-053.
- **OPS-055** Flag lifecycle: `experimental → GA → removed`. Graduating a flag to GA means the behavior is on unconditionally; the flag key remains readable as `true` and PATCHing it to `false` returns a 400 explaining it has graduated, for at least one minor release before removal. A flag MUST NOT change its meaning between releases.
- **OPS-056** Later phases shipping risky surfaces (SPEC-007 ES aggregations DSL, vector search, MCP/UMICP protocols) MUST gate them behind these flags.

## 7. Telemetry policy (A-07)

- **OPS-060** Telemetry is **opt-in only**: `telemetry.enabled` defaults to `false`, and no collection code path is reachable while it is false (config-off means the send path is a compile-in no-op, not a suppressed send). Default-on with an opt-out flag is forbidden.
- **OPS-061** On startup with telemetry disabled, log exactly one INFO line stating that nothing is collected and how to opt in. With it enabled, log what is collected.
- **OPS-062** What MAY be collected when enabled — anonymous, instance-level only: a random instance ID, Lexum version, OS/arch, coarse counters (index count, total doc count bucketed, feature-flag states, endpoint usage counts). What MUST NEVER be collected: document content, field names, queries or query fragments, index names, API keys, IP addresses, or anything derivable from user data.
- **OPS-063** The full payload schema MUST be documented in `docs/deployment/TELEMETRY.md`, and that document MUST match the implementation exactly (OPS-044 honesty rule applies).

## 8. Acceptance criteria

1. **ES-client parse gate**: an ES 7.x client's `cluster.health()`, `cat.indices()`, `indices.stats()`, and `nodes.info()` all parse Lexum's responses without error.
2. **Real numbers** (OPS-001/030): index N docs → `/{index}/_stats` docs.count == N and `store.size_in_bytes` equals bytes on disk; `grep` finds no `+= 100`/`+= 1024`/fake-UUID code paths; no unrouted handler modules remain (OPS-033).
3. **`_cat` contract**: two indices with docs → `_cat/indices?v&format=json` lists both with correct counts; plain text is column-aligned; header only with `?v`; bad `?h=` column errors per OPS-023.
4. **Flags**: `PATCH {"unknownFlag": true}` → 400 SPEC-003; a valid flag toggles at runtime, survives restart, blocks/unblocks its gated endpoint (OPS-052/053); an in-flight gated task is not reinterpreted on flip-off (OPS-054).
5. **Telemetry off by default**: fresh default-config install has no reachable collection path and logs the opt-in notice exactly once (OPS-060/061).
6. **Prometheus**: after traffic, `/metrics` parses as Prometheus text format, `lexum_http_requests_total > 0`, and `lexum_http_request_duration_seconds_bucket` series exist (OPS-040/042); `/_metrics` output is identical.
