# Proposal: phase6_ops-observability-surface

## Why

The Elasticsearch execution plan puts the ops surface in the P0 compatibility
kernel (item 7 under F-055): "Cluster/ops surface Kibana-style tooling
expects: `_cluster/health`, `_cat/indices`, `_cat/shards`, `_stats`,
`_nodes`". The elastic parity matrix (Monitoring/telemetry row) says to ship
`_cluster/health` + `_cat` compatibility *early*, because dashboards,
Prometheus exporters, Ansible/Terraform health checks, and every ES client's
`ping()`/cluster helper call these endpoints before doing anything useful.
The Meilisearch plan adds two governance items: **R-12** — a runtime
`GET/PATCH /experimental-features` gate (F-022: exactly how Meilisearch
de-risked vector search, v1.3 experimental → v1.13 GA), so Lexum ships MCP,
aggregations, and vector search behind flags; and **A-07** — telemetry must
be opt-in or first-run prompted, never default-on, because default-on
analytics generates community distrust for a young project.

A code audit (2026-07) shows the current surface is partly real, partly
theater, so this task is gap-closing plus honesty repair:

- `_cluster/health|stats|state|settings` and `_nodes/stats` routes exist
  (`crates/lexum-server/src/handlers/admin.rs`), but `get_cluster_stats`
  **fabricates data** (`total_documents += 100; total_size_bytes += 1024` per
  index, admin.rs:234-235), `get_cluster_state` returns hardcoded fake UUIDs,
  and `get_node_stats` frames `sys_info` numbers as fake JVM heap.
- `handlers/cluster.rs` contains a much richer ES-shaped implementation
  (`ClusterHealth` with `active_shards_percent_as_number`, full
  `NodesStats`/`IndicesStats` trees) that is **entirely dead code** — never
  imported by `router.rs`.
- No `_cat/*` routes exist; no ES-form `/_stats`; no plain `/_nodes`
  (docs/api/API_REFERENCE.md documents endpoints that do not exist).
- lexum-core exposes only `IndexStats { name, num_docs, num_segments }`
  (`index/manager.rs:1217`) — no size-on-disk, deleted-doc, or memory stats
  for any endpoint to serve.
- Prometheus is a hand-rolled text formatter (`handlers/metrics.rs`, no
  `prometheus`/`metrics` crate) with **no histogram buckets**, and
  `middleware/metrics.rs::metrics_middleware` is **not wired into the
  router** — `lexum_http_requests_total` stays empty forever. The endpoint is
  `/_metrics` while docs promise `/metrics`.
- No feature-flag system, no telemetry code, and no `telemetry`/`features`
  section in `Config` (`crates/lexum-core/src/config.rs`), despite
  docs/deployment/TELEMETRY.md documenting a full `telemetry:` YAML block.

## What Changes

1. **Real stats plumbing in lexum-core.** Extend `IndexStats`
   (`crates/lexum-core/src/index/manager.rs`) with size-on-disk (sum segment
   file sizes via the Tantivy directory), deleted-doc count, and per-segment
   detail — enough to back `_stats` and `_cat/indices` with real numbers.
   Delete every fabricated value (admin.rs `+=100/+=1024`, fake UUIDs in
   `get_cluster_state`, fake JVM-heap framing in `get_node_stats`).
2. **ES-compatible read surface.** Reconcile the dead `handlers/cluster.rs`
   with the live `admin.rs` (revive its richer ES-shaped types or fold them
   in and delete the dead module): `_cluster/health` gains `cluster_name`,
   `timed_out`, `active_shards_percent_as_number`; add `GET /_stats` and
   `GET /{index}/_stats` (ES shape: `docs`, `store`, `segments` sections);
   add `GET /_nodes` (node identity, roles, http/transport addresses,
   version). Honest single-node semantics now; shard-aware fields carry
   schema placeholders that phase9 (distributed-clustering) fills in.
3. **`_cat` API.** `GET /_cat/indices` and `GET /_cat/shards` with the ES
   contract: aligned-column text by default, `?format=json`, `?v` (headers),
   `?h=` (column selection), `?bytes=`. Single-node `_cat/shards` reports one
   primary shard per index (same schema phase9 extends).
