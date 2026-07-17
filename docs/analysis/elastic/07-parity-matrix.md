# 7. Feature Parity Matrix and Anti-Goals

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-047…).

## 7.1 What "Elasticsearch parity" should mean

Guiding principle: **target the ES 7.10-era core API** (the OpenSearch-shared de-facto standard, F-004) for compatibility, plus modern vector/hybrid search; ignore the long tail.

### F-047 — Parity for Lexum = "drop-in for the 20% of the ES 7.10 API that 95% of clients use, plus 2026-grade vector/hybrid search, minus ES's legacy" — not endpoint-count parity
- **Evidence:** Synthesis of F-004 (7.10 API as de-facto standard), F-015 (`_bulk` leverage), F-041/F-042 (vector table stakes), and the anti-goals below
- **Impact:** This definition scopes the whole roadmap ([§8](08-execution-plan.md)). The distributed layer (sharding/replication) is the moat and the risk: adopt ES's proven concepts (seq-no replication, in-sync sets, quorum-free auto-configured elections, rollover-based growth) while skipping its retrofit pain. LQL + MCP is the forward-looking bet Elastic itself validates with ES|QL and its agent tooling.
- **Confidence:** High

## 7.2 Feature parity matrix

Status legend: **✅ Implemented** · **🟡 Partial** · **📋 Planned** · **❌ Missing** (not currently planned). Based on Lexum's current feature set (LQL, ~39 REST endpoints + OpenAPI, CLI, snapshots, index templates, API keys, rate limiting, query cache; planned: sharding/replication, MCP/UMICP, GUI, telemetry, aggregations).

| Area | Elasticsearch | Lexum status | Notes / priority |
|---|---|---|---|
| **Storage engine** | Lucene 10 segments | ✅ Tantivy 0.25 segments | Same architectural family |
| **Document CRUD** | index/get/update/delete, _mget, update/delete_by_query | ✅ (core CRUD via REST) | Verify optimistic concurrency + `refresh=wait_for` semantics (P0) |
| **Bulk API** | `_bulk` NDJSON, per-item errors | 🟡 | Exact ES `_bulk` wire compatibility is P0 for ecosystem |
| **Query language** | JSON Query DSL | 🟡 LQL (SQL-like) instead | Add ES-compatible `_search` DSL subset (P0); keep LQL as flagship |
| **Piped/SQL query language** | ES\|QL (GA 8.14), SQL | ✅ LQL | Lexum ahead of trajectory here; extend with `STATS`-style aggs |
| **Full-text queries** | match family, BM25 | ✅ (Tantivy BM25) | Per-field similarity tuning, `explain` — gaps |
| **Term-level queries** | term/range/exists/wildcard | ✅/🟡 | Confirm filter-context caching equivalent |
| **Aggregations** | bucket/metric/pipeline framework | 📋 Planned | Highest-leverage gap; Tantivy's ES-style agg module is the shortcut |
| **Pagination** | from/size, search_after, PIT, scroll | 🟡 from/size | Implement search_after + PIT; skip scroll (P0) |
| **Mappings/schema** | dynamic + explicit, 40+ field types | 🟡 | Need text/keyword multi-field convention + capped dynamic mapping |
| **Analyzers** | 30+ languages, ICU, synonyms, custom chains | 🟡 basic (Tantivy built-ins) | Biggest engine-level gap; add synonym + language packs incrementally |
| **Index templates** | Composable templates + components | ✅ | Align matching/priority semantics with ES v2 templates |
| **Data streams / rollover** | Data streams, rollover, ILM | ❌ | P1: rollover + delete-phase "ILM-lite" |
| **ILM tiers** | hot/warm/cold/frozen | ❌ | Later; needs distribution first |
| **Snapshots** | Repo-based incremental, SLM, searchable snapshots | ✅ backup/restore | Add scheduled snapshots (SLM-lite); object-storage repos |
| **Security: authn** | Realms, TLS-by-default | 🟡 API keys | TLS-by-default posture recommended (ES 8.0 lesson) |
| **Security: RBAC** | Roles, index privileges, DLS/FLS | ❌ (API keys only) | P1: index-pattern-scoped roles; skip DLS/FLS initially |
| **Rate limiting** | (not built-in; via proxy) | ✅ | Lexum ahead |
| **Query cache** | Node query cache, request cache | ✅ query cache | Match ES's filter-bitset caching semantics if possible |
| **Ingest pipelines** | Processor pipelines on ingest nodes | ❌ | P1-lite: set/rename/date/dissect processors |
| **Sharding/replication** | Primaries/replicas, seq-no replication, auto-rebalance | 📋 Planned | The critical path; adopt ES concepts ([§5](05-distributed-model.md)), design seq-nos from day 1 |
| **Cluster coordination** | Raft-like, quorum-free config (7.0+) | 📋 Planned | Use proven Raft crate; never expose quorum settings |
| **Consistency/durability** | Translog fsync-per-request, NRT refresh | 🟡 (single-node durability via Tantivy commits) | Must own WAL + refresh-interval semantics (F-012) |
| **Vector search** | dense_vector, HNSW, int8/BBQ quantization, hybrid RRF | ❌ (VecLite adjacent) | P1: table stakes for 2026; integrate ANN over Tantivy segments |
| **Semantic search** | ELSER, semantic_text, inference endpoints | ❌ | P2: external-inference-endpoint shape only (F-044) |
| **Runtime fields** | Painless schema-on-read (7.11+) | ❌ | Skip; LQL computed expressions instead |
| **Monitoring/telemetry** | _stats, _cat, Prometheus exporters ecosystem | 📋 Planned telemetry | Ship `_cluster/health` + `_cat` compatibility early (tooling expects it) |
| **UI** | Kibana | 📋 Planned GUI | Dev-tools-style console + index management first |
| **CLI** | none first-party (curl culture) | ✅ | Lexum ahead |
| **OpenAPI spec** | Published API specs (recent) | ✅ Swagger/OpenAPI | Lexum ahead on discoverability |
| **Agent protocols** | MCP server via Agent Builder (9.x era) | 📋 MCP + UMICP planned | Both converging; Lexum can be MCP-native first |
| **Cross-cluster search/replication** | CCS/CCR | ❌ | Not worth planning yet |
| **ML/anomaly detection** | X-Pack ML | ❌ | Anti-goal; delegate to external tools |

