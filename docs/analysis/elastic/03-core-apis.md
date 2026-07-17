# 3. Core APIs

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-014…).

Elasticsearch's REST surface is enormous (hundreds of endpoints). The subset below is the part that constitutes "Elasticsearch" in users' minds.

## 3.1 Document CRUD

- `PUT /{index}/_doc/{id}` / `POST /{index}/_doc` — index (upsert) a document; supports `op_type=create`, optimistic concurrency via `if_seq_no` + `if_primary_term`, and `?refresh=true|wait_for`.
- `GET /{index}/_doc/{id}` (realtime), `HEAD`, `GET /_mget` for batches.
- `POST /{index}/_update/{id}` — partial update (doc merge or script); `_update_by_query` and `_delete_by_query` run a query and rewrite/delete matches (implemented as scroll + bulk internally).
- `DELETE /{index}/_doc/{id}`.
- ([Document APIs](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs.html))

### F-014 — The CRUD contract clients depend on: optimistic concurrency (`if_seq_no` + `if_primary_term`), `?refresh=true|wait_for`, and realtime get
- **Evidence:** [Document APIs](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs.html)
- **Impact:** These parameters are not optional decoration — libraries and shippers use them. Lexum needs seq_no/primary_term (or a documented Lexum equivalent) and correct `refresh=wait_for` semantics for CRUD compatibility (P0 in the [execution plan](08-execution-plan.md)).
- **Confidence:** High

## 3.2 `_bulk`

`POST /_bulk` with NDJSON (action line + optional source line per op: `index`/`create`/`update`/`delete`). Returns per-item status; partial failures are normal and callers must inspect `items`. ([Bulk API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html))

### F-015 — `_bulk` is the highest-value single endpoint for compatibility: every shipper and client uses it
- **Evidence:** [Bulk API docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html)
- **Impact:** `_bulk` is **the** ingestion workhorse. Getting its semantics exactly right — NDJSON framing, per-item errors (partial failures are normal; callers must inspect `items`), `?refresh` — unlocks the entire shipper ecosystem for Lexum in one endpoint. Lexum's current bulk support is partial (see [parity matrix](07-parity-matrix.md)).
- **Confidence:** High

## 3.3 `_search` and the Query DSL

`GET|POST /{index}/_search` with a JSON body: `query` (Query DSL), `from`/`size`, `sort`, `_source` filtering, `highlight`, `aggs`, `suggest`, `search_after`, `track_total_hits`, `runtime_mappings`, `knn`. Also `_count`, `_explain`, `_validate/query`, `_msearch` (bulk searches), `_search/template`. [§4](04-query-dsl.md) covers the DSL itself.

## 3.4 Aggregations framework

Three families, arbitrarily nestable ([aggregations docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html)):

- **Bucket** — group docs: `terms`, `range`, `date_histogram`, `histogram`, `filters`, `nested`, `composite` (paginated buckets), `significant_terms`, geo grids.
- **Metric** — compute per bucket: `min`/`max`/`sum`/`avg`/`stats`, `cardinality` (HyperLogLog++), `percentiles` (TDigest/HDR), `top_hits`, `value_count`.
- **Pipeline** — aggs over agg outputs: `derivative`, `moving_fn`, `cumulative_sum`, `bucket_script`, `bucket_selector`.

Aggregations run off **doc values** (columnar), distributed as map-reduce: each shard computes partial results, coordinator reduces. Accuracy caveats are part of the contract (e.g. `terms` agg counts are approximate across shards, with `doc_count_error_upper_bound`).

### F-016 — Aggregations — more than full-text search — made ES the backbone of Kibana, observability, and analytics; they are Lexum's highest-leverage missing feature
- **Evidence:** [Aggregations docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html); three nestable families (bucket/metric/pipeline) running off columnar doc values as distributed map-reduce
- **Impact:** Without aggregations, Lexum cannot serve the dashboard/analytics workloads that constitute most real ES usage. Note the accuracy caveats are part of the API contract (approximate cross-shard `terms` counts with `doc_count_error_upper_bound`) and must be reproduced, not "fixed".
- **Confidence:** High

