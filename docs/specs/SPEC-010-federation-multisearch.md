# SPEC-010 — Multi-Search, Federation & the Normalized Ranking Score

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Phase 8 · tasks 1–5 (`.rulebook/tasks/phase8_multisearch-federation/tasks.md`) |
| **Planning source** | [phase8 proposal](../../.rulebook/tasks/phase8_multisearch-federation/proposal.md); Meilisearch R-06 / F-036 (normalized score), R-07 / F-016, F-023 (federation-first distribution), A-03 (never all-or-nothing); Elastic P1 item 12 (`_msearch` NDJSON); existing `crates/lexum-core/src/search/multi_search.rs`, `crates/lexum-server/src/handlers/query_ops.rs` |

Requirement IDs `FED-xxx`. RFC 2119 keywords are normative. This spec is the **scatter-gather layer**: SPEC-011 distributed search and SPEC-012 hybrid fusion are consumers of the merge engine and score contract defined here — neither may introduce a second merge path. Errors follow the SPEC-003 error contract; per-sub-query authorization uses the SPEC-009 `AuthContext`.

## 1. Model

One execution core, three wire surfaces:

```
                    ┌─ POST /multi-search  (Meilisearch-shaped, federated or not)
client ─────────────┼─ POST /api/v1/_msearch (ES-compat alias, JSON or NDJSON)
                    └─ internal: SPEC-011 shard scatter, SPEC-012 hybrid branches
                                  │
                    MultiSearchExecutor (bounded parallelism)
                                  │
                    FederatedMergeEngine (k-way merge over normalized scores)
```

- **FED-001** All three surfaces MUST execute through the same `MultiSearchExecutor` and, when merging, the same `FederatedMergeEngine` in `crates/lexum-core/src/search/multi_search.rs`. Wire-format adapters own only (de)serialization.
- **FED-002** Sub-queries execute concurrently with bounded parallelism (default: `min(queries, num_cpus)`); response slots preserve request order regardless of completion order.

## 2. Normalized ranking score

### 2.1 Normalization function

- **FED-010** The normalized ranking score of a hit is `s = clamp(raw / bound(q), 0.0, 1.0)` where `raw` is the Tantivy BM25 score and `bound(q)` is the **per-query theoretical upper bound**, computed from the query and index statistics alone:

  `bound(q) = Σ_{scoring term clauses t} boost_t · weight_field(t) · idf_t · (k1 + 1)`

  (`idf_t · (k1+1)` is the BM25 saturation limit of a single term as tf → ∞; boolean `should`/`must` scoring clauses sum; pure filter context contributes 0). For query shapes with no computable term bound (e.g. `match_all`, constant-score), `bound = 1.0` and `raw` is the constant score.
- **FED-011** `bound(q)` MUST be independent of the result set. Normalizing against the local top hit (or any per-page statistic) is **forbidden**: the top hit differs per shard, per node, and per pagination window, so top-hit-relative scores are not comparable across the sub-results a merge combines — it would silently poison federated ordering, SPEC-011 shard merging, and SPEC-012 `semanticRatio`. This prohibition is load-bearing for every downstream spec.
- **FED-012** Determinism: identical (document, query, index settings, index statistics) MUST yield an identical normalized score — in particular on a rebuilt index of the same corpus on another node. Scores are monotonic with `raw` within a query. Cross-corpus comparability is expressly NOT claimed (idf is corpus-dependent); the honest contract is stated in user docs.
- **FED-013** The raw score remains untouched in `_score`. The normalized score appears as `_rankingScore` only when requested.

### 2.2 Request/response surface

- **FED-014** Search requests (single and per sub-query) accept `showRankingScore: bool` (default false) → each hit carries `"_rankingScore": 0.0..=1.0`; and `showRankingScoreDetails: bool` → each hit carries a per-criterion breakdown:

```json
"_rankingScoreDetails": {
  "bm25": { "order": 0, "score": 0.83,
            "fields": { "title": { "weight": 2.0, "score": 0.61 },
                        "body":  { "weight": 1.0, "score": 0.22 } } }
}
```

  The object is extensible: phase2 criteria (exactness, typo, proximity) and SPEC-012 (`vector`, `fusion`) add sibling entries with the same `{ order, score }` shape; per-criterion `score` is itself in [0,1].