### F-048 — Lexum's largest compatibility gaps per the matrix: aggregations, ES-compatible `_search` DSL, `search_after`/PIT pagination, `_bulk` wire compatibility, RBAC, vector search, and the entire distribution layer
- **Evidence:** Parity matrix above (🟡/📋/❌ rows), cross-referenced against Lexum's current feature set (LQL, ~39 REST endpoints + OpenAPI, CLI, snapshots, templates, API keys, rate limiting, query cache)
- **Impact:** These gaps directly define P0/P1 in the [execution plan](08-execution-plan.md); everything else is polish or deliberately skipped.
- **Confidence:** High

### F-049 — Lexum is already *ahead* of ES on several axes: a shipped piped/SQL-style query language (LQL, where ES only reached GA with ES|QL in 8.14), first-party CLI, published OpenAPI spec, and built-in rate limiting
- **Evidence:** Parity matrix rows "Piped/SQL query language", "CLI", "OpenAPI spec", "Rate limiting"; [ES|QL GA](https://www.elastic.co/blog/whats-new-elastic-8-14-0)
- **Impact:** These are differentiation assets to preserve and market, not to dilute in pursuit of ES parity — the parity target is the ecosystem-facing surface, while LQL/CLI/OpenAPI remain the flagship developer experience.
- **Confidence:** High

## 7.3 Anti-goals: ES complexity traps NOT to replicate

1. **Mapping types** — ES spent 5 versions (5.x→8.0) removing multiple types per index; docs: [removal of types](https://www.elastic.co/guide/en/elasticsearch/reference/master/removal-of-types.html). Lexum: one schema per index, never anything type-like. (Already the case — keep it so.)
2. **Unbounded dynamic mapping / mapping explosion** — dynamic mapping of arbitrary JSON grows cluster state until masters die; ES bolted on `total_fields.limit: 1000` and the `flattened` type as mitigations. Lexum: strict-by-default or tightly capped dynamic mapping, and a flattened-style type early.
3. **Scroll API** — legacy, stateful, resource-pinning; superseded by `search_after`+PIT. Implement the modern pair only.
4. **The full `query_string` Lucene syntax** on user input — decades of parse-error and DoS pain; offer `simple_query_string` semantics and LQL instead.
5. **General-purpose scripting in the engine** (Groovy→sandboxed Painless saga, including remote-code-execution history pre-5.0) — huge security/complexity surface. Prefer a small expression language, not a scripting VM.
6. **API sprawl** — ES has hundreds of endpoints, many redundant (`_cat` duplicating JSON APIs, template v1 vs v2, `_xpack` remnants). Lexum's ~39 endpoints + OpenAPI is a feature; add compatibility aliases, not parallel systems.
7. **Types of node/tier proliferation before scale demands it** — ship with 2 roles (master-eligible/data or even single-role), add tiers when someone actually has warm data.
8. **Fixed 5-shard-style defaults** — default to 1 primary shard; make rollover the growth story, as ES itself concluded in 7.0.
9. **Multi-tenancy via one giant cluster** — ES's security model (DLS/FLS) grew baroque partly to serve this; small per-tenant clusters/indices with API-key scoping is simpler and matches Lexum's likely deployment profile.
10. **Joins** (`join` field / parent-child, nested-doc overuse) — the most misused ES features, with severe performance cliffs. Support `nested` eventually; skip parent-child.

### F-050 — Ten concrete ES complexity traps are identified as Lexum anti-goals; the two most dangerous are unbounded dynamic mapping (cluster-state death) and in-engine general-purpose scripting (pre-5.0 Groovy remote-code-execution history)
- **Evidence:** [Removal of types](https://www.elastic.co/guide/en/elasticsearch/reference/master/removal-of-types.html); ES's own mitigation history (`total_fields.limit`, `flattened` type, Groovy→Painless sandbox saga, 5→1 default-shard change in 7.0); full list above
- **Impact:** These anti-goals are load-bearing scope decisions: they keep Lexum's surface (~39 endpoints + OpenAPI) small by design and steer engineering effort away from features ES itself spent version-cycles removing or regretting.
- **Confidence:** High

---

Next: [8. Execution Plan](08-execution-plan.md)