4. **Experimental-features runtime gate (R-12).** `GET/PATCH
   /experimental-features` returning a flat `{flag: bool}` object; an
   `ExperimentalFeatures` registry in lexum-core config with documented
   defaults (all `false`), persisted across restarts; unknown-flag PATCH
   rejected with the uniform error object (R-02, phase1). Wire at least one
   real consumer (gate the MCP/UMICP protocol handlers under
   `crates/lexum-server/src/protocols/`) so the mechanism is proven, and
   adopt the convention that later phases (5, 10) ship risky endpoints behind
   these flags.
5. **Telemetry opt-in scaffold (A-07).** Add a `telemetry` config section
   that is **disabled by default** (`enabled: false`), with a first-run log
   line stating that nothing is collected and how to opt in. Implement only
   the anonymous instance-level payload schema + opt-in switch (no collection
   backend yet); reconcile docs/deployment/TELEMETRY.md with what actually
   exists (it currently documents `/_health`, `/_analytics/queries/*`, OTLP
   tracing, and dozens of metrics that do not exist).
6. **Prometheus alignment.** Wire `metrics_middleware` into the router layer
   stack so HTTP metrics actually record; replace the hand-rolled formatter
   with the `prometheus` (or `metrics` + exporter) crate; emit real histogram
   `_bucket` series for `lexum_http_request_duration_seconds` and
   `lexum_search_duration_seconds`; fix `lexum_process_cpu_percent`
   (currently hardcoded 0.0); serve `/metrics` (keep `/_metrics` as an
   alias); trim TELEMETRY.md's metric inventory to the set actually emitted.

Cross-phase dependencies: depends on **phase1** (uniform error object R-02
for all new error responses; the `_tasks` surface stays where phase1 puts
it). Feeds **phase9** (distributed-clustering fills in shard/node dimensions
of `_cluster/*`, `_cat/shards`, `_nodes` — this task fixes the shapes so
phase9 only changes values) and **phases 5/10** (experimental-features flags
gate their risky endpoints).

## Impact

- Affected specs: `.rulebook/tasks/phase6_ops-observability-surface/specs/`
  (ops-surface spec: endpoint shapes, flag registry, telemetry consent model)
- Affected code: `crates/lexum-server/src/handlers/admin.rs`,
  `crates/lexum-server/src/handlers/cluster.rs` (dead code — revive or
  delete), `crates/lexum-server/src/handlers/metrics.rs`,
  `crates/lexum-server/src/middleware/metrics.rs`,
  `crates/lexum-server/src/router.rs` (new routes + metrics layer), new
  `crates/lexum-server/src/handlers/cat.rs` and
  `handlers/experimental.rs`, `crates/lexum-server/src/openapi.rs`,
  `crates/lexum-core/src/index/manager.rs` (`IndexStats`),
  `crates/lexum-core/src/config.rs` (`telemetry`, `features` sections),
  `docs/deployment/TELEMETRY.md`, `docs/api/API_REFERENCE.md`
- Breaking change: NO (new endpoints; existing responses gain fields and lose
  only fabricated values; `/_metrics` kept as an alias)
- User benefit: Kibana-style tooling, Prometheus scrapers, and ES client
  health checks work out of the box; operators get real numbers instead of
  placeholders; risky features become opt-in flags instead of surprises; and
  Lexum earns trust by shipping telemetry off-by-default.

## Success criteria

- `GET /_cluster/health`, `GET /_stats`, `GET /{index}/_stats`,
  `GET /_nodes`, `GET /_cat/indices?v`, `GET /_cat/shards?v` all return 200
  with the documented shapes; an ES 7.x client's `cluster.health()` and
  `cat.indices()` parse them without error.
- `/{index}/_stats` doc count and store size match the actual index
  (integration test: index N docs, compare) — zero fabricated values remain
  (no `+= 100`/`+= 1024` placeholders, no fake UUIDs).
- No unrouted handler modules remain (`handlers/cluster.rs` routed or
  removed).
- `PATCH /experimental-features {"unknownFlag": true}` → 400 with the
  uniform error object; a valid flag persists across server restart; a gated
  endpoint returns an error with the flag off and works with it on.
- Fresh install with default config emits no telemetry (no collection code
  path reachable) and logs the opt-in notice exactly once.
- `GET /metrics` exposes `lexum_http_requests_total` > 0 after traffic and
  `lexum_http_request_duration_seconds_bucket` series; output passes a
  Prometheus text-format parser test.
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and the
  workspace test suite pass.
