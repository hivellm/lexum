# 7. Feature Parity Matrix — Meilisearch vs Lexum

> Part of the [Meilisearch analysis](README.md) · Previous: [§6 Relevancy](06-relevancy.md) · Next: [§8 Execution Plan](08-execution-plan.md)

Lexum today: Tantivy 0.25 core, LQL, 39 REST endpoints + OpenAPI, CLI, snapshots/restore, index templates, API-key auth, rate limiting, query cache. Planned: sharding/replication, MCP/UMICP, Electron GUI, telemetry, aggregations.

Status legend — Lexum: **✅ implemented**, **🟡 partial**, **❌ missing**, **🔧 planned**. "P0/P1/P2 #N" references map to recommendations R-01..R-17 in the [execution plan](08-execution-plan.md).

| # | Meilisearch feature | Meilisearch status | Lexum status | Notes / recommendation |
|---|---|---|---|---|
| 1 | Full-text search (BM25-class) | ✅ stable | ✅ | Tantivy BM25 vs milli bucket-sort; Lexum fine here |
| 2 | Search-as-you-type (<50 ms, prefix search) | ✅ stable | 🟡 | Tantivy supports prefix queries; needs precomputed prefix strategy + `searchCutoffMs` budget |
| 3 | Typo tolerance (5/9 thresholds, first-letter ×2) | ✅ stable | ❌ | P0 #4 — wire Tantivy FuzzyTermQuery with Meilisearch defaults |
| 4 | Ranking rules / bucket sort, custom rules | ✅ stable | ❌ | Lexum uses BM25 only; adopt normalized score + explainability (P1 #6) rather than full bucket sort |
| 5 | Ranking score + score details + threshold | ✅ stable | ❌ | P1 #6 |
| 6 | Filtering DSL (`=,!=,>,<,TO,IN,EXISTS,NOT,AND,OR…`) | ✅ stable | 🟡 | LQL covers this for power users; add flat `filter` param on simple search (P0 #5 / avoid-list) |
| 7 | Faceting (distribution, stats, maxValuesPerFacet) | ✅ stable | 🟡 | Tantivy aggregations exist; expose Meilisearch-shaped `facets` param (P2 #14) |
| 8 | Facet search endpoint | ✅ stable | ❌ | P2 #14 |
| 9 | Sorting (query-time, sortableAttributes, sort-as-ranking-rule) | ✅ stable | 🟡 | Lexum sorts via LQL; add `sort` param + relevance/sort ordering control |
| 10 | Geosearch (`_geoRadius`, `_geoBoundingBox`, `_geoPoint`, GeoJSON) | ✅ stable | ❌ | P2 #15 |
| 11 | Multi-search (N queries, 1 request) | ✅ stable | ❌ | P1 #7 |
| 12 | Federated search (merged ranking, weights, remote) | ✅ stable (remote: v1.13+) | ❌ / 🔧 | P1 #7 — build as the foundation of planned sharding |
| 13 | Hybrid/vector search (embedders, semanticRatio, hannoy HNSW) | ✅ GA since v1.13 | ❌ | P2 #13 |
| 14 | Multimodal search (images) | 🧪 experimental (v1.16+) | ❌ | Defer |
| 15 | Chat / conversational (RAG) endpoint | 🧪 experimental (v1.15+) | 🔧 | Lexum's MCP protocol is the analogous play (P2 #16) |
| 16 | `/similar` recommendations | ✅ | ❌ | P2 #17, after vectors |
| 17 | Personalization (Cohere) | 🧪 experimental (v1.25+) | ❌ | Defer |
| 18 | API keys (scoped actions × indexes × expiry) | ✅ stable | 🟡 | Lexum has API keys; add scoping/expiry to match `/keys` |
| 19 | Tenant tokens (JWT search rules) | ✅ stable | ❌ | P1 #8 |
| 20 | Rate limiting | ❌ (cloud-level only) | ✅ | Lexum ahead here |
| 21 | Async task queue (statuses, filters, cancel atomic) | ✅ stable | ❌ | **P0 #1 — highest-impact gap** |
| 22 | Auto-batching of tasks | ✅ stable | ❌ | P0 #1 |
| 23 | `/batches` introspection | ✅ | ❌ | With P0 #1 |
| 24 | Task webhooks | ✅ | ❌ | P1 #11 |
| 25 | Snapshots (binary, scheduled + on-demand) | ✅ stable | ✅ | Lexum has repo-based snapshots — comparable or better |
| 26 | Dumps (version-portable logical export) | ✅ stable | ❌ | P1 #9 |
| 27 | Dumpless in-place upgrades | ✅ (v1.13+) | ❌ | Long-term; matters once Lexum has stable releases |
| 28 | Index swap (atomic alias-like) | ✅ stable | 🟡 | Lexum has templates; atomic swap for zero-downtime reindex worth adding |
| 29 | Index templates | ❌ (no equivalent) | ✅ | Lexum ahead (Elasticsearch heritage) |
| 30 | SQL-like query language | ❌ (by design) | ✅ | Lexum differentiator — keep, but not as the only door |
| 31 | Settings-as-resource with defaults | ✅ stable | 🟡 | Lexum has per-index config; reshape as GET/PATCH-able settings object (P0 #3) |
| 32 | Synonyms / stop words / distinct attribute | ✅ stable | ❌ | Include in settings work (P0 #3) |
| 33 | Multi-language tokenization (charabia: CJK, Arabic, Hebrew…) | ✅ stable | 🟡 | Tantivy has tokenizer ecosystem (lindera, jieba available as crates) — expose per-index/language config |
| 34 | Highlighting / cropping / match positions | ✅ stable | 🟡 | Tantivy SnippetGenerator; expose Meilisearch-shaped params (P0 #5) |
| 35 | `matchingStrategy` (last/all/frequency) | ✅ stable | ❌ | P1 #10 |
| 36 | Uniform error format with doc links | ✅ stable | 🟡 | P0 #2 |
| 37 | OpenAPI spec + Swagger | ✅ (+ llms.txt) | ✅ | Parity; consider adding llms.txt |
| 38 | Official SDKs (10+ languages) | ✅ | 🔧 planned | Prioritize JS + Python first, mirroring Meilisearch's adoption path |
| 39 | CLI / offline admin tool (`meilitool`) | ✅ | ✅ | Lexum CLI broader |
| 40 | Prometheus `/metrics` | 🧪 experimental | 🔧 planned | Fits Lexum telemetry phase |
| 41 | Experimental-features runtime gate | ✅ pattern | ❌ | P1 #12 |
| 42 | Sharding / replication | ✅ Enterprise-only (v1.19/v1.37+) | 🔧 planned | Ship it open source — key differentiation |
| 43 | S3-streaming snapshots | 🧪/Enterprise | ❌ | Natural extension of Lexum snapshot repositories |
| 44 | Query cache | ❌ (relies on OS page cache) | ✅ | Lexum ahead |
| 45 | Aggregations (ES-style buckets/metrics) | ❌ (facets only, by design) | 🔧 planned | Start with facet distribution/stats shape; full ES-style only on demand |
| 46 | Web GUI (mini-dashboard / Cloud UI) | ✅ (built-in mini dashboard) | 🔧 planned (Electron) | Consider a built-in web dashboard before Electron — lower friction |
| 47 | Document edit-by-function | 🧪 experimental | ❌ | Low priority |
| 48 | Dynamic search rules (pinning/merchandising) | 🧪 experimental (v1.41+) | ❌ | Watch — merchandising is an e-commerce differentiator |

## Findings

**F-038 — Of 48 surveyed Meilisearch-relevant capabilities, Lexum scores ✅ 8 · 🟡 9 · 🔧 6 · ❌ 25; the largest clusters of misses are the async task/write pipeline (rows 21–24) and the relevancy UX layer (rows 3–5, 35)**
- Evidence: matrix above, compiled from the sources indexed in the [README](README.md#source-index) against Lexum's current feature set (Tantivy 0.25 core, LQL, 39 REST endpoints + OpenAPI, CLI, snapshots/restore, index templates, API-key auth, rate limiting, query cache)
- Impact: Both miss clusters are P0/P1 in the [execution plan](08-execution-plan.md) precisely because everything else (federation, distribution, vectors) stacks on them.
- Confidence: high

**F-039 — Lexum is ahead of Meilisearch in five areas: rate limiting (Meilisearch: cloud-level only), index templates (no Meilisearch equivalent), SQL-like query language (Meilisearch omits one by design), query cache (Meilisearch relies on the OS page cache), and a broader CLI**
- Evidence: matrix rows 20, 29, 30, 39, 44
- Impact: These are Lexum's Elasticsearch-heritage differentiators to keep and market — with the caveat that LQL must not remain the only door (see F-025 and the avoid-list in the [execution plan](08-execution-plan.md)).
- Confidence: high