- **FED-015** `rankingScoreThreshold: f64 ∈ [0,1]` drops hits with `_rankingScore` strictly below the threshold **after scoring, before pagination** (and before federated merging: FED-035). Invalid values → 400 `invalid_search_ranking_score_threshold`.
- **FED-016** Computing the normalized score is mandatory internally for every federated/distributed/hybrid execution even when `showRankingScore` is false; the flags control serialization only.

## 3. `POST /multi-search` — non-federated

- **FED-020** Request body:

```json
{ "queries": [
    { "indexUid": "products", "q": "shoes", "filter": "price < 100", "limit": 10,
      "showRankingScore": true },
    { "indexUid": "brands", "q": "shoes" }
] }
```

  Each entry is a full search request plus mandatory `indexUid`. Unknown fields per sub-query are rejected exactly as on the single-search endpoint.
- **FED-021** Response (HTTP 200): `{ "results": [ ... ] }` — one slot per query, in request order. A successful slot is the standard search response plus its `indexUid`.
- **FED-022** **Per-query errors (A-03).** A failing sub-query yields `{ "indexUid": "...", "error": { message, code, type, link } }` in its slot; all other slots return normally. There is NO input for which one bad query fails the whole request — the all-or-nothing failure path MUST NOT exist. The request as a whole errors only for malformed top-level JSON, cap violations (FED-060), or top-level auth failure.
- **FED-023** Authorization: the middleware admits the route with `search`; the handler then checks each `indexUid` against the SPEC-009 `AuthContext` (and tenant `searchRules`). An unauthorized index produces a per-query `insufficient_permissions` error object in that slot only.
- **FED-024** Tenant tokens: forced filters are resolved and AND-combined per sub-query per SEC-056/057.

## 4. Federated mode

- **FED-030** Presence of the top-level `federation` object selects federated mode:

```json
{ "federation": { "limit": 20, "offset": 0 },
  "queries": [
    { "indexUid": "movies", "q": "batman",
      "federationOptions": { "weight": 1.0 } },
    { "indexUid": "comics", "q": "batman",
      "federationOptions": { "weight": 0.8, "remote": "eu-node" } }
] }
```

  Response: a single search-response-shaped object with one merged `hits` list, `limit`/`offset` applied after the merge, plus `remoteErrors` (FED-045) when applicable.
