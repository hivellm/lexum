# 8. Execution Plan

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-051…). This file turns the parity analysis into a phased plan, grounded in Tantivy's real advantages and limitations.

## 8.1 Foundation check: Tantivy vs Lucene — Lexum's natural advantages and limitations

### Advantages

- **No JVM**: no GC pauses, no heap-vs-page-cache split-brain memory tuning (ES's "50% heap, ≤ ~30GB compressed-oops" folklore), small static binaries, fast cold start — great for edge/embedded/sidecar deployments ES can't touch.
- **Raw speed**: Tantivy consistently ranks at/near the top of lexical-search benchmarks and generally beats Lucene on indexing/query throughput in like-for-like tests ([Tantivy README](https://github.com/quickwit-oss/tantivy), [search-benchmark-game](https://tantivy-search.github.io/bench/)).
- **Memory safety + fearless concurrency** for the distributed layer Lexum is about to write — the hardest ES bugs (replication races, recovery edge cases) are exactly where Rust helps.
- **Modern core**: columnar fast fields, built-in ES-style aggregations, SIMD-friendly design; proven at scale by **Quickwit** (log search on object storage, built on Tantivy).
- **No legacy API debt**: Lexum can be "ES 7.10-compatible where it counts" with a clean modern core, like OpenSearch without the Java inheritance.

### F-051 — Tantivy gives Lexum structural advantages Lucene/ES cannot match: no JVM (no GC pauses, no heap folklore, small static binaries, fast cold start), top-tier benchmark speed, Rust memory safety for the distributed layer, and zero legacy API debt
- **Evidence:** [Tantivy README](https://github.com/quickwit-oss/tantivy), [search-benchmark-game](https://tantivy-search.github.io/bench/); Quickwit as at-scale proof
- **Impact:** Edge/embedded/sidecar deployments are a market ES structurally cannot serve; and Rust's safety applies exactly where ES's hardest bugs lived (replication races, recovery edge cases). These advantages should shape positioning, not just implementation.
- **Confidence:** High

### Limitations (be honest in planning)

- **Analyzer/language ecosystem**: Lucene's 30+ language analyzers, ICU, Kuromoji, Nori, phonetic filters, etc. dwarf Tantivy's built-ins (basic stemmers via rust-stemmers, ngram, custom tokenizer API; CJK needs third-party crates like lindera/tantivy-jieba).
- **No built-in vector/HNSW index** in Tantivy 0.25 — Lucene has native HNSW.
- **Fewer query primitives**: no full equivalents of span/interval queries, percolator, join/nested docs; Tantivy's feature list is deliberately smaller ([features table in README](https://github.com/quickwit-oss/tantivy)).
- **No translog / replication primitives**: Lucene doesn't have them either — but ES has 15 years of hardening on top. Everything in [§2.4–§2.5](02-architecture.md) and [§5](05-distributed-model.md) is Lexum's to build and test.
- **Smaller bus factor/community** than Lucene; Tantivy evolves with Quickwit's needs.

### F-052 — Multilingual analysis is Lexum's biggest raw-engine gap: Lucene's 30+ language analyzers, ICU, Kuromoji, Nori, and phonetic filters dwarf Tantivy's built-ins (rust-stemmers basics, ngram, custom tokenizer API; CJK via third-party lindera/tantivy-jieba)
- **Evidence:** [Tantivy README features table](https://github.com/quickwit-oss/tantivy); [ES analysis docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/analysis.html) (F-020)
- **Impact:** Plan incremental language packs (synonyms first, then per-language stemming/tokenization) rather than promising ES-level analyzer breadth; do not market multilingual parity Lexum cannot deliver.
- **Confidence:** High

### F-053 — Tantivy 0.25 has no built-in vector/HNSW index (Lucene does); Lexum must integrate its own ANN layer and solve vector-index-in-segment-lifecycle itself
- **Evidence:** [Tantivy README](https://github.com/quickwit-oss/tantivy); Lucene-native HNSW per [dense_vector docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/dense-vector)
- **Impact:** Vector support (P1) is an integration project, not a wiring task: candidate ANN layers are VecLite (sibling project) or HNSW crates, and the hard part is binding vector indexes to Tantivy's segment lifecycle (build/merge/delete).
- **Confidence:** High

### F-054 — All durability/replication machinery (translog, seq-nos, recovery — everything in §2.4–2.5 and §5) is Lexum's to build and test; Jepsen-style testing is recommended, and Tantivy's smaller community/bus factor is a real dependency risk
- **Evidence:** Neither Lucene nor Tantivy ships these primitives; ES has 15 years of hardening on top ([Jepsen: Elasticsearch](https://aphyr.com/posts/317-jepsen-elasticsearch)); Tantivy evolves with Quickwit's needs
- **Impact:** The distributed layer is the single largest engineering risk in the plan below — budget for correctness testing (Jepsen-style fault injection) as a first-class deliverable, not an afterthought. Tantivy also lacks some query primitives (span/interval, percolator, join/nested), which constrains long-tail DSL parity.
- **Confidence:** High

## 8.2 Phased roadmap

Guiding principle (F-047): target the ES 7.10-era core API for compatibility, plus modern vector/hybrid search; ignore the long tail.

### Phase P0 — the compatibility kernel (what clients/tools actually call)

1. **`_bulk`** with exact NDJSON semantics and per-item errors (F-015) — unlocks every shipper.
2. **`_search` with the core Query DSL**: `bool` (+ correct query/filter context, F-025/F-026), `match`/`multi_match`/`match_phrase`, `term`/`terms`/`range`/`exists`/`prefix`/`wildcard`, `ids`; `from/size`, `sort`, `_source`, `track_total_hits`, `highlight`.
3. **Document CRUD** + `_mget`, `?refresh=true|wait_for`, optimistic concurrency (seq_no/primary_term or a Lexum equivalent) (F-014).
4. **Mappings**: explicit + dynamic mapping, `text`/`keyword` multi-field convention (F-027), `_mapping` GET/PUT, `_analyze`.
5. **Aggregations**: `terms`, `date_histogram`, `histogram`, `range`, `filters` buckets; `min/max/sum/avg/stats/cardinality/percentiles/top_hits` metrics; nesting. (Tantivy's ES-modeled aggregation module is the shortcut, F-017.)
6. **`search_after` + PIT pagination** (skip scroll, F-018).
7. **Cluster/ops surface Kibana-style tooling expects**: `_cluster/health`, `_cat/indices`, `_cat/shards`, `_stats`, `_nodes`.

### Phase P1 — production platform

8. **Roles/RBAC** over existing API keys (index-pattern privileges) (F-023).
9. **ILM-lite**: rollover by size/age + delete phase + data-stream-style aliases — the 80% of ILM that log retention needs (F-021). Lexum already has templates and snapshots, the two prerequisites.
10. **Ingest-pipeline-lite**: `set`/`rename`/`date`/`grok|dissect`/`drop` processors (F-024).
11. **`dense_vector` + kNN query + hybrid RRF** (F-040/F-042) — table stakes for 2026 search engines; quantization (at least int8) from day one (F-041).
12. **`_msearch`, `_count`, `_explain`** (even approximate), `_validate/query`.

### Phase P2 — differentiation and polish

13. **`semantic_text`-shaped field** with external inference endpoints (F-044).
14. **ES|QL-inspired piped extensions to LQL** (aggregations in LQL, `STATS ... BY`) — LQL is already the right bet (F-030).
15. **SLM** (scheduled snapshots), searchable-snapshot-style cold tier, downsampling — only after distribution works (F-022).
16. **Percolate-style reverse search** (great for alerting; Tantivy has building blocks).

### F-055 — The P0 compatibility kernel (items 1–7) is deliberately small: seven work areas make Lexum usable by the existing ES ecosystem, before any distribution work ships
- **Evidence:** Phase P0 list above; each item traces to a finding (F-014, F-015, F-017, F-018, F-025–F-027) and to a 🟡/📋 row in the [parity matrix](07-parity-matrix.md)
- **Impact:** Sequencing insight: ecosystem compatibility (P0) is achievable on the current single-node engine and should not wait for the distributed layer — while the distributed layer (sharding/replication, F-039/F-054) proceeds in parallel as the moat-and-risk track. P1 turns Lexum into a production platform; P2 is differentiation.
- **Confidence:** Medium

## 8.3 Strategic summary

Lexum should define parity as: **"drop-in for the 20% of the ES 7.10 API that 95% of clients use, plus 2026-grade vector/hybrid search, minus ES's legacy"** — not endpoint-count parity (F-047). The distributed layer (sharding/replication) is the moat and the risk: adopt ES's proven concepts (seq-no replication, in-sync sets, quorum-free auto-configured elections, rollover-based growth) while skipping its retrofit pain (F-039). LQL + MCP is the forward-looking bet Elastic itself validates with ES|QL and its agent tooling (F-030, F-046).

## Key sources

- Elastic blog: [Elasticsearch is Open Source. Again!](https://www.elastic.co/blog/elasticsearch-is-open-source-again) · [Security is now free](https://www.elastic.co/blog/security-for-elasticsearch-is-now-free) · [ES|QL GA (8.14)](https://www.elastic.co/blog/whats-new-elastic-8-14-0) · [9.0 release](https://www.elastic.co/blog/whats-new-elastic-platform-9-0-0) · [Runtime fields](https://www.elastic.co/blog/introducing-elasticsearch-runtime-fields) · [Removal of mapping types](https://www.elastic.co/blog/removal-of-mapping-types-elasticsearch)
- Elastic docs: [Query DSL](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html) · [Aggregations](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html) · [Translog](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html) · [ILM](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-lifecycle-management.html) · [dense_vector](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/dense-vector) · [kNN](https://www.elastic.co/docs/solutions/search/vector/knn) · [semantic_text](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/semantic-text) · [ELSER](https://www.elastic.co/docs/explore-analyze/machine-learning/nlp/ml-nlp-elser) · [ES|QL reference](https://www.elastic.co/docs/reference/query-languages/esql) · [Size your shards](https://www.elastic.co/guide/en/elasticsearch/reference/current/size-your-shards.html)
- Third-party: [Wikipedia: Elasticsearch](https://en.wikipedia.org/wiki/Elasticsearch) · [Wikipedia: OpenSearch](https://en.wikipedia.org/wiki/OpenSearch_(software)) · [Linux Foundation OpenSearch announcement](https://www.linuxfoundation.org/press/linux-foundation-announces-opensearch-software-foundation-to-foster-open-collaboration-in-search-and-analytics) · [InfoQ on AGPL](https://www.infoq.com/news/2024/09/elastic-open-source-agpl/) · [endoflife.date/elasticsearch](https://endoflife.date/elasticsearch) · [Jepsen: Elasticsearch](https://aphyr.com/posts/317-jepsen-elasticsearch) · [Tantivy](https://github.com/quickwit-oss/tantivy)
