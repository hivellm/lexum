## 1. Real stats plumbing in lexum-core
- [ ] 1.1 Extend `IndexStats` (crates/lexum-core/src/index/manager.rs:1217) beyond `{name, num_docs, num_segments}`: add `size_in_bytes` (sum segment file sizes via the Tantivy directory), `deleted_docs`, and per-segment detail (docs, deletes) exposed by `IndexManager::get_index_stats`
- [ ] 1.2 Add an `IndexManager` method returning stats for all indices in one pass (backing `/_stats`, `_cat/indices`, and `_cluster/stats`) with unit tests asserting real values against a freshly built index
- [ ] 1.3 Remove all fabricated ops data: the `+= 100`/`+= 1024` placeholders in `admin::get_cluster_stats` (crates/lexum-server/src/handlers/admin.rs:234-235), the hardcoded fake UUIDs/node attributes in `get_cluster_state`, and the fake JVM-heap framing in `get_node_stats` (report real process RSS/CPU from sys_info under honest names)

## 2. ES-compatible cluster/stats/nodes endpoints
- [ ] 2.1 Reconcile dead `crates/lexum-server/src/handlers/cluster.rs` with the live `admin.rs`: keep exactly one implementation of the cluster handlers (revive the richer ES-shaped structs or fold them into admin.rs), route it, and delete the dead module — no unrouted handler modules remain
- [ ] 2.2 Bring `GET /_cluster/health` to ES 7.x shape: add `cluster_name`, `timed_out`, `active_shards_percent_as_number`, `number_of_pending_tasks`; keep honest single-node values; support `?wait_for_status=&timeout=` (immediate return single-node)
- [ ] 2.3 Add `GET /_stats` and `GET /{index}/_stats` with ES-shaped `_all`/`indices.{name}` trees containing `primaries.docs` (count, deleted), `primaries.store.size_in_bytes`, and `segments.count`, backed by §1 stats
- [ ] 2.4 Add `GET /_nodes` (node identity: name, roles, version, http/transport publish addresses from `Config.network`) and align `GET /_nodes/stats` to nest under `nodes.{node_id}` the way ES clients expect
- [ ] 2.5 Register all new routes in crates/lexum-server/src/router.rs and document them in crates/lexum-server/src/openapi.rs

## 3. _cat API
- [ ] 3.1 New `crates/lexum-server/src/handlers/cat.rs` with a shared column-table formatter implementing the `_cat` contract: aligned plain-text by default, `?v` (header row), `?h=` (column selection), `?format=json`, `?bytes=b|kb|mb`
- [ ] 3.2 `GET /_cat/indices`: health, status, index, uuid, pri, rep, docs.count, docs.deleted, store.size, pri.store.size — real values from §1
- [ ] 3.3 `GET /_cat/shards`: one primary-shard row per index (index, shard=0, prirep=p, state=STARTED, docs, store, node) — schema ready for phase9 to add rows
- [ ] 3.4 Integration test: create 2 indices with docs, assert `_cat/indices?v&format=json` returns both with correct doc counts, and plain-text output has aligned columns and headers only with `?v`

## 4. Experimental-features gate (R-12)
- [ ] 4.1 `ExperimentalFeatures` registry in crates/lexum-core/src/config.rs: typed struct of boolean flags (start with `mcp_protocol`, `umicp_protocol`, `vector_search`, `es_aggregations_dsl`), all default `false`, serde with `deny_unknown_fields`, persisted to a small JSON file in the data dir and reloaded at startup
- [ ] 4.2 New `crates/lexum-server/src/handlers/experimental.rs`: `GET /experimental-features` (flat `{flag: bool}` object) and `PATCH /experimental-features` (partial update; unknown flag → 400 with the uniform error object from phase1 R-02; persists synchronously)
- [ ] 4.3 Wire one real consumer: gate the MCP/UMICP protocol surface (crates/lexum-server/src/protocols/) behind its flags — disabled flag → feature-disabled error mentioning the flag name
- [ ] 4.4 Tests: PATCH toggles at runtime without restart, unknown flag rejected, value survives restart (persist/reload roundtrip), gated endpoint blocked/unblocked by flag

## 5. Telemetry opt-in scaffold (A-07)
- [ ] 5.1 Add `TelemetryConfig { enabled: bool (default false), instance_id: Option<String> }` to `Config` (crates/lexum-core/src/config.rs) with env override support via the existing `apply_env_overrides`
- [ ] 5.2 On startup with telemetry disabled (default), log a single INFO line stating no telemetry is collected and how to opt in; with it enabled, log what would be collected (anonymous instance-level payload schema only — no collection backend in this phase)
- [ ] 5.3 Test asserting a default-config server has no reachable telemetry code path (config default is false and the send path is a no-op unless enabled)

## 6. Prometheus alignment
- [ ] 6.1 Wire `metrics_middleware` (crates/lexum-server/src/middleware/metrics.rs) into the router layer stack so `lexum_http_requests_total` and duration series actually record — currently never called in production
- [ ] 6.2 Replace the hand-rolled text formatter in crates/lexum-server/src/handlers/metrics.rs with the `prometheus` crate (or `metrics` + exporter): counters, gauges, and real histograms with `_bucket` series for `lexum_http_request_duration_seconds` and `lexum_search_duration_seconds`
- [ ] 6.3 Fix `lexum_process_cpu_percent` (currently hardcoded 0.0) and keep `lexum_search_queries_total`/`lexum_indexing_operations_total` recording sites (handlers/search.rs, handlers/document.rs) working against the new registry
- [ ] 6.4 Serve `GET /metrics` with `/_metrics` kept as an alias; test that output parses as Prometheus text format and contains `_bucket` lines after traffic
- [ ] 6.5 Reconcile docs/deployment/TELEMETRY.md and docs/api/API_REFERENCE.md with reality: remove or mark-as-planned every endpoint/metric that does not exist (`/_health`, `/_analytics/queries/*`, OTLP tracing, `lexum_cluster_*`/`lexum_disk_*` series), and document the actual metric inventory

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
