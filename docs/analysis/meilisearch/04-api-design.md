# 4. API Design

> Part of the [Meilisearch analysis](README.md) · Previous: [§3 Core Features](03-core-features.md) · Next: [§5 Indexing and Storage](05-indexing-storage.md)

## Route surface

The full API is small and predictable ([API overview](https://www.meilisearch.com/docs/reference/api/overview)):

```
/indexes                     CRUD + swap-indexes
/indexes/{uid}/documents     add/replace (POST), add/update (PUT), get, delete,
                             fetch/delete by filter, edit-by-function (exp.)
/indexes/{uid}/search        POST (and GET) — the search endpoint
/indexes/{uid}/facet-search  search within facet values
/indexes/{uid}/similar       AI recommendations
/indexes/{uid}/settings      the whole settings object + one sub-route per setting
/multi-search                multiple queries / federation
/tasks                       list/filter, get, cancel, delete
/batches                     inspect auto-batched task groups
/keys                        API key management (master key only)
/dumps  /snapshots           data backup
/stats  /health  /version  /metrics  /logs
/experimental-features       runtime feature flags
/network                     distributed topology (exp.)
/webhooks  /chats  /export   (newer/experimental)
```

Conventions: Bearer `Authorization` header everywhere except `/health`; master key ≥ 16 bytes; documents accepted as **JSON, NDJSON, or CSV**.

## What makes it developer-friendly vs Elasticsearch

1. **Schemaless by default.** Push a JSON array at `/indexes/movies/documents` and search works. No mappings, no analyzers. Settings *refine* behavior afterwards; changing them re-indexes automatically.
2. **Search is one flat object.** Compare 28 flat, orthogonal parameters ([search API](https://www.meilisearch.com/docs/reference/api/search): `q`, `filter`, `sort`, `facets`, `limit/offset` or `page/hitsPerPage`, `attributesToRetrieve/Crop/Highlight`, `cropLength/cropMarker`, `highlightPre/PostTag`, `showMatchesPosition`, `distinct`, `matchingStrategy`, `showRankingScore(Details)`, `rankingScoreThreshold`, `attributesToSearchOn`, `hybrid`, `vector`, `retrieveVectors`, `locales`, `media`, `personalize`) against Elasticsearch's recursive bool/must/should Query DSL. There is no query *language* to learn for the common path — the "language" only exists inside the `filter` string.
3. **Settings are a resource, not config files.** `GET/PATCH/DELETE /indexes/{uid}/settings` (and per-setting sub-routes) — introspectable, diffable, resettable to defaults. Every setting has a documented default ([settings API](https://www.meilisearch.com/docs/reference/api/settings)).
4. **Uniform async contract.** Every write returns `{ taskUid, indexUid, status: "enqueued", type, enqueuedAt }` — one polling pattern for everything, instead of ES's mix of sync writes, `?refresh=`, and task APIs.
5. **Uniform error object.** Errors are always `{ message, code, type, link }` where `link` points at documentation for that exact error code ([errors reference](https://www.meilisearch.com/docs/reference/errors/overview)). The self-documenting `link` field is a small touch with outsized DX value.
6. **Built-in result shaping.** Highlighting, cropping, and match positions are first-class search parameters, not a separate "highlighter" configuration language.
7. **Official SDKs for 10+ languages** and an OpenAPI spec, plus `llms.txt` for AI-assisted integration ([API overview](https://www.meilisearch.com/docs/reference/api/overview)).

## `matchingStrategy` — graceful degradation

Instead of ES's `minimum_should_match` arithmetic, one enum ([search API](https://www.meilisearch.com/docs/reference/api/search)): `last` (default — drop terms from the end of the query until enough results), `all` (require every term), `frequency` (keep rare terms, drop common ones). Simple, covers 95% of real needs.

## Findings

**F-024 — Meilisearch is schemaless by default: push a JSON array and search works, with no mappings or analyzers; settings refine behavior afterwards and changing them re-indexes automatically; documents are accepted as JSON, NDJSON, or CSV**
- Evidence: https://www.meilisearch.com/docs/reference/api/overview
- Impact: The zero-schema onboarding path is the core of the "learned in hours, not months" claim; Lexum's simple-search path should preserve a comparable no-configuration first experience.
- Confidence: high

**F-025 — The entire search request is one flat object of 28 orthogonal parameters; the only query "language" is the `filter` string — there is no recursive DSL to learn for the common path**
- Evidence: https://www.meilisearch.com/docs/reference/api/search
- Impact: Direct design constraint for Lexum: keep `q` + `filter` + `sort` as flat parameters on a simple search endpoint; LQL must be the power-user layer, never the only door.
- Confidence: high

**F-026 — Settings are a REST resource, not config files: `GET/PATCH/DELETE /indexes/{uid}/settings` plus per-setting sub-routes, introspectable/diffable/resettable, with a documented default for every setting**
- Evidence: https://www.meilisearch.com/docs/reference/api/settings
- Impact: The model Lexum should adopt over Elasticsearch's split between mappings, settings, and cluster state; Lexum's index-templates feature can then stamp settings objects of the same shape (see [execution plan](08-execution-plan.md), R-03).
- Confidence: high

**F-027 — The API has two uniform contracts: every write returns `{ taskUid, indexUid, status, type, enqueuedAt }` (one polling pattern for everything), and every error is `{ message, code, type, link }` where `link` points at documentation for that exact error code**
- Evidence: https://www.meilisearch.com/docs/reference/errors/overview · https://www.meilisearch.com/docs/learn/async/asynchronous_operations
- Impact: Lexum has 39 endpoints; a consistent machine-readable error contract with self-documenting links is cheap to add now and painful later (see [execution plan](08-execution-plan.md), R-02).
- Confidence: high

**F-028 — `matchingStrategy` replaces Elasticsearch's `minimum_should_match` arithmetic with a single enum: `last` (default, drop trailing terms until enough results), `all`, `frequency` (keep rare terms, drop common ones)**
- Evidence: https://www.meilisearch.com/docs/reference/api/search
- Impact: Trivial to implement over a Tantivy BooleanQuery and huge for search-as-you-type quality (never show zero results mid-typing) — see [execution plan](08-execution-plan.md), R-10.
- Confidence: high
