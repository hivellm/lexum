# 8. Execution Plan for Lexum

> Part of the [Meilisearch analysis](README.md) · Previous: [§7 Parity Matrix](07-parity-matrix.md)

Context — Lexum today: Tantivy 0.25 core, LQL, 39 REST endpoints + OpenAPI, CLI, snapshots/restore, index templates, API-key auth, rate limiting, query cache. Planned: sharding/replication, MCP/UMICP, Electron GUI, telemetry, aggregations.

The recommendations below are phased by priority. Recommendation IDs R-01..R-17 correspond to the "P0 #1 … P2 #17" references used in the [parity matrix](07-parity-matrix.md). Each item cites the findings (F-NNN) that motivate it.

## Phase 1 (P0) — Adopt now: foundational, hard to retrofit

**R-01 — Async task queue for all writes (index-scheduler pattern).** Every mutating operation returns a `taskUid` immediately; statuses `enqueued/processing/succeeded/failed/canceled`; `GET /tasks` with filters; auto-batching of compatible consecutive tasks. This is *the* architectural keystone of Meilisearch — it enables batching (Tantivy commits are expensive, exactly like LMDB write transactions: batch many doc additions per commit), crash recovery, dumps, and later replication (a task log is already an operation log to replicate). Retrofit cost grows with every new write endpoint Lexum adds — do it before Phase 2 distribution. *(Motivated by F-006, F-010, F-020, F-031.)*

**R-02 — Uniform error object with documentation links.** `{ message, code, type, link }` on every error. Lexum has 39 endpoints; a consistent machine-readable error contract is cheap now, painful later. *(F-027.)*

**R-03 — Settings as a REST resource with defaults.** Meilisearch's per-index `settings` object (every knob GET/PATCH/resettable, documented defaults) should be Lexum's model rather than Elasticsearch's split between mappings, settings, and cluster state. Map: `searchableAttributes` (ordered = field boost), `filterableAttributes`, `sortableAttributes`, `displayedAttributes`, `rankingRules`, `stopWords`, `synonyms`, `typoTolerance`, `pagination.maxTotalHits`, `searchCutoffMs`. Lexum's index-templates feature can then stamp settings objects — same shape. *(F-026, F-037.)*

**R-04 — Typo tolerance with Meilisearch's exact defaults.** Tantivy has `FuzzyTermQuery` (Levenshtein automata) — wire it with the proven heuristics: 0 typos < 5 chars, 1 typo 5–8, 2 typos 9+, first-letter typo counts double, `disableOnWords/Attributes/Numbers`. Copying these numbers buys years of Meilisearch's relevance tuning for free. *(F-012.)*

**R-05 — Search response shaping in the core search endpoint.** Highlighting (`attributesToHighlight`, pre/post tags), cropping (`attributesToCrop`, `cropLength`, `cropMarker`), `attributesToRetrieve`, `showMatchesPosition`, and both pagination styles (`offset/limit` and `page/hitsPerPage`). These are table stakes for instant-search UIs and Tantivy's `SnippetGenerator` covers most of it. *(F-025.)*

## Phase 2 (P1) — Adopt next: high value, builds on Phase 1

**R-06 — Normalized ranking score + score details.** Expose `showRankingScore` (0–1 normalized) and `showRankingScoreDetails`. Even with BM25 underneath, normalize (e.g., against the theoretical max or top hit) — this unlocks `rankingScoreThreshold`, federated merging across indexes/shards, and hybrid fusion later. Score explainability as a query flag is a major DX differentiator vs Elasticsearch. *(F-036.)*

**R-07 — `/multi-search` with federation.** Lexum plans sharding; Meilisearch proved federation-first is the cheaper path: build `POST /multi-search` (N queries, one round trip), then federated merging (one ranked list, per-query `weight`), then `remote` targeting other Lexum nodes. Federation *is* the scatter-gather layer of Lexum's future distributed search — designing it as a public API first (as Meilisearch did) means the distribution work reuses a tested code path. *(F-016, F-023.)*

**R-08 — Tenant tokens (JWT search rules).** Lexum already has API keys; add backend-mintable JWTs signed with an API key, embedding per-index forced filters + expiry, honored by the search endpoints only. Zero server-side state; ideal for the SaaS use cases Lexum targets. *(F-019.)*

**R-09 — Dumps (logical export) distinct from snapshots.** Lexum has binary snapshots; add a version-portable logical dump (documents + settings + keys + tasks as JSON/NDJSON) for upgrades — Tantivy segment formats change between versions, so Lexum will need this the first time it bumps Tantivy. *(F-021.)*

**R-10 — `matchingStrategy` (`last`/`all`/`frequency`).** Trivial to implement over Tantivy BooleanQuery, huge for search-as-you-type result quality (never show zero results mid-typing). *(F-028.)*

