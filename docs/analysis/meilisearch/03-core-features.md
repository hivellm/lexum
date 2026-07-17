# 3. Core Features

> Part of the [Meilisearch analysis](README.md) · Previous: [§2 Architecture](02-architecture.md) · Next: [§4 API Design](04-api-design.md)

## 3.1 Search-as-you-type

Sub-50 ms search on every keystroke is the founding use case ([What is Meilisearch](https://www.meilisearch.com/docs/learn/getting_started/what_is_meilisearch)). Enablers:

- **Prefix search** computed at indexing time (setting `prefixSearch: "indexingTime"` by default; can be disabled) — the last query word is treated as a prefix, cheaply, because prefix posting lists are precomputed ([settings](https://www.meilisearch.com/docs/reference/api/settings)).
- **`searchCutoffMs`** — a per-index deadline (default 1500 ms) after which Meilisearch returns the best results found so far, guaranteeing bounded latency.
- **Facet search endpoint** (`POST /indexes/{uid}/facet-search`) for search-as-you-type *within facet values*.

**F-011 — Search-as-you-type latency is achieved by precomputing prefix posting lists at indexing time (`prefixSearch: "indexingTime"` default) plus a per-index `searchCutoffMs` deadline (default 1500 ms) that returns best-so-far results**
- Evidence: https://www.meilisearch.com/docs/reference/api/settings · https://www.meilisearch.com/docs/learn/getting_started/what_is_meilisearch
- Impact: Lexum's search-as-you-type story needs both a precomputed prefix strategy (Tantivy supports prefix queries but not precomputed prefix postings out of the box) and a bounded-latency budget mechanism.
- Confidence: high

## 3.2 Typo tolerance

Documented in detail at [typo tolerance settings](https://www.meilisearch.com/docs/learn/relevancy/typo_tolerance_settings):

- Uses a **prefix Levenshtein** algorithm; operations = insertion, deletion, substitution.
- Defaults: words of **1–4 chars: 0 typos**; **5–8 chars: 1 typo** (`minWordSizeForTypos.oneTypo: 5`); **9+ chars: 2 typos** (`twoTypos: 9`). Max 2 typos per word.
- **A typo on the first letter counts as two typos** (`caturday` does not match `saturday`) — a clever heuristic since first letters are rarely mistyped.
- Tunable: `enabled`, `minWordSizeForTypos`, `disableOnWords` (case-insensitive word list), `disableOnAttributes` (per-field opt-out), `disableOnNumbers` (exact-match numbers only, so `2024` doesn't match `2025`).

**F-012 — Meilisearch's typo tolerance defaults encode years of relevance tuning: 0 typos under 5 chars, 1 typo at 5–8, 2 typos at 9+, max 2 per word, first-letter typo counts as two typos, with per-word/per-attribute/numbers opt-outs**
- Evidence: https://www.meilisearch.com/docs/learn/relevancy/typo_tolerance_settings
- Impact: Lexum has no typo tolerance today; Tantivy's `FuzzyTermQuery` (Levenshtein automata) can be wired with these exact proven thresholds, buying Meilisearch's tuning for free (see [execution plan](08-execution-plan.md), R-04).
- Confidence: high

## 3.3 Ranking rules

Covered fully in [§6 Relevancy Model](06-relevancy.md).

## 3.4 Filtering and faceting

- Attributes must be declared in `filterableAttributes` before use (an indexing-time contract — filters are precomputed into facet databases).
- Filter language ([reference](https://www.meilisearch.com/docs/learn/filtering_and_sorting/filter_expression_reference)): `=`, `!=`, `>`, `<`, `>=`, `<=`, `TO` (range), `EXISTS`, `IS EMPTY`, `IS NULL`, `IN [...]`, `NOT`, `AND`, `OR`, parentheses, `STARTS WITH`, and experimental `CONTAINS` (SQL-LIKE-style substring). Equality is case-insensitive. Comparison operators work on numbers and (since v1.15) lexicographic strings.
- Filters accept **string syntax** (`"genres = horror AND director = 'Jordan Peele'"`) or **array syntax** (outer array = AND, inner arrays = OR) — the array form is trivially safe to build programmatically.
- **Faceting**: `facets` search parameter returns counts per value; settings control `maxValuesPerFacet` (default 100) and sort order; `facetsByIndex`/`mergeFacets` exist for federated search ([multi-search API](https://www.meilisearch.com/docs/reference/api/multi_search)).
- Facet **distribution + stats** (min/max for numeric facets) come back inside the search response — no separate aggregations framework.

**F-013 — Filtering is an indexing-time contract (`filterableAttributes` must be declared; filters are precomputed into facet databases), and the filter language is the only "query language" in Meilisearch; facet distribution + stats come back inside the search response with no separate aggregations framework**
- Evidence: https://www.meilisearch.com/docs/learn/filtering_and_sorting/filter_expression_reference · https://www.meilisearch.com/docs/reference/api/multi_search
- Impact: Confirms a facet-distribution/stats model (not ES-style aggregations) is sufficient for the site-search use case; Lexum's planned aggregations should start with this shape. The dual string/array filter syntax (array form is injection-safe to build programmatically) is worth copying.
- Confidence: high

## 3.5 Sorting

Query-time `sort` parameter (e.g., `price:asc`) on attributes declared in `sortableAttributes`. Sort is *a ranking rule* — its position in the ranking rules array decides whether relevance or the sort dominates. This is an elegant resolution of the "relevance vs sort" tension.

**F-014 — Sort is implemented as a ranking rule: its position in the ranking-rules array decides whether relevance or the sort dominates**
- Evidence: https://www.meilisearch.com/docs/learn/relevancy/ranking_rules · https://www.meilisearch.com/docs/reference/api/settings
- Impact: An elegant resolution of the "relevance vs sort" tension that avoids ES-style boosting arithmetic; a model Lexum should expose when adding a simple `sort` parameter.
- Confidence: high

## 3.6 Geosearch

([geosearch docs](https://www.meilisearch.com/docs/learn/filtering_and_sorting/geosearch)):

- Reserved `_geo` field: `{ "lat": ..., "lng": ... }`; malformed `_geo` fails indexing.
- Filters: `_geoRadius(lat, lng, meters)`, `_geoBoundingBox([lat, lng], [lat, lng])`.
- Sort: `_geoPoint(lat, lng):asc` (distance ordering); results include `_geoDistance`.
- Requires `_geo` in `filterableAttributes` / `sortableAttributes`.
- Newer versions support **GeoJSON** for complex geometries (polygons).

**F-015 — Geosearch is a small, reserved-field convention (`_geo` with `_geoRadius`/`_geoBoundingBox` filters and `_geoPoint` distance sort, `_geoDistance` in results), recently extended with GeoJSON polygons**
- Evidence: https://www.meilisearch.com/docs/learn/filtering_and_sorting/geosearch
- Impact: Tantivy lacks native geo; Lexum can replicate this exact surface via indexed lat/lng fast fields + Haversine post-filtering (see [execution plan](08-execution-plan.md), R-15).
- Confidence: high

## 3.7 Multi-search and federation

`POST /multi-search` ([API reference](https://www.meilisearch.com/docs/reference/api/multi_search)) has two modes:

- **Non-federated** (`federation: null`): N queries in one HTTP round trip, N separate result lists. (Introduced v1.1.)
- **Federated** (v1.10+, [changelog](https://www.meilisearch.com/docs/changelog/changelog)): results from all queries **merged and ranked together** into one list. Federation object supports `limit/offset`, `facetsByIndex`, `mergeFacets`, `distinct` across queries. Per-query `federationOptions.weight` is a multiplicative score factor; `federationOptions.remote` routes a query to another instance — **remote federated search** (v1.13) is the building block of Meilisearch's distributed story.
- Errors are all-or-nothing: any failing query fails the whole request (no partial results).

**F-016 — Remote federated search (v1.13, `federationOptions.remote`) is the building block of Meilisearch's entire distributed story; federation errors are all-or-nothing (any failing query fails the whole request, no partial results)**
- Evidence: https://www.meilisearch.com/docs/reference/api/multi_search · https://www.meilisearch.com/docs/changelog/changelog
- Impact: Federation-first is the cheaper path to distribution and Lexum should follow it — but with per-query error objects and partial results (Elasticsearch's contract), avoiding Meilisearch's all-or-nothing weakness.
- Confidence: high

## 3.8 Hybrid / vector / AI search

([AI search docs](https://www.meilisearch.com/docs/learn/ai_powered_search/getting_started_with_ai_search), [changelog](https://www.meilisearch.com/docs/changelog/changelog)):

- Timeline: experimental vector store (v1.3, 2023) → embedders and hybrid search matured → **GA by default in v1.13**; binary quantization (v1.11) cut vector DB size up to 10×; **hannoy** HNSW store stabilized ~v1.37.
- **Embedders** are an *index setting* — Meilisearch calls the embedding API for you at indexing time: OpenAI, HuggingFace (local inference), Ollama, Cohere, Mistral, Google Gemini, Cloudflare Workers AI, Voyage, Jina, AWS Bedrock, a generic **REST embedder** for any API, and `userProvided` (you supply raw vectors).
- **`documentTemplate`** (Liquid syntax) controls what text gets embedded per document — short, relevant excerpts beat whole documents.
- Embeddings are **cached**: only new/changed documents are re-embedded on subsequent indexing.
- **Hybrid query**: `q` + `hybrid: { embedder, semanticRatio }` — `semanticRatio: 0.0` = pure keyword, `1.0` = pure semantic; results merged by normalized ranking score.
- Related AI features: `POST /indexes/{uid}/similar` (recommendations by document), **chat completions** (v1.15 "chat with your indexes" — a built-in RAG/conversational endpoint), **multimodal** embeddings (v1.16, image+text via `media` search param), **search personalization** via Cohere (v1.25, experimental).

**F-017 — Embedders are an index setting: Meilisearch calls the embedding API for you at indexing time (12+ providers plus a generic REST embedder and `userProvided`), with a Liquid `documentTemplate` controlling embedded text and an embedding cache so only new/changed documents are re-embedded**
- Evidence: https://www.meilisearch.com/docs/learn/ai_powered_search/getting_started_with_ai_search
- Impact: This "engine owns the embedding calls" design is the key to Meilisearch's AI-native DX; the generic REST embedder alone covers every provider — the pattern Lexum should start with (see [execution plan](08-execution-plan.md), R-13).
- Confidence: high

**F-018 — Hybrid search fuses keyword and semantic results via a single `semanticRatio` knob (0.0 = pure keyword, 1.0 = pure semantic), merging on the normalized ranking score; vector search went v1.3 experimental → v1.13 GA, with binary quantization (v1.11) cutting vector DB size up to 10×**
- Evidence: https://www.meilisearch.com/docs/learn/ai_powered_search/getting_started_with_ai_search · https://www.meilisearch.com/docs/changelog/changelog · https://blog.kerollmops.com/meilisearch-indexes-embeddings-7x-faster-with-binary-quantization
- Impact: Hybrid fusion requires normalized scores (see F-036) — a prerequisite Lexum must build first; the experimental→GA path is also the governance model to copy (F-022).
- Confidence: high

## 3.9 Multi-tenancy: tenant tokens

([tenant tokens docs](https://www.meilisearch.com/docs/learn/security/multitenancy_tenant_tokens)):

- **JWTs generated in your backend, signed with a Meilisearch API key** — no server round trip needed to mint them.
- Payload embeds `searchRules`: per-index **forced filters** (e.g., `user_id = 123`) automatically applied to every search made with the token; plus `expiresAt`.
- Restriction applies to the **search endpoint only** — admin operations still need real API keys.
- The layered model: master key → scoped API keys (`/keys` route: actions × indexes × expiry) → tenant tokens (per end-user).

**F-019 — Tenant tokens are backend-minted JWTs signed with a Meilisearch API key (no server round trip), embedding per-index forced filters (`searchRules`) and expiry, honored by the search endpoint only; the security model is layered: master key → scoped API keys (actions × indexes × expiry) → tenant tokens**
- Evidence: https://www.meilisearch.com/docs/learn/security/multitenancy_tenant_tokens
- Impact: Zero-server-side-state multi-tenancy is ideal for SaaS use cases Lexum targets; Lexum already has API keys, so adding JWT search rules is an incremental step (see [execution plan](08-execution-plan.md), R-08).
- Confidence: high

## 3.10 Tasks / async API

([async operations docs](https://www.meilisearch.com/docs/learn/async/asynchronous_operations)):

- **Every write is asynchronous**: document add/update/delete, index create/update/delete/swap, settings changes, dumps, snapshots — the API returns instantly with a `taskUid`.
- Statuses: `enqueued → processing → succeeded | failed | canceled`. Failed tasks make **no changes** (transactional).
- **Auto-batching**: consecutive tasks targeting the same index, same type, same content-type are merged into one batch transparently (order preserved). Inspectable via `/batches`.
- Strict FIFO with priority exceptions (task cancelation, upgrades, task deletion, compaction jump the queue).
- Task queue DB capped (~10 GB) — beyond that, writes error with `no_space_left_on_device` until old tasks are deleted.
- Cancelation is **atomic** (all matched tasks or none); finished tasks can be bulk-deleted by filter.
- **Webhooks** notify external services when tasks finish.

**F-020 — Every Meilisearch write is asynchronous and transactional (failed tasks make no changes), with transparent auto-batching of compatible consecutive tasks, strict FIFO with priority exceptions, atomic cancelation, task webhooks, and a ~10 GB task-queue cap that errors with `no_space_left_on_device` rather than degrading silently**
- Evidence: https://www.meilisearch.com/docs/learn/async/asynchronous_operations
- Impact: The complete contract Lexum's task queue should implement (see F-010 and [execution plan](08-execution-plan.md), R-01); the cap-and-error approach beats unbounded queues.
- Confidence: high

## 3.11 Dumps and snapshots

([snapshots vs dumps](https://www.meilisearch.com/docs/learn/data_backup/snapshots_vs_dumps)):

| | Snapshot | Dump |
|---|---|---|
| Nature | Exact copy of `data.ms` (LMDB files) | Portable logical export ("blueprint") |
| Version compat | Same Meilisearch version only | Import older-version dumps into newer versions |
| Restore speed | Fast (data already indexed) | Slow (full re-indexing) |
| Creation | Scheduled at launch (`--schedule-snapshot=86400`) or on demand (v1.5+) | On demand via `POST /dumps` |
| Use case | Periodic backup / disaster recovery | Version migration |

S3-streaming snapshots exist (Enterprise / experimental, v1.25+). v1.13 also added **dumpless upgrades** (in-place DB migration between versions), reducing the need for dumps.

**F-021 — Meilisearch maintains two distinct backup primitives: binary snapshots (fast restore, same-version only) and version-portable logical dumps (slow re-index, cross-version migration); v1.13 added dumpless in-place upgrades**
- Evidence: https://www.meilisearch.com/docs/learn/data_backup/snapshots_vs_dumps
- Impact: Lexum has binary snapshots but no logical dump; it will need one the first time it bumps Tantivy (segment formats change between versions) — see [execution plan](08-execution-plan.md), R-09.
- Confidence: high

## 3.12 Experimental features pattern

A runtime toggle API — `GET/PATCH /experimental-features` — gates unstable functionality per instance ([experimental features API](https://www.meilisearch.com/docs/reference/api/experimental_features)). Current flags include: `metrics` (Prometheus `/metrics`), `logsRoute`, `editDocumentsByFunction` (edit docs via a function/expression), `containsFilter`, `network` (distributed features), `compositeEmbedders`, `chatCompletions`, `multimodal`, `foreignKeys` (document hydration across indexes), `dynamicSearchRules`, `getTaskDocumentsRoute`, `taskQueueCompactionRoute`, `renderRoute` (template preview), and others. Other experimental features are enabled by CLI flags/env vars instead when they must be set at startup.

This is an excellent governance pattern: features ship early behind flags, get telemetry and feedback, then stabilize (vector search followed exactly this path: v1.3 experimental → v1.13 GA).

**F-022 — Meilisearch gates unstable features behind a runtime `GET/PATCH /experimental-features` API (plus CLI flags for startup-time features), shipping early, gathering telemetry, then stabilizing — vector search followed exactly this path (v1.3 experimental → v1.13 GA)**
- Evidence: https://www.meilisearch.com/docs/reference/api/experimental_features · https://www.meilisearch.com/docs/changelog/changelog
- Impact: A governance pattern Lexum (alpha) should adopt for shipping MCP, aggregations, and vector search safely (see [execution plan](08-execution-plan.md), R-12).
- Confidence: high

## 3.13 Distribution (new): sharding and replication

Historically single-node; as of v1.19 (automatic document sharding) and v1.37 (replicated sharding), Meilisearch Enterprise supports spreading one index across multiple instances ("remotes", each holding named shards) with redundancy, built on top of **remote federated search** as the query-fanout mechanism; v1.30 added dynamic topology changes without restart ([changelog](https://www.meilisearch.com/docs/changelog/changelog), [sharding overview](https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview), [blog](https://www.meilisearch.com/blog/horizontal-scaling-with-sharding)).

Architecturally notable: distribution was layered **on top of** the single-node engine via federation, not by rewriting the engine into a distributed system.

**F-023 — Meilisearch's sharding (v1.19 automatic document sharding, v1.37 replicated sharding, v1.30 dynamic topology) was layered on top of the single-node engine using remote federated search as the query-fanout mechanism — the engine was never rewritten into a distributed system**
- Evidence: https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview · https://www.meilisearch.com/blog/horizontal-scaling-with-sharding · https://www.meilisearch.com/docs/changelog/changelog
- Impact: Direct blueprint for Lexum's planned sharding: build multi-search/federation as a public API first, then distribution reuses that tested code path (see [execution plan](08-execution-plan.md), R-07).
- Confidence: high
