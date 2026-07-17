# SPEC-004 — Search API (Kernel Contract)

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | phase2_search-kernel-parity · tasks 1–7 ([proposal](../../.rulebook/tasks/phase2_search-kernel-parity/proposal.md), [tasks](../../.rulebook/tasks/phase2_search-kernel-parity/tasks.md)) |
| **Planning source** | Meilisearch plan [R-04, R-05, R-10, A-04](../analysis/meilisearch/08-execution-plan.md) (F-012, F-025, F-028); [§4 API design](../analysis/meilisearch/04-api-design.md) (F-024–F-028); [§6 relevancy](../analysis/meilisearch/06-relevancy.md) (F-034–F-036); Elastic plan [P0 #2, #6](../analysis/elastic/08-execution-plan.md) (F-055); [§4 Query DSL](../analysis/elastic/04-query-dsl.md) (F-025–F-030) |

Requirement IDs `SRCH-xxx`. The key words MUST, MUST NOT, SHOULD, SHOULD NOT, MAY are to be interpreted as described in RFC 2119. Errors follow the uniform error object of SPEC-003; write-side semantics (refresh visibility, task queue) are SPEC-002. Facets and aggregations carried in the same search body are SPEC-007. LQL lowers onto this kernel per SPEC-014.

## 1. Model — one kernel, two front doors

Lexum exposes a single search execution kernel behind two request grammars:

1. The **simple path**: one flat JSON object of orthogonal parameters (`q`, `filter`, `sort`, pagination, response shaping). No query language, no recursive DSL (A-04/F-025).
2. The **ES-compatible Query DSL subset**: the recursive `query` object accepted by ES 7.10 clients, scoped to the clauses in §4.

- **SRCH-001** Both grammars MUST compile to the same core `Query` representation and execute through the same executor. A feature reachable through the DSL MUST NOT change semantics when reached through the simple path, and vice versa.
- **SRCH-002** The simple path is first-class: every capability in this spec (typo tolerance, matchingStrategy, response shaping, both pagination styles, sort, filter) MUST be reachable with flat parameters and zero DSL/LQL (A-04).
- **SRCH-003** A request MAY combine `q` (simple full-text) with a structured `query` object; when both are present the effective query is `bool { must: [derived-from-q, query] }`. `q` alone with no `query` is the common path and MUST NOT require any other parameter.
- **SRCH-004** LQL (SPEC-014) is the third front door and MUST lower onto this kernel with no private execution path.

## 2. Endpoints

| Route | Method | Purpose |
|---|---|---|
| `/api/v1/indices/{index}/search` | POST | Primary search endpoint; accepts simple-path parameters, `query` DSL, or both. Body shape auto-detected. |
| `/api/v1/indices/{index}/search` | GET | Query-string subset of the simple path (`q`, `filter`, `sort`, `limit`, `offset`, `page`, `hitsPerPage`, `facets`). |
| `/{index}/_search` | POST/GET | ES-compatible alias. Same kernel; response rendered in the ES envelope (§6.6). |
| `/api/v1/_pit` | POST | Open a point-in-time reader (§5.3). |
| `/api/v1/_pit/{pit_id}` | POST / DELETE | Extend / close a PIT. |
| `/api/v1/_search/scroll` | POST | **Legacy-frozen** (§5.4). |

- **SRCH-010** The ES alias route MUST accept the DSL subset of §4 and the body parameters of §5–§6 exactly as an ES 7.10 client sends them (`from`/`size`, `sort`, `_source`, `track_total_hits`, `highlight`, `search_after`, `pit`).
- **SRCH-011** An unknown top-level body parameter MUST be rejected with `invalid_search_request` naming the parameter (no silent ignoring), except on the ES alias route where unknown ES parameters outside the compatibility scope MUST be rejected with `es_parameter_unsupported` naming the parameter.
- **SRCH-012** GET search MUST support the full simple-path semantics for the parameters it carries; a parameter available on POST but absent from GET is not an error, but a GET parameter MUST NOT behave differently than its POST equivalent.

## 3. Simple path — flat parameters

| Parameter | Type | Default | Meaning |
|---|---|---|---|
| `q` | string | `null` | Full-text query over searchable attributes. Analyzed; typo tolerance (§7) and matchingStrategy (§8) apply here and only here. |
| `filter` | Query \| Query[] \| string | `null` | Filter context (§4.2): no score contribution, cache-eligible. String form uses the LQL `WHERE` expression grammar (SPEC-014 §5). |
| `sort` | array | `null` | Sort contract (§9). |
| `limit` / `offset` | int | `10` / `0` | Offset pagination (§5.1). |
| `page` / `hitsPerPage` | int | `null` / `20` (when `page` set) | Exhaustive page pagination (§5.2). |
| `search_after` | array | `null` | Cursor pagination (§5.3). |
| `pit` | object | `null` | `{ "id": string, "keep_alive": string }` (§5.3). |
| `attributesToRetrieve` | string[] | `["*"]` | Source filtering (§6.1). |
| `attributesToHighlight` | string[] | `null` | Highlighting into `_formatted` (§6.2). |
| `highlightPreTag` / `highlightPostTag` | string | `"<em>"` / `"</em>"` | Highlight tags (§6.2). |
| `attributesToCrop` | string[] | `null` | Cropping into `_formatted` (§6.3). |
| `cropLength` | int | `10` | Crop window in **words** (§6.3). |
| `cropMarker` | string | `"…"` | Crop ellipsis marker (§6.3). |
| `showMatchesPosition` | bool | `false` | Emit `_matchesPosition` (§6.4). |
| `matchingStrategy` | enum | `"last"` | `last` \| `all` \| `frequency` (§8). |
| `min_score` | float | `null` | Drop hits scoring below the threshold. |
| `facets` | string[] | `null` | Facet distribution/stats — SPEC-007 §2. |
| `aggregations` / `aggs` | object | `null` | Aggregations — SPEC-007 §4. |
| `fields` | string[] | `null` | Legacy alias of `attributesToRetrieve` (kept; MUST NOT be combined with it). |
| `highlight` | object | `null` | Structured highlight config (ES-shaped on the alias route, native `HighlightConfig` otherwise). |
| `explain` | bool | `false` | Attach per-hit scoring explanation. |
| `track_total_hits` | bool \| int | `10000` | Total-count accuracy contract (§6.5). |

- **SRCH-020** `q` MUST be interpreted as terms combined per `matchingStrategy` over the index's searchable attributes (ordered field importance per the settings resource, R-03), never as a query-string mini-language: no operators, no field prefixes, no quotes-as-syntax beyond phrase grouping. Characters like `:` and `-` in `q` are text, not syntax.
- **SRCH-021** `filter` MUST accept: (a) a single structured query object, (b) an array of them (implicit AND), or (c) a string in the LQL filter-expression grammar. All three compile to filter context (§4.2).
- **SRCH-022** Every simple-path parameter MUST validate its type and range before execution; the first invalid parameter fails the request with `invalid_search_request` and the parameter name — never partial application.
- **SRCH-023** An empty or absent `q` with no `query` and no `filter` MUST behave as match-all ordered by the sort contract (§9), enabling browse/listing UIs.

## 4. ES-compatible Query DSL subset

### 4.1 Scope

- **SRCH-030** The `query` object MUST accept exactly these clause types; any other clause type MUST fail with `invalid_search_query` naming the unknown clause:

| Clause | Class | Notes |
|---|---|---|
| `match_all`, `match_none` | — | `match_all` supports `boost`. |
| `match` | full-text | String shorthand `{"match": {"f": "text"}}` and object form with `query`, `operator` (`or` default \| `and`), `fuzziness` (`0`/`1`/`2`/`"AUTO"`), `prefix_length`, `boost`. |
| `multi_match` | full-text | `query`, `fields` (with `^boost` suffixes), `type`: `best_fields` (default) \| `most_fields` \| `phrase`; `operator`, `fuzziness`, `tie_breaker`. |
| `match_phrase` | full-text | `query`, `slop` (default `0`), `boost`. |
| `term` | term-level | Exact, not analyzed. `{"term": {"f": {"value": v, "boost": b}}}` and shorthand. |
| `terms` | term-level | Field + array of values, OR semantics, non-scoring constant per ES. |
| `range` | term-level | `gt`/`gte`/`lt`/`lte`; on date fields values MUST accept ISO 8601 and ES date-math (`now`, `now-1d/d`, `\|\|` anchor syntax); `format` MAY override parsing. |
| `exists` | term-level | Matches documents with at least one indexed value for `field`. |
| `prefix` | term-level | Compiled to a Tantivy prefix automaton, never a regex fallback. |
| `wildcard` | term-level | `*`/`?`; leading wildcards allowed but SHOULD be documented as slow. |
| `ids` | term-level | `{"ids": {"values": [...]}}` over document IDs. |
| `bool` | compound | §4.2. |

- **SRCH-031** Explicitly **out of scope** (rejected with `es_query_unsupported` naming the clause): `query_string`, `simple_query_string` (on the ES wire; the native variant remains internal), `span_*`, `intervals`, `percolate`, `has_child`/`has_parent`/`parent_id`, `function_score` scripting on the ES wire, `more_like_this` on the ES wire. Rejection MUST be deterministic — never a silent downgrade to a different query.
- **SRCH-032** The native snake_case query shape that predates this spec MUST keep working unchanged on the primary route; the ES wire shape is additive.
- **SRCH-033** A `term`/`terms` clause against a `text` (analyzed) field MUST match against the indexed terms without query-time analysis (ES semantics, F-027) — the classic footgun is preserved deliberately for compatibility, and the documentation MUST recommend the `field.keyword` multi-field convention.

### 4.2 `bool` — query context vs filter context

```json
{ "bool": {
    "must":     [ ... ],   "should":   [ ... ],
    "must_not": [ ... ],   "filter":   [ ... ],
    "minimum_should_match": 1, "boost": 1.0
} }
```

- **SRCH-040** `must` and `should` clauses execute in **query context** and contribute to `_score`. `filter` and `must_not` clauses execute in **filter context**: they MUST NOT contribute to `_score` and MUST be eligible for the filter bitset cache (`search/filter_cache.rs`) (F-025).
- **SRCH-041** Adding or removing a `filter` clause MUST leave the `_score` of every matching hit byte-identical (phase2 gate 7.2).
- **SRCH-042** `should` promotion (F-026): when a `bool` has no `must` and no `filter` clauses, `minimum_should_match` defaults to `1` (at least one `should` must match); otherwise it defaults to `0`. Explicit `minimum_should_match` (integer ≥ 0) overrides in both cases. Negative and percentage forms are out of scope and MUST be rejected.
- **SRCH-043** `bool` clauses nest arbitrarily; context is positional and MUST propagate: every clause inside a `filter` or `must_not` subtree executes in filter context regardless of its own type.
- **SRCH-044** A query passed at the top level of the simple path's `filter` parameter is equivalent to wrapping it in `bool.filter` — one implementation, verified by identical results.

## 5. Pagination

Three mechanisms; `scroll` is legacy-frozen. A request MUST use at most one of {`from`/`size` \| `offset`/`limit`}, {`page`/`hitsPerPage`}, {`search_after`}; mixing styles fails with `invalid_search_request`.

### 5.1 Offset style — `offset`/`limit` and `from`/`size`

- **SRCH-050** `from`/`size` (ES names) and `offset`/`limit` (native names) are aliases of the same two integers. Both spellings MUST be accepted on all routes; supplying both spellings of the same integer with different values fails with `invalid_search_request`.
- **SRCH-051** Defaults: `offset = 0`, `limit = 10`. Bounds: `offset + limit` MUST NOT exceed the index setting `pagination.maxTotalHits` (default **10000**); exceeding it fails with `invalid_search_offset` naming the setting.
- **SRCH-052** Offset-style responses report `estimatedTotalHits` (native envelope) / `hits.total` with `relation` (ES envelope) per the `track_total_hits` contract (§6.5). The engine MUST NOT pay for an exact count that was not requested.

### 5.2 Page style — `page`/`hitsPerPage`

- **SRCH-053** When `page` (1-based) is present, the response switches to the exhaustive contract: exact `totalHits` and `totalPages` MUST be returned (internally forcing an exact count up to `pagination.maxTotalHits`), plus echo fields `page` and `hitsPerPage`. Default `hitsPerPage = 20`. `page = 0` is valid and returns zero hits with correct totals (Meilisearch placeholder-search semantics). Per the §5 mixing rule, `page` present ⇒ any `offset`/`limit`/`from`/`size` in the same request fails with `invalid_search_request`.

### 5.3 Cursor style — `search_after` + PIT (one contract)

Deterministic deep pagination is **search_after over a point-in-time reader**. Legacy `scroll` is not part of this contract.

- **SRCH-054** `POST /api/v1/_pit?index={index}&keep_alive={dur}` opens a PIT and returns `{ "id": string, "keep_alive": string }`. `keep_alive` uses duration syntax (`30s`, `1m`, `1h`); default `1m`, maximum the server setting `pit.maxKeepAlive` (default `24h`). Each search carrying the PIT MAY refresh `keep_alive`. Expired or unknown PIT ids fail with `pit_not_found` (404-class); PITs are dropped on index deletion; `DELETE /api/v1/_pit/{id}` is idempotent.
- **SRCH-055** A search body MAY carry `pit: { id, keep_alive }`. A PIT search MUST NOT also specify an index in the path on the ES alias route (the PIT pins the index and snapshot); the native route requires the path index to match the PIT's index.
- **SRCH-056** Tiebreak determinism: when a sort is supplied, the engine MUST append an implicit unique document-id tiebreaker as the final sort key; when no sort is supplied with a PIT/search_after request, the effective sort is `[_score desc, _id asc]`. Every hit in a cursor response MUST carry its `sort` values array; the client passes the last hit's array as `search_after`.
- **SRCH-057** Snapshot stability: within one PIT, repeated `search_after` pages MUST observe a frozen index state — concurrent writes MUST NOT cause duplicates or skips across pages (phase2 gate 3.4: 50k docs walked under concurrent writes, zero dupes/skips).
- **SRCH-058** `search_after` without a PIT is allowed (live-index cursor) and MUST apply the same implicit tiebreaker, with the documented caveat that concurrent writes can shift pages. `search_after` MUST NOT be combined with `offset`/`from` or `page`.

### 5.4 Scroll — legacy-frozen

- **SRCH-059** Existing `/api/v1/_search/scroll` routes remain functional but frozen: no new capabilities, responses carry a `Deprecation` header pointing at the PIT contract, and documentation marks them legacy (ES anti-goal F-050 #3).

## 6. Response shaping

### 6.1 Source filtering

- **SRCH-060** `attributesToRetrieve` (default `["*"]`) selects the fields of `source` returned per hit; supports `*` and dotted paths for nested objects. The ES alias route exposes the same machinery as `_source`: `true` (all) / `false` (none) / array of patterns / `{ "includes": [...], "excludes": [...] }` with `excludes` applied after `includes`.
- **SRCH-061** Source filtering is presentation-only: it MUST NOT affect matching, scoring, highlighting inputs, or sort.

### 6.2 Highlighting

- **SRCH-062** `attributesToHighlight` wraps every matched term occurrence in `highlightPreTag`/`highlightPostTag` (defaults `<em>`/`</em>`) and emits results in a per-hit `_formatted` object containing the **full document as shaped by `attributesToRetrieve`**, with highlighted attributes replaced by their tagged versions (Meilisearch semantics). `["*"]` highlights all displayed attributes.
- **SRCH-063** On the ES alias route, `highlight: { fields, pre_tags, post_tags, fragment_size (default 100), number_of_fragments (default 3) }` maps onto the same highlighter and returns the ES-shaped per-hit `highlight: { field: [fragments...] }` object.
- **SRCH-064** Terms matched via typo expansion (§7) and prefix expansion MUST be highlighted as matches.

### 6.3 Cropping

- **SRCH-065** `attributesToCrop` (entries `field` or `field:length`) crops each listed attribute inside `_formatted` to a window of `cropLength` **words** (default 10; per-field `:length` overrides) centered on the densest match region; when the attribute has no match the crop is taken from the start of the field.
- **SRCH-066** `cropMarker` (default `…`) is prepended/appended only where text was actually removed. Cropping composes with highlighting in the same `_formatted` value: crop first, then tag.

### 6.4 Match positions

- **SRCH-067** `showMatchesPosition: true` adds per-hit `_matchesPosition`: `{ attribute: [ { "start": <byte offset>, "length": <bytes> }... ] }` for every matched term in every searchable attribute, computed on the **unformatted** source values. Offsets are UTF-8 byte offsets.

### 6.5 Totals — `track_total_hits`

- **SRCH-068** `track_total_hits` accepts `true` (exact count), `false` (no count), or an integer threshold N (exact up to N, then lower bound). Default **10000**. The result carries `total.value` and `total.relation` ∈ {`eq`, `gte`} on the ES envelope; the native envelope maps this to `estimatedTotalHits` (offset style) or exact `totalHits` (page style, which internally forces exactness per SRCH-053).

### 6.6 Response envelopes

- **SRCH-069** Native envelope (primary route): `{ "hits": [ { "id", "score", "source", "_formatted"?, "_matchesPosition"?, "highlight"?, "sort"? } ], "estimatedTotalHits" | ("totalHits" + "totalPages" + "page" + "hitsPerPage"), "limit", "offset", "processingTimeMs", "query", "facetDistribution"?, "facetStats"?, "aggregations"? }`. ES envelope (alias route): `{ "took", "timed_out", "hits": { "total": {"value","relation"}, "max_score", "hits": [ { "_index", "_id", "_score", "_source", "sort"?, "highlight"? } ] }, "aggregations"? }`. Field names in both envelopes are normative.

## 7. Typo tolerance

Applies **only** to terms derived from `q` (and LQL `MATCH`, SPEC-014 §6). Explicit `term`/`match_phrase`/DSL clauses are never fuzzed.

- **SRCH-070** Typo budget by word length (Meilisearch defaults, R-04/F-012):

| Query word length | Max typos |
|---|---|
| 1–4 chars | 0 |
| 5–8 chars | 1 |
| ≥ 9 chars | 2 |

- **SRCH-071** A typo on the **first letter** counts as two typos: a word in the 1-typo tier MUST NOT match candidates differing in the first letter; in the 2-typo tier a first-letter typo consumes the whole budget. Implemented as prefix-anchored Levenshtein automata over Tantivy `FuzzyTermQuery`.
- **SRCH-072** Exact matches MUST rank at or above their fuzzy variants for the same document set (typo count is a ranking criterion, F-035 rule 2).
- **SRCH-073** Per-index setting object `typoTolerance` (mounted under the settings resource, R-03):

```json
{
  "enabled": true,
  "minWordSizeForTypos": { "oneTypo": 5, "twoTypos": 9 },
  "disableOnWords": [],
  "disableOnAttributes": [],
  "disableOnNumbers": false
}
```

Constraints: `1 ≤ oneTypo ≤ twoTypos ≤ 255`, violations fail with `invalid_settings_typo_tolerance`. `disableOnWords` matches query words case-insensitively after analysis; `disableOnAttributes` disables expansion for terms matched against the listed attributes; `disableOnNumbers: true` exempts purely numeric tokens.
- **SRCH-074** Typo expansion is a query-time rewrite: it MUST NOT change what is indexed, and disabling it (`enabled: false`) restores exact-term matching with no re-index.

## 8. matchingStrategy

- **SRCH-080** `matchingStrategy` ∈ {`last` (default), `all`, `frequency`} controls term relaxation for multi-term `q` (R-10/F-028):
  - `last`: build progressively relaxed boolean variants dropping query words **from the end**; return the strictest variant that yields results.
  - `all`: every query word required; zero results is an acceptable outcome.
  - `frequency`: drop the **highest document-frequency** word first (Tantivy `doc_freq`), repeating until results exist.
- **SRCH-081** Under `last` and `frequency`, hits matching more query words MUST rank above hits matching fewer (the `words` criterion, F-035 rule 1), regardless of BM25 tie values — implemented by boosting stricter variants.
- **SRCH-082** Single-word queries are unaffected by `matchingStrategy`. The strategy applies only to the `q` path; DSL queries express their own semantics via `bool`/`minimum_should_match`.

## 9. Sort contract

- **SRCH-090** Native sort accepts an array of `{ "field": string, "order": "asc" | "desc" }` (default `asc`) and the string shorthands `"field:asc"` / `"field:desc"`; ES sort accepts `[ { "field": { "order": "desc" } }, "field2", "_score" ]`. Both map to the same `SortOption` list.
- **SRCH-091** `_score` is a valid sort field in any position (descending by default). When no sort is given, the effective sort is `[_score desc]` (plus the cursor tiebreaker per SRCH-056 when applicable).
- **SRCH-092** Sortable fields MUST be declared (`sortableAttributes` in settings / fast field in the schema); sorting on an undeclared field fails with `invalid_search_sort` naming the field and the setting to declare. `_score` and `_id` need no declaration.
- **SRCH-093** Missing values: documents lacking the sort field sort **last** regardless of direction by default; the ES sort object accepts `missing: "_first" | "_last"` per field to override. Missing-value placement MUST be stable across pages of the same cursor.
- **SRCH-094** Multi-valued fields sort by their minimum value for `asc` and maximum for `desc` (ES `mode: min/max` defaults); other modes are out of scope.
- **SRCH-095** When an explicit sort is present, `_score` MUST still be computed and returned per hit if the query has query-context clauses, but MUST NOT influence order except where listed in the sort array.

## 10. Errors

- **SRCH-100** All failures use the uniform error object of SPEC-003 (`{ message, code, type, link }`). Codes introduced by this spec: `invalid_search_request`, `invalid_search_query`, `es_query_unsupported`, `es_parameter_unsupported`, `invalid_search_offset`, `invalid_search_sort`, `invalid_settings_typo_tolerance`, `pit_not_found`.
- **SRCH-101** Search MUST fail atomically: no partial hit lists accompany an error. Per-query partial results exist only in multi-search/federation (out of scope here; see the phase8 spec).

## 11. Acceptance criteria

1. **DSL conformance suite** (phase2 task 7.1): fixture corpus covering every §4.1 clause plus nested `bool` mixing contexts; 100% pass on hits, order, totals.
2. **Filter-context proof** (task 7.2): adding a `filter` clause leaves `_score` byte-identical (SRCH-041).
3. **Deep-paging gate** (task 3.4): ≥50k docs via `search_after` + PIT under concurrent writes — zero duplicates/skips (SRCH-057); `from/size` and `page/hitsPerPage` report consistent totals.
4. **Typo table tests** (task 5.4): word lengths 1–20 against the SRCH-070 budgets; first-letter double-count (SRCH-071); each `disableOn*` knob; custom `minWordSizeForTypos`.
5. **matchingStrategy tests** (task 6.3): trailing nonsense term → non-empty under `last`, empty under `all`; `frequency` drops the stopword-like term first.
6. **Golden-response tests** (task 4.5): crop/highlight/retrieve/showMatchesPosition on both pagination styles, both envelopes.
7. **Simple-door test** (task 7.3, A-04): every feature above exercised through flat parameters with no DSL body and no LQL.