**R-11 — Task webhooks.** Lexum's MCP/UMICP plans imply event-driven consumers; webhook-on-task-completion is the minimal version and matches Meilisearch. *(F-020.)*

**R-12 — Experimental-features gate.** A runtime `GET/PATCH /experimental-features` route. Lexum is alpha; shipping MCP, aggregations, vector search behind flags with telemetry is exactly how Meilisearch de-risked vector search (v1.3 experimental → v1.13 GA). *(F-022.)*

## Phase 3 (P2) — Strategic: differentiators, larger efforts

**R-13 — Hybrid vector search.** Meilisearch's AI-native pivot is its growth engine. For Lexum: embedders as index settings (start with `rest` + `userProvided` + Ollama — the generic REST embedder alone covers every provider), `documentTemplate`, embedding cache keyed by doc hash, `hybrid: { semanticRatio }` fusion over normalized scores. Vector store options in Rust: hannoy itself (LMDB-based, MIT), usearch, or Tantivy-adjacent HNSW — evaluate hannoy first since it's proven at Meilisearch scale. *(F-004, F-009, F-017, F-018.)*

**R-14 — Facet search endpoint + facet stats.** Search-as-you-type over facet values, and min/max stats for numeric facets in search responses — cheap with Tantivy's aggregations and vital for e-commerce UIs. *(F-013.)*

**R-15 — Geosearch sugar.** Tantivy lacks native geo; implement `_geo`-style filters (`_geoRadius`, `_geoBoundingBox`) and distance sort via indexed lat/lng fast fields + Haversine post-filtering, exposing Meilisearch-compatible syntax in LQL and the simple search API. *(F-015.)*

**R-16 — Chat/conversational search via MCP.** Meilisearch built `/chats` (RAG over indexes, v1.15). Lexum's planned MCP protocol is the better-generalized version of the same idea — prioritize MCP with a built-in "search tool" definition; that positions Lexum for AI agents the way `/chats` positions Meilisearch. *(F-004.)*

**R-17 — `/similar` (more-like-this) endpoint** once vectors exist. *(Depends on R-13.)*

## What to avoid (anti-patterns observed in Meilisearch)

**A-01 — Don't adopt LMDB/single-writer storage.** Meilisearch spent years engineering around the single write transaction ([indexer rewrite](https://blog.kerollmops.com/meilisearch-is-too-slow)) and lives with unreclaimable disk space and ~26× space amplification ([storage docs](https://www.meilisearch.com/docs/learn/engine/storage)). Tantivy's segment model is the better foundation for Lexum's distributed goals. Adopt Meilisearch's *coordination* layer, not its storage. *(F-029, F-030, F-032, F-033.)*

**A-02 — Don't gate distribution behind a paid edition.** Sharding/replication being Enterprise-only ([sharding docs](https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview)) is Meilisearch's biggest community friction point — and Lexum's clearest open-source differentiation opportunity. *(F-001.)*

**A-03 — Don't fail multi-search entirely on one bad query.** Meilisearch returns no partial results if any federated query errors ([multi-search API](https://www.meilisearch.com/docs/reference/api/multi_search)); for a *distributed* engine, per-query error objects with partial results are the better contract (Elasticsearch got this right). *(F-016.)*

**A-04 — Don't let LQL become the only door.** Meilisearch's core lesson: the default path must require *no query language at all*. Keep `q` + `filter` + `sort` as flat parameters on a simple search endpoint; LQL is the power-user layer, like `filter` strings are Meilisearch's only "language". *(F-002, F-025.)*

**A-05 — Don't ship aggregations Elasticsearch-style.** Meilisearch deliberately offers only facet counts + stats and remains dramatically easier to use. Lexum's planned aggregations should start with the facet-distribution/stats model and add ES-style buckets only behind clear demand. *(F-013.)*

**A-06 — Avoid unbounded task queues.** Meilisearch's 10 GB cap with explicit errors beats silent degradation; copy the cap-and-error approach. *(F-020.)*

**A-07 — Avoid analytics/telemetry opt-out friction.** Meilisearch collects anonymous telemetry by default with `--no-analytics`; for a young project, default-on telemetry generates community distrust. Make Lexum's planned telemetry opt-in or first-run prompted.

## Sequencing rationale

Phase 1 items are foundational contracts (task queue, errors, settings, typo defaults, result shaping) whose retrofit cost grows with every endpoint and feature added. Phase 2 builds the normalization + federation layer that Lexum's planned distribution reuses directly (F-023: Meilisearch's own sharding is federation underneath). Phase 3 items are strategic differentiators that stack on Phases 1–2: hybrid search requires normalized scores (R-06) and settings-as-resource (R-03); `/similar` requires vectors (R-13). The parity matrix ([§7](07-parity-matrix.md), F-038) confirms the ordering: the two largest miss clusters — the async write pipeline and the relevancy UX layer — are exactly the Phase 1/2 items everything else stacks on.