### F-017 — Tantivy ships an aggregation module intentionally modeled on (a subset of) ES aggregation JSON, which Lexum can expose almost directly
- **Evidence:** [Tantivy aggregation docs](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html) — bucket/metric aggs like terms, range, histogram, date_histogram, stats, percentiles
- **Impact:** The highest-leverage gap (F-016) has a shortcut: Lexum does not need to build an aggregation engine, only wire Tantivy's ES-shaped module into `_search`'s `aggs` and add the distributed reduce step later.
- **Confidence:** High

## 3.5 Pagination: scroll vs `search_after` vs PIT

- `from`/`size` — cheap but capped by `index.max_result_window` (default 10,000) because deep paging is O(shards × from+size).
- **scroll** — legacy cursor holding a snapshot per request; expensive, stateful, **no longer recommended for deep pagination** ([paginate docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/paginate-search-results.html)).
- **`search_after`** — stateless keyset pagination on a sort key; the modern default.
- **PIT (point in time, 7.10+)** — `POST /{index}/_pit?keep_alive=1m` pins a consistent view of segments; combine `search_after` + `pit` for consistent deep pagination ([PIT API](https://www.elastic.co/guide/en/elasticsearch/reference/current/point-in-time-api.html)).

### F-018 — ES itself deprecated scroll in favor of `search_after` + PIT; Lexum should implement the modern pair and skip classic scroll
- **Evidence:** [Paginate search results docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/paginate-search-results.html), [PIT API](https://www.elastic.co/guide/en/elasticsearch/reference/current/point-in-time-api.html); `from`/`size` capped at `index.max_result_window` (default 10,000) because deep paging is O(shards × from+size)
- **Impact:** Lexum avoids ES's legacy: implement `search_after` (stateless keyset pagination) + PIT (pinned segment view, 7.10+ so inside the compatibility target) and offer classic scroll at most as a thin compatibility shim.
- **Confidence:** High

## 3.6 Mappings and analyzers

- **Mapping** = schema per index: field types (`text`, `keyword`, `long`, `double`, `date`, `boolean`, `ip`, `geo_point`, `nested`, `object`, `dense_vector`, `sparse_vector`, `semantic_text`, `flattened`, `join`, `range` types, `search_as_you_type`, `rank_feature`...), multi-fields (`fields: { raw: { type: keyword } }`), `dynamic` mapping (auto-add fields on first sight; `strict`/`false` to disable), dynamic templates. Fields can be added to a live index; **existing fields cannot change type** — reindex required. ([Mapping docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping.html))
- **Analysis** = char filters → tokenizer → token filters, configurable per field at index and search time; built-in analyzers (`standard`, `english` and ~30 other language analyzers, `whitespace`, `pattern`, `icu`, `kuromoji` via plugins), plus `_analyze` endpoint for debugging ([analysis docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/analysis.html)). Synonyms, stemming, stopwords, edge-ngrams for autocomplete all live here.

### F-019 — Dynamic mapping of arbitrary user JSON causes mapping explosion; ES's guardrail is `index.mapping.total_fields.limit` (default 1000)
- **Evidence:** [Mapping docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/mapping.html); guardrail exists because unbounded field auto-creation grows cluster state without limit (see F-008 and the anti-goals in [§7](07-parity-matrix.md))
- **Impact:** Lexum should be strict-by-default or tightly cap dynamic mapping, and add a `flattened`-style type early. Also note the schema contract: fields can be added live but existing fields can never change type (reindex required) — a semantic Lexum must match.
- **Confidence:** High

### F-020 — The analysis chain (char filters → tokenizer → token filters, per field, index- and search-time) is where synonyms, stemming, stopwords, and autocomplete live; ES ships ~30 language analyzers
- **Evidence:** [Analysis docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/analysis.html); `_analyze` debugging endpoint; `icu`/`kuromoji` via plugins
- **Impact:** Search relevance in practice is mostly analyzer configuration. Lexum needs the configurable-chain model plus an `_analyze`-equivalent; the language-analyzer breadth gap versus Tantivy is quantified in F-052.
- **Confidence:** High

## 3.7 Index templates, data streams, ILM

- **Composable index templates** (7.8+): `index_patterns` + priority + reusable `component_templates`; auto-apply settings/mappings/aliases at index creation ([templates docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-templates.html)).
- **Data streams**: append-only abstraction over auto-rolled backing indices (`.ds-logs-...-000001`), the standard target for logs/metrics.
- **ILM** ([docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-lifecycle-management.html)): policies move indices through phases **hot → warm → cold → frozen → delete**, with actions per phase: `rollover` (by size/age/docs), `shrink`, `forcemerge`, `allocate` (tier migration), `searchable_snapshot`, `readonly`, `delete`. ILM is what makes ES viable for time-series retention at scale.

### F-021 — ILM (rollover + phase transitions + delete) is what makes ES viable for time-series retention; Lexum already has the two prerequisites (templates, snapshots) for an "ILM-lite"
- **Evidence:** [ILM docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-lifecycle-management.html), [templates docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-templates.html); data streams (`.ds-logs-...-000001`) are the standard log/metric target
- **Impact:** Rollover by size/age + a delete phase + data-stream-style aliases is the 80% of ILM that log retention needs — a P1 item in the [execution plan](08-execution-plan.md). Full hot/warm/cold/frozen tiering requires distribution first.
- **Confidence:** High

## 3.8 Snapshots

- Repository-based (`fs`, `s3`, `gcs`, `azure`, `hdfs`), **incremental at segment-file level** across snapshots, restorable per-index; `_snapshot` APIs + **SLM** (snapshot lifecycle management) for scheduled snapshots + retention ([snapshot/restore docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/snapshot-restore.html)).
- **Searchable snapshots** (7.11+, paid tier): mount snapshots as read-only indices backed by local cache — the basis of the frozen tier ([7.11 announcement](https://www.elastic.co/blog/whats-new-elastic-7-11-0-searchable-snapshots-schema-on-read)).

### F-022 — ES snapshots are incremental at the segment-file level (immutable segments make this natural); searchable snapshots turn object storage into a queryable frozen tier
- **Evidence:** [Snapshot/restore docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/snapshot-restore.html), [7.11 announcement](https://www.elastic.co/blog/whats-new-elastic-7-11-0-searchable-snapshots-schema-on-read)
- **Impact:** Lexum already has backup/restore; the incremental-by-segment design and SLM-style scheduling are the natural next steps, with object-storage repositories (s3/gcs/azure) as the deployment expectation. Searchable snapshots are a later, post-distribution play (P2).
- **Confidence:** High

## 3.9 Security

Free tier (since 6.8/7.1): TLS, authentication realms (native, file, LDAP/AD/SAML/OIDC in paid tiers), **RBAC roles** (cluster privileges + index privileges by pattern), **API keys** (create/invalidate, can carry restricted role descriptors, expiration) ([security APIs](https://www.elastic.co/guide/en/elasticsearch/reference/current/security-api.html)). Paid: document-level and field-level security (DLS/FLS), audit logging.

### F-023 — ES 8.0 made security on-by-default (auto-generated certs/enrollment tokens) after years of exposed open clusters; Lexum's missing middle layer is index-pattern-scoped RBAC roles
- **Evidence:** [Security APIs](https://www.elastic.co/guide/en/elasticsearch/reference/current/security-api.html), [security became free](https://www.elastic.co/blog/security-for-elasticsearch-is-now-free); ES 8.0 secure-by-default was a lesson learned from exposed clusters
- **Impact:** Lexum already has API keys and rate limiting; roles with index-pattern-scoped privileges are the minimum for multi-tenant use (P1). Adopt the secure/TLS-by-default posture from the start rather than repeating ES's open-cluster era. DLS/FLS can be skipped initially (see anti-goals, [§7](07-parity-matrix.md)).
- **Confidence:** High

## 3.10 Ingest pipelines

`PUT _ingest/pipeline/{id}` — ordered list of **processors** (`set`, `rename`, `grok`, `dissect`, `date`, `geoip`, `script`, `inference`...) run on ingest nodes before indexing; attach via `?pipeline=` or `index.default_pipeline`. Simulate API for testing. ([Ingest docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/ingest.html))

### F-024 — Ingest pipelines moved most Logstash-style transformation into the engine itself
- **Evidence:** [Ingest docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/ingest.html); processors run pre-indexing on ingest nodes, attach via `?pipeline=` or `index.default_pipeline`, with a simulate API for testing
- **Impact:** A small "ingest-pipeline-lite" (`set`/`rename`/`date`/`grok|dissect`/`drop` processors) covers most log-shipping transformation needs without requiring users to deploy a separate ETL tier — a P1 item for Lexum.
- **Confidence:** High

---

Next: [4. Query DSL Deep Dive](04-query-dsl.md)