- **FED-031** Merge key: `weightedRankingScore = federationOptions.weight × _rankingScore`, `weight ∈ (0, +∞)` default `1.0`. The k-way merge orders all hits by `weightedRankingScore` descending.
- **FED-032** Deterministic total order — ties break by: (1) `weightedRankingScore` desc, (2) `queriesPosition` asc, (3) `indexUid` bytewise asc, (4) document id bytewise asc. The same inputs MUST produce the same merged list on every node (this rule is what makes SPEC-011's shard-parity test possible).
- **FED-033** Dedup: hits with identical `(indexUid, docId)` reached by multiple queries appear once, keeping the highest `weightedRankingScore` (ties per FED-032).
- **FED-034** Every merged hit carries

```json
"_federation": { "indexUid": "movies", "queriesPosition": 0,
                 "weightedRankingScore": 0.7132, "remote": "eu-node" }
```

  (`remote` present only for remote hits).
- **FED-035** In federated mode per-query `offset`/`limit`/`page`/`hitsPerPage` are rejected (400 `invalid_multi_search_query_pagination` as a per-request error — it is a request-shape error, not a runtime failure); per-query `rankingScoreThreshold`, `filter`, `sort` remain valid and apply before merging. `showRankingScore(Details)` on the federation level propagates to all hits.

## 5. `remote` federation and the `/network` resource

- **FED-040** Config resource:

```
GET  /network → { "self": "us-node", "remotes": { "eu-node": { "url": "https://lexum-eu:7700", "searchApiKey": "lxk_..." } } }
PATCH /network   (partial update; null deletes a remote)
```

  Guarded by `network.get` / `network.update` (SPEC-009 §3). `searchApiKey` values are write-only: GET returns `"searchApiKey": "<redacted>"`.
- **FED-041** `federationOptions.remote: "<name>"` proxies that sub-query to the named remote's `POST /multi-search` (federated, single-query body), authenticating with its `searchApiKey`. An unknown name → per-query error `invalid_multi_search_remote` (the rest of the federation proceeds).
- **FED-042** Remote hits are merged by the `weightedRankingScore` **returned by the remote** through the same `FederatedMergeEngine` (FED-001) — no rescoring, no separate remote merge path. This is sound precisely because of FED-010/011/012 and is the reuse seam SPEC-011 distributed search builds on.
- **FED-043** The proxy client is connection-pooled with per-remote `requestTimeout` (default 3000 ms) inside the request's overall budget; a remote MUST also be given the originating request's remaining `searchCutoffMs` when smaller.
- **FED-044** Remote failures (timeout, unreachable, TLS/auth failure, malformed response) degrade to per-query error objects with partial results — never all-or-nothing (A-03). Codes: `remote_timeout`, `remote_could_not_send_request`, `remote_invalid_api_key`, `remote_bad_response`.
- **FED-045** Federated responses additionally carry `remoteErrors: { "<remote>": { message, code, type, link } }` for remotes that contributed no hits due to failure; absence of the key means all remotes answered.
- **FED-046** Loop safety: proxied requests carry `X-Lexum-Federation-Hops` (incremented per hop); requests arriving with hops ≥ 1 MUST NOT fan out to further remotes (error `remote_federation_loop` per offending sub-query).

## 6. `_msearch` ES-compat alias

- **FED-050** `POST /api/v1/_msearch` remains available with its current JSON contract (regression-pinned) and is re-routed onto the FED-001 core; `MultiSearchResponseItem`'s per-item `error` slot is preserved.
- **FED-051** NDJSON bodies (`Content-Type: application/x-ndjson`, alternating header/body lines `{"index": "..."}\n{query}\n`) are accepted for ES shipper compatibility; the response is the ES shape `{ "took": n, "responses": [ ... ] }` with per-item errors.
- **FED-052** `_msearch` never supports federated merging — it is strictly the non-federated surface; `federation` in an `_msearch` body is an error.

## 7. Limits

- **FED-060** `queries` count caps, configurable: default **20** non-federated, **30** federated. Exceeding → 400 `too_many_search_queries` (whole request — a cap violation is a request-shape error).
- **FED-061** The `query_complexity` middleware budget applies to the **sum** of sub-query complexities; sub-queries share the request's rate-limit accounting.
- **FED-062** Wall-clock: a 10-query `/multi-search` MUST complete in ≤ 1.3× the slowest constituent query on the bench corpus (parallelism guarantee, tracked in `benchmark/`).

## 8. Acceptance criteria

1. **Score properties** (property tests): `_rankingScore ∈ [0,1]`; strictly monotonic with `_score` within a query; identical doc+query+settings on a rebuilt index → identical score (FED-012); `rankingScoreThreshold: 0.8` filters exactly the hits below 0.8.
2. **Partial results**: 5 queries with 1 malformed + 1 unauthorized → 3 results + 2 per-query error objects, HTTP 200 (FED-022/023); same shape when a remote is unreachable (FED-044) with `remoteErrors` populated.
3. **Federated determinism**: 3-index federation returns one list ordered by weighted score; changing a `weight` reorders per FED-031; duplicate `(indexUid, docId)` appears once; repeated runs byte-identical ordering (FED-032/033 fixtures).
4. **Remote round trip**: two live Lexum processes configured as each other's remotes federate a query, authenticated with a SPEC-009 search-scoped key; killing one mid-test yields partial results + `remoteErrors`; hop guard blocks loops (FED-046).
5. **Compat**: `_msearch` JSON regression suite green; NDJSON accepted; FED-062 benchmark recorded in `benchmark/`.
