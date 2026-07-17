# Proposal: phase8_multisearch-federation

## Why

Two findings make this the pivot task of the whole plan:

- **R-06 (F-036)** — Lexum exposes only raw BM25 scores today
  (`SearchHit.score` in `crates/lexum-core/src/search/result.rs` is the
  unbounded Tantivy score). A **normalized 0–1 ranking score** with
  `showRankingScore`/`showRankingScoreDetails` is the prerequisite for
  everything that merges results from more than one query: federated
  multi-search, distributed scatter-gather (phase9), and hybrid
  lexical+vector fusion (phase10, `semanticRatio` is meaningless over
  unbounded BM25). Score explainability is also a major DX
  differentiator vs Elasticsearch.
- **R-07 (F-016, F-023)** — Meilisearch proved federation-first is the
  cheaper path to distribution: build `POST /multi-search` (N queries,
  one round trip), then federated merging (one ranked list, per-query
  `weight`), then `remote` targeting other Lexum nodes. **Federation IS
  the scatter-gather layer of Lexum's future distributed search** —
  building it as a public API first means phase9's distribution work
  reuses a tested, benchmarked code path instead of inventing a private
  one (Meilisearch's own sharding is federation underneath).

Lexum already has partial machinery to build on, not around:
`_msearch` exists (`crates/lexum-server/src/handlers/query_ops.rs` →
`crates/lexum-core/src/search/multi_search.rs`), and its
`MultiSearchResponseItem` already carries a per-item `Option<
MultiSearchError>` — the right contract per **A-03**: never fail the
whole request because one query is bad; Meilisearch's all-or-nothing
federation is the documented anti-pattern, Elasticsearch's per-item
errors are the model. What is missing: the Meilisearch-shaped
`/multi-search` API, federated merging into one ranked list, remotes,
and the normalized score that makes cross-query merging sound.

Dependencies: builds on phase2 (search kernel/response shaping) and
phase7 (per-sub-query index privileges via `AuthContext`; remotes
authenticate with API keys). Feeds phase9 (scatter-gather reuse) and
phase10 (hybrid fusion over normalized scores).

## What Changes

1. **Normalized ranking score (R-06).** Add `showRankingScore` (0–1),
   `showRankingScoreDetails` (per-criterion breakdown: bm25 per field
   with weights, exactness/typo components once phase2 lands them), and
   `rankingScoreThreshold` to the search request. Normalization is
   deterministic and documented (BM25 normalized against a computable
   bound, not against the top hit — top-hit normalization is not stable
   across shards/queries and would poison federation). Raw score stays
   in `_score` untouched.
2. **`POST /multi-search` (non-federated).** Body `{ queries: [...] }`
   where each entry is a full search request plus `indexUid`. Response
   is an array of per-query results **with per-query error objects**
   (A-03): a failed query yields `{ indexUid, error }` in its slot while
   the others return normally.
3. **Federated mode.** `{ federation: { limit, offset }, queries: [...] }`
   merges all hits into a single list ranked by weighted normalized
   score; per-query `federationOptions: { weight }`; dedup identical
   (index, docId) pairs keeping the best score; each hit carries
   `_federation: { indexUid, queriesPosition, weightedRankingScore }`.
4. **`remote` federation (R-07).** `GET/PATCH /network` config resource
   (`self` name + map of remotes `{ url, searchApiKey }`);
   `federationOptions.remote` proxies that query to the named Lexum
   node's `/multi-search`, merges returned normalized scores locally.
   Remote failures degrade to per-query error objects with partial
   results — never all-or-nothing. This is deliberately the seed of
   phase9's scatter-gather: the coordinator/merge code written here is
   the code distribution will call.
5. **`_msearch` ES-compat alias.** Keep the existing `_msearch` endpoint
   as an alias over the same execution core (one merge/execute engine,
   two wire formats), including NDJSON body support for shipper
   compatibility (ES P1 item 12).

## Impact

- Affected specs: `.rulebook/tasks/phase8_multisearch-federation/specs/`
  (score normalization contract, multi-search/federation API, network
  resource)
- Affected code:
  - `crates/lexum-core/src/search/result.rs` (normalized score + details
    on `SearchHit`), `crates/lexum-core/src/search/executor.rs`
    (score bound computation, threshold)
  - `crates/lexum-core/src/search/multi_search.rs` and
    `multi_executor.rs` (federated merge engine, weights, dedup)
  - `crates/lexum-server/src/handlers/query_ops.rs` (alias wiring),
    new `crates/lexum-server/src/handlers/multi_search.rs` and
    `network.rs`; `crates/lexum-server/src/router.rs`,
    `crates/lexum-server/src/openapi.rs`
  - `crates/lexum-server/src/services/` (remote proxy client,
    reqwest-based)
- Breaking change: NO (all new fields opt-in; `_score` and existing
  `_msearch` responses unchanged)
- User benefit: one round trip for multi-widget UIs, cross-index ranked
  results with tunable weights, and cross-node search today — while
  paying down the exact code path distributed search needs tomorrow.

## Success criteria

- `showRankingScore` returns scores in [0,1], strictly monotonic with
  raw BM25 within a query (property test), and stable across identical
  corpora on different nodes (same doc + query → same score, test).
- `rankingScoreThreshold: 0.8` filters exactly the hits below 0.8.
- Federated search over 3 indexes returns one list sorted by weighted
  normalized score; changing a query's `weight` reorders accordingly
  (deterministic fixture test); duplicate (index, docId) appears once.
- One malformed query in a 5-query request: 4 results + 1 error object,
  HTTP 200 (A-03 contract test); same for an unreachable `remote`.
- Remote federation round-trips between two live Lexum processes in an
  integration test, authenticating with a phase7 search-scoped key.
- `_msearch` keeps its current response contract (regression tests) and
  accepts NDJSON.
- Benchmark: 10-query `/multi-search` completes in ≤ 1.3x the wall time
  of the slowest single query on the bench corpus (parallel execution,
  not sequential), recorded in `benchmark/`.
