# SPEC-007 — Aggregations & Facets

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | phase5_aggregations-facets · tasks 1–6 ([proposal](../../.rulebook/tasks/phase5_aggregations-facets/proposal.md), [tasks](../../.rulebook/tasks/phase5_aggregations-facets/tasks.md)) |
| **Planning source** | Meilisearch plan [A-05, R-14](../analysis/meilisearch/08-execution-plan.md) (F-013); Elastic plan [P0 #5](../analysis/elastic/08-execution-plan.md) (F-017, F-055); 2026-07 code audit in the phase5 proposal (page-scoped executor at `crates/lexum-core/src/search/executor.rs:436-440`, stubs and `todo!()` paths in `crates/lexum-core/src/aggregation/`) |

Requirement IDs `AGG-xxx`. RFC 2119 keywords normative. Facets and aggregations ride in the search body defined by SPEC-004 and return inside its response envelopes (SPEC-004 §6.6). Facetable/aggregatable field declaration comes from the settings resource (phase4, R-03). Errors per SPEC-003. LQL `AGGREGATE`/`HISTOGRAM`/`TERMS` lower onto this spec per SPEC-014 §9.

## 1. Model — facet-first, ES layer on top

Two tiers over one engine:

1. **Facets** (the default door, A-05): `facets` on the search request → `facetDistribution` + `facetStats` in the response, plus a facet-search endpoint (R-14). Meilisearch wire shapes.
2. **ES-compatible aggregations** (the power tier): ES 7.10-shaped `aggs` blocks executed through `tantivy::aggregation`, for Kibana-style tooling and ES clients.

- **AGG-001** All facet and aggregation computation MUST execute over the **full matching document set** of the query (query + filter context), never over the returned page of hits. This is the correctness contract that replaces the page-scoped executor; the phase5 gate test (10k docs, `limit: 10`, terms agg — bucket counts sum to the full match count) is normative.
- **AGG-002** Execution MUST route through `tantivy::aggregation` (`AggregationCollector` over fast fields, single searcher pass alongside top-docs collection), honoring the index's `searchCutoffMs` budget. The pre-existing page-of-hits executor path is retired for every type this spec covers.
- **AGG-003** Facets are internally terms + stats aggregations; a result reachable as a facet and as an explicit aggregation over the same field MUST agree on counts and stats.
- **AGG-004** Anything this spec does not list as supported MUST return the uniform error `aggregation_type_unsupported` naming the type — never a placeholder, a silently wrong number, or a panic. No `todo!()`/`unimplemented!()` path may remain reachable in `crates/lexum-core/src/aggregation/`.

## 2. Facets in search responses

- **AGG-010** `SearchRequest` (POST and GET, SPEC-004 §2) gains `facets: string[]`. Each entry MUST name a field declared in the index's `filterableAttributes`; otherwise the request fails with `invalid_search_facets` naming the offending field **and** the setting to declare (`filterableAttributes`).
- **AGG-011** Response field `facetDistribution`: for each requested facet, a map of value → count of matching documents containing that value, computed under the full query + filter context:

```json
"facetDistribution": {
  "genre":  { "fiction": 412, "sci-fi": 187 },
  "rating": { "3": 50, "4": 210, "5": 96 }
}
```

Facet values are rendered as strings (numbers/bools stringified); array fields count each distinct value once per document.
- **AGG-012** Value count per facet is capped by the index setting `faceting.maxValuesPerFacet` (default **100**, settable 1–10000). Values are returned in **descending count order, ties broken lexicographically ascending**; values beyond the cap are omitted (counts of returned values remain exact).
- **AGG-013** Response field `facetStats`: for every requested facet whose values are numeric, `{ "min": number, "max": number }` over the matching set. Non-numeric facets are simply absent from `facetStats` (not an error).
- **AGG-014** `facets: ["*"]` expands to all `filterableAttributes`. An empty `facets` array is a no-op (no facet fields in the response).
- **AGG-015** Facets require fast fields (§6); the phase4 settings pipeline stamps them, and requesting a facet on a field without one fails per AGG-060.

## 3. Facet search endpoint

`POST /api/v1/indices/{index}/facet-search` — search-as-you-type over the **values** of one facet (R-14).

Request:

```json
{
  "facetName": "genre",          // required, must be in filterableAttributes
  "facetQuery": "fic",           // optional, default "" (match all values)
  "q": "dune",                   // optional main query (SPEC-004 §3 semantics)
  "filter": "rating > 3",        // optional, SPEC-004 filter grammar
  "matchingStrategy": "last"     // optional, applies to q
}
```

Response:

```json
{
  "facetHits": [ { "value": "fiction", "count": 412 } ],
  "facetQuery": "fic",
  "processingTimeMs": 3
}
```

- **AGG-020** `facetName` is validated against `filterableAttributes` (error `invalid_facet_search_facet_name`, naming field and setting). `facetQuery` matches facet values by **case-insensitive, diacritic-insensitive prefix**; an empty `facetQuery` returns the top values.
- **AGG-021** Counts in `facetHits` MUST respect the main `q`/`filter` context — the endpoint answers "if I also picked this facet value, how many hits would I get".
- **AGG-022** `facetHits` is ordered by descending count, ties lexicographically ascending; length capped by `faceting.maxValuesPerFacet`.
- **AGG-023** The endpoint MUST meet the phase5 latency gate: < 50 ms on the 10k-doc fixture.

## 4. ES-compatible aggregations

### 4.1 Request grammar

- **AGG-030** The search body accepts `aggs` or `aggregations` (exact synonyms; both present fails with `invalid_search_aggregations`) as `{ name: { <type>: { ...params }, aggs?: { ... } } }` — type-keyed exactly as ES 7.10, no `"type"` discriminator. Aggregation names MUST match `[a-zA-Z0-9_-]+`.
- **AGG-031** The ES-shaped grammar ships behind the `es_aggregations_dsl` experimental flag (phase6, R-12) until validated against real ES clients; when the flag is off, ES-shaped blocks fail with `feature_not_enabled` naming the flag. Facets (§2–§3) are NOT flag-gated.
- **AGG-032** The pre-existing internally-tagged native shape (`{"type": "terms", ...}`) MUST keep executing for one release, mapped 1:1 onto the same engine, with a deprecation warning in logs and a `Deprecation` response header; it is removed the release after.

### 4.2 Bucket aggregations

| Type | Parameters (defaults) | Notes |
|---|---|---|
| `terms` | `field` (req), `size` (10), `order` ({`_count`: `desc`}), `min_doc_count` (1), `missing` | Keyed by fast-field value. |
| `histogram` | `field` (req), `interval` (req, > 0), `min_doc_count` (0), `offset` (0), `extended_bounds`, `hard_bounds` | Numeric buckets `key = floor((val - offset) / interval) * interval + offset`. |
| `date_histogram` | `field` (req), exactly one of `fixed_interval` \| `calendar_interval`, `offset`, `min_doc_count` (0), `extended_bounds` | `fixed_interval` units `ms,s,m,h,d`; `calendar_interval` ∈ `minute,hour,day,week,month,quarter,year`. Keys are epoch millis; `key_as_string` in RFC 3339 UTC. |
| `range` | `field` (req), `ranges` (req: `{from?, to?, key?}`) | `from` inclusive, `to` exclusive; unbounded ends allowed; default key `"from-to"` with `*` for unbounded. |
| `filters` | `filters` (req: named map or array of query objects) | Each entry is a full SPEC-004 §4 query, actually evaluated (replaces the MatchAll-only stub). Anonymous array form yields keyed buckets `0..n-1`. |

- **AGG-040** All five bucket types MUST support nested `aggs` (sub-aggregations); results appear inside their parent bucket object. Maximum nesting depth: **4** levels (bucket-in-bucket counts a level); deeper fails with `aggregation_too_deep`.
- **AGG-041** `terms` responses MUST include `doc_count_error_upper_bound` and `sum_other_doc_count` (0 and the exact remainder respectively on a single node; the fields exist now so distributed merging in phase9 does not change the shape).

### 4.3 Metric aggregations

| Type | Parameters (defaults) | Result shape |
|---|---|---|
| `min`, `max`, `sum`, `avg` | `field` (req), `missing` | `{ "value": number \| null }` |
| `value_count` | `field` (req) | `{ "value": int }` |
| `stats` | `field` (req) | `{ "count", "min", "max", "avg", "sum" }` |
| `extended_stats` | `field` (req), `sigma` (2.0) | stats + `sum_of_squares`, `variance`, `std_deviation`, `std_deviation_bounds` |
| `cardinality` | `field` (req), `precision_threshold` (3000, max 40000) | `{ "value": int }` — approximate (HyperLogLog++-class); documented error bound ≤ ~1% below the threshold |
| `percentiles` | `field` (req), `percents` ([1,5,25,50,75,95,99]) | `{ "values": { "50.0": n, ... } }` — approximate (t-digest-class) |
| `top_hits` | `size` (3, max 100), `sort`, `_source` | `{ "hits": { "total", "max_score", "hits": [...] } }` using SPEC-004 hit envelope and sort contract |

- **AGG-042** Metrics are valid at top level and as sub-aggregations of any §4.2 bucket. Metrics MUST NOT declare sub-aggregations (`aggs` under a metric fails with `invalid_search_aggregations`).
- **AGG-043** `missing` (where listed) substitutes the given value for documents lacking the field; without it such documents are excluded from that aggregation.

### 4.4 Response wire shape

- **AGG-044** Responses render under the top-level `aggregations` key of the SPEC-004 envelope: buckets as `{ name: { "buckets": [ { "key", "key_as_string"?, "doc_count", <sub-agg name>: {...} } ] } }` (keyed `filters` uses an object of named buckets), metrics as `.value` / `.values` per §4.3. Field names and shapes MUST parse without error in an ES 7.x client library (phase5 gate 4.4).

## 5. Limits

| Limit | Default | Behavior on breach |
|---|---|---|
| `search.maxBuckets` — total buckets materialized per request | 65536 | `too_many_buckets` error naming the limit |
| `faceting.maxValuesPerFacet` | 100 | silent truncation (facets, AGG-012) |
| `terms.size` maximum | 10000 | `invalid_search_aggregations` |
| Nesting depth | 4 | `aggregation_too_deep` |
| `top_hits.size` maximum | 100 | `invalid_search_aggregations` |

- **AGG-050** `search.maxBuckets` counts every bucket across all aggregations and nesting levels in one request and is a server-level setting.

## 6. Field requirements and degraded paths

- **AGG-060** Aggregating or faceting a field that is not a fast field MUST fail with `invalid_aggregation_field` naming the field and the setting that would declare it (`filterableAttributes` / `sortableAttributes` → `fast: true` stamping per phase4) — never empty or wrong results, never a panic (covers pre-existing indices built without fast fields).
- **AGG-061** GET search MUST support `facets` (the pre-spec GET path hardcoded `aggregations: None`); ES-shaped `aggs` blocks remain POST-only.
- **AGG-062** `search_after`/PIT cursor requests carrying `aggs` or `facets` MUST fail with `invalid_search_aggregations` ("aggregations belong on the first page" — ES semantics), replacing the pre-spec silent drop.
- **AGG-063** Aggregations run inside the query's `searchCutoffMs`; on cutoff the whole request fails with `search_timeout` (no partial aggregation results on the single-node engine).

## 7. Deliberately not planned

- **AGG-070** The following are NOT planned until concrete demand exists, and MUST return `aggregation_type_unsupported`: `composite`, all pipeline aggregations (`bucket_script`, `bucket_selector`, `bucket_sort`, `cumulative_*`, `derivative`, `moving_*`, `serial_differencing`, `normalize`), `scripted_metric`, `significant_terms`, `sampler`, `global`, `missing`, `nested`/`reverse_nested`/`children`/`parent`, all geo aggregations (`geo_bounds`, `geo_centroid`, `geo_distance`, `geohash_grid`, `geo_line`), `ip_range`, `date_range` (revisit with demand — `range` over epoch-millis values covers the interim), `rate`, `t_test`, `boxplot`, `string_stats`, `median_absolute_deviation`, `weighted_avg`, `rollup`. This list mirrors the stub retirement in phase5 task 5 (A-05: start with facets, add ES buckets only behind clear demand).
- **AGG-071** Un-supporting a type is a wire-level contract: the error is stable and machine-readable so clients can feature-detect; no unsupported type may ever return a number.

## 8. Acceptance criteria

1. **Full-docset gate** (phase5 task 1.1/1.5): 10k docs, `limit: 10`, terms agg → bucket doc_counts sum to full matching count; metric values match hand-computed fixtures.
2. **Facet shape gate** (task 2.3): `facetDistribution`/`facetStats` match the documented shapes under an active filter; `maxValuesPerFacet` truncates; non-filterable facet errors with the setting named.
3. **Facet-search gate** (task 3.3): prefix narrowing as `facetQuery` grows; counts respect `q`/`filter`; empty query returns top values; < 50 ms on the 10k fixture.
4. **ES client parse gate** (task 4.4): terms, date_histogram (fixed + calendar), histogram, range, filters, cardinality, percentiles, stats, and terms→avg+top_hits nesting parsed by an ES 7.x client without error; `filters` buckets actually evaluate their queries.
5. **No-stub gate** (task 5.4): grep-clean of `todo!`/`unimplemented!`/placeholder in `crates/lexum-core/src/aggregation/`; every §7 type returns the uniform unsupported error.
6. **Degraded-path gate** (task 5.3/6.3): GET + `facets` works; `search_after` + aggs errors explicitly; non-fast-field aggregation errors with field and setting named.
