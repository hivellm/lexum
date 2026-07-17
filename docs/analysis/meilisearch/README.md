# Meilisearch — Deep Analysis for Lexum

> **Purpose**: A study of [Meilisearch](https://github.com/meilisearch/meilisearch) — architecture, features, API design, storage, and relevancy model — with concrete, prioritized recommendations for Lexum.
>
> **Research date**: 2026-07-16. Latest Meilisearch release at time of writing: **v1.49.0** (July 6, 2026) ([releases](https://github.com/meilisearch/meilisearch/releases)).
>
> All claims are sourced inline. Where Meilisearch documentation has evolved (e.g., ranking rules), the current documented behavior is stated and the historical form noted.

## Section Index

| § | File | Contents |
|---|---|---|
| §1 | [01-overview-positioning.md](01-overview-positioning.md) | Overview, licensing/editions, vs Elasticsearch/Typesense/Algolia (F-001..F-004) |
| §2 | [02-architecture.md](02-architecture.md) | Crate map, milli, index-scheduler, heed/LMDB, charabia, arroy→hannoy, layering (F-005..F-010) |
| §3 | [03-core-features.md](03-core-features.md) | Search-as-you-type, typo tolerance, filtering/faceting, sorting, geosearch, federation, hybrid/AI, tenant tokens, tasks, dumps/snapshots, experimental gate, sharding (F-011..F-023) |
| §4 | [04-api-design.md](04-api-design.md) | Route surface, error format, search params, DX comparison vs Elasticsearch (F-024..F-028) |
| §5 | [05-indexing-storage.md](05-indexing-storage.md) | LMDB single-writer, space amplification, 2024 indexer rewrite, contrast with Tantivy (F-029..F-033) |
| §6 | [06-relevancy.md](06-relevancy.md) | Ranking-rules bucket sort, default order, normalized score, supporting machinery (F-034..F-037) |
| §7 | [07-parity-matrix.md](07-parity-matrix.md) | 48-feature Meilisearch vs Lexum parity matrix (F-038..F-039) |
| §8 | [08-execution-plan.md](08-execution-plan.md) | Phased plan: 17 recommendations (R-01..R-17, P0/P1/P2) + 7 anti-patterns (A-01..A-07) |

Findings are numbered **F-001..F-039** globally across the analysis, in reading order.

## Executive Summary

Meilisearch (Rust, since 2018) wins the instant, user-facing search niche on three pillars — sub-50 ms performance, zero-config relevancy, and developer experience — while deliberately refusing Elasticsearch's scope (query DSL, aggregations, cluster management). Its single most transferable asset for Lexum is not its storage (LMDB single-writer, ~26× space amplification, years spent engineering around one write transaction) but its **coordination layer**: every write is an asynchronous task flowing through one scheduler, enabling auto-batching, crash recovery, dumps, webhooks, and — via remote federated search — its entire Enterprise sharding story, layered on the single-node engine without rewriting it. Lexum's Tantivy segment model is already the better storage foundation for distributed goals; the gaps are the async task queue, the relevancy UX layer (typo tolerance, normalized/explainable scores, matchingStrategy), and a no-query-language simple-search path alongside LQL. Meilisearch gates sharding/replication behind its Enterprise/BSL edition — Lexum's clearest open-source differentiation opportunity. Of 48 surveyed capabilities, Lexum implements 8, partially covers 9, plans 6, and misses 25 (see [§7](07-parity-matrix.md)); the [execution plan](08-execution-plan.md) phases 17 recommendations (P0/P1/P2) plus 7 anti-patterns to avoid.

### Top findings by impact

| # | Impact | Theme | Finding | Evidence |
|---|---|---|---|---|
| F-010 | Highest-impact gap | [§2 Architecture](02-architecture.md) | Lexum lacks an async task/queue layer between REST and Tantivy, while all Meilisearch writes are asynchronous tasks through the index-scheduler | [DeepWiki](https://deepwiki.com/meilisearch/meilisearch), [async docs](https://www.meilisearch.com/docs/learn/async/asynchronous_operations) |
| F-033 | Central conclusion | [§5 Storage](05-indexing-storage.md) | The transferable asset is Meilisearch's coordination layer (task queue, batching, transactional settings), not its LMDB storage; Tantivy's segments are the better foundation | [indexer rewrite](https://blog.kerollmops.com/meilisearch-is-too-slow), [storage docs](https://www.meilisearch.com/docs/learn/engine/storage) |
| F-001 | Strategic opening | [§1 Positioning](01-overview-positioning.md) | Meilisearch gates sharding/replication and S3-streaming snapshots behind Enterprise/BSL 1.1 (sharding: Enterprise v1.37+) | [sharding docs](https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview) |
| F-023 | Distribution blueprint | [§3 Features](03-core-features.md) | Sharding was layered on top of the single-node engine via remote federated search — the engine was never rewritten into a distributed system | [sharding overview](https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview), [blog](https://www.meilisearch.com/blog/horizontal-scaling-with-sharding) |
| F-036 | Prerequisite for federation + hybrid | [§6 Relevancy](06-relevancy.md) | Normalized 0–1 ranking scores (with per-rule details and threshold) are what make federated merging and hybrid keyword+semantic fusion possible — raw BM25 scores are not comparable across indexes | [ranking rules](https://www.meilisearch.com/docs/learn/relevancy/ranking_rules), [search API](https://www.meilisearch.com/docs/reference/api/search) |
| F-025 | API design constraint | [§4 API](04-api-design.md) | The entire search request is 28 flat, orthogonal parameters; the only query "language" is the `filter` string — LQL must not be Lexum's only door | [search API](https://www.meilisearch.com/docs/reference/api/search) |
| F-012 | Free relevance tuning | [§3 Features](03-core-features.md) | Typo-tolerance defaults (0 typos <5 chars, 1 at 5–8, 2 at 9+, first-letter typo counts double) encode years of tuning; wire Tantivy `FuzzyTermQuery` with these exact numbers | [typo tolerance settings](https://www.meilisearch.com/docs/learn/relevancy/typo_tolerance_settings) |
| F-030 | Anti-pattern evidence | [§5 Storage](05-indexing-storage.md) | ~26× space amplification (8.6 MB dataset → 224 MB on disk) and no space reclamation are the documented cost of LMDB + precompute-everything | [storage docs](https://www.meilisearch.com/docs/learn/engine/storage) |
| F-038 | Parity snapshot | [§7 Parity](07-parity-matrix.md) | Of 48 capabilities: Lexum ✅ 8 · 🟡 9 · 🔧 6 · ❌ 25; largest miss clusters are the async write pipeline and the relevancy UX layer | [parity matrix](07-parity-matrix.md) |
| F-016 | Contract to improve on | [§3 Features](03-core-features.md) | Federated multi-search errors are all-or-nothing (no partial results) — a weakness Lexum should fix with per-query error objects | [multi-search API](https://www.meilisearch.com/docs/reference/api/multi_search) |

### Execution plan at a glance ([§8](08-execution-plan.md))

- **Phase 1 (P0, foundational)**: R-01 async task queue for all writes · R-02 uniform error object with doc links · R-03 settings as a REST resource · R-04 typo tolerance with Meilisearch's defaults · R-05 search response shaping (highlight/crop/pagination).
- **Phase 2 (P1, builds on P0)**: R-06 normalized ranking score + details · R-07 `/multi-search` with federation (the future scatter-gather layer) · R-08 tenant tokens (JWT search rules) · R-09 version-portable dumps · R-10 `matchingStrategy` · R-11 task webhooks · R-12 experimental-features gate.
- **Phase 3 (P2, strategic)**: R-13 hybrid vector search (embedders as settings; evaluate hannoy) · R-14 facet search + stats · R-15 geosearch sugar · R-16 chat/RAG via MCP · R-17 `/similar`.
- **Avoid (A-01..A-07)**: LMDB/single-writer storage · paid-edition distribution · all-or-nothing multi-search errors · LQL as the only door · ES-style aggregations first · unbounded task queues · default-on telemetry.

## Source Index

- Meilisearch GitHub repo & releases: https://github.com/meilisearch/meilisearch · https://github.com/meilisearch/meilisearch/releases
- What is Meilisearch: https://www.meilisearch.com/docs/learn/getting_started/what_is_meilisearch
- Architecture (crates) — DeepWiki: https://deepwiki.com/meilisearch/meilisearch
- Storage (LMDB): https://www.meilisearch.com/docs/learn/engine/storage
- Indexer rewrite: https://blog.kerollmops.com/meilisearch-is-too-slow · https://github.com/meilisearch/meilisearch/pull/4900
- Vector store: https://blog.kerollmops.com/from-trees-to-graphs-speeding-up-vector-search-10x-with-hannoy · https://www.meilisearch.com/blog/3xfaster-vector-store · https://blog.kerollmops.com/meilisearch-indexes-embeddings-7x-faster-with-binary-quantization · https://github.com/meilisearch/meilisearch/pull/5767
- heed: https://github.com/meilisearch/heed · charabia: https://github.com/meilisearch/charabia
- Ranking rules: https://www.meilisearch.com/docs/learn/relevancy/ranking_rules
- Typo tolerance: https://www.meilisearch.com/docs/learn/relevancy/typo_tolerance_settings
- Filters: https://www.meilisearch.com/docs/learn/filtering_and_sorting/filter_expression_reference
- Geosearch: https://www.meilisearch.com/docs/learn/filtering_and_sorting/geosearch
- Async tasks: https://www.meilisearch.com/docs/learn/async/asynchronous_operations
- Tenant tokens: https://www.meilisearch.com/docs/learn/security/multitenancy_tenant_tokens
- Snapshots vs dumps: https://www.meilisearch.com/docs/learn/data_backup/snapshots_vs_dumps
- AI search: https://www.meilisearch.com/docs/learn/ai_powered_search/getting_started_with_ai_search
- API references: https://www.meilisearch.com/docs/reference/api/overview · /search · /multi_search · /settings · /experimental_features · errors: https://www.meilisearch.com/docs/reference/errors/overview
- Changelog / milestones: https://www.meilisearch.com/docs/changelog/changelog
- Sharding: https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview · https://www.meilisearch.com/blog/horizontal-scaling-with-sharding · https://www.meilisearch.com/blog/sharding-replication
- Comparisons: https://www.meilisearch.com/docs/resources/comparisons/elasticsearch · https://www.meilisearch.com/blog/meilisearch-vs-elasticsearch · https://www.meilisearch.com/blog/algolia-vs-typesense · https://typesense.org/typesense-vs-algolia-vs-elasticsearch-vs-meilisearch/
