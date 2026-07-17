## 1. Core query gaps (greenfield variants)
- [ ] 1.1 Add `Terms` (field + value set), `Exists` (field has any indexed value), `Ids` (doc-id set), and `Prefix` variants to the `Query` enum in `crates/lexum-core/src/query/types.rs`, with builder support in `query/builder.rs` (Prefix compiles to Tantivy prefix/automaton query, not a Regex fallback)
- [ ] 1.2 Query/filter context in the executor: `BoolQuery.filter` and `must_not` clauses compile to scoreless, cache-eligible Tantivy occurs (wire into the existing `search/filter_cache.rs`); `must`/`should` keep scoring
- [ ] 1.3 `track_total_hits` support in `crates/lexum-core/src/search/executor.rs` and `result.rs`: `true` (exact), `false`, integer threshold; result carries `total.value` + `total.relation` (`eq`/`gte`)

## 2. ES `_search` wire adapter (gap-closing: rich native enum exists, ES wire shape does not)
- [ ] 2.1 New `crates/lexum-core/src/query/es_dsl.rs`: deserialize the ES 7.10 query grammar — `bool` (must/should/must_not/filter, `minimum_should_match`), `match` (string shorthand + object form with `operator`, `fuzziness`), `multi_match` (fields with `^boost`), `match_phrase` (`slop`), `term`/`terms`/`range` (gt/gte/lt/lte, date math passthrough)/`exists`/`prefix`/`wildcard`/`ids`, `match_all` — into the core `Query` enum; unknown clause → `invalid_search_query` error naming the clause
- [ ] 2.2 ES body params on the search handler (`crates/lexum-server/src/handlers/search.rs`): `from`/`size` (alias of offset/limit, both accepted), ES-shaped `sort` (`[{"field": {"order": "desc"}}, "_score"]`) mapped to `SortOption`s, `_source` (bool / array / `{includes, excludes}`) unified with the existing `fields` param, `highlight` (ES `fields`/`pre_tags`/`post_tags`/`fragment_size`/`number_of_fragments`) mapped onto `HighlightConfig`
- [ ] 2.3 Route the ES-shaped body: accept it on the existing `POST /api/v1/indices/{index}/search` (shape auto-detected) and add an ES-compatible alias route in `src/router.rs`; hits rendered with `_index`, `_id`, `_score`, `_source` envelope on the alias path
- [ ] 2.4 Update `src/openapi.rs` with the ES request/response schemas

## 3. Modern pagination: search_after + PIT as one contract (gap-closing: both halves exist separately)
- [ ] 3.1 Accept a `pit: { id, keep_alive }` object in the search body and execute `search_after` against the pinned PIT reader (`crates/lexum-core/src/search/point_in_time.rs` + `search_after.rs` unified path); PIT search without explicit sort gets an implicit unique doc-id tiebreaker so cursors are total-ordered
- [ ] 3.2 Every hit in a PIT/search_after response carries its `sort` values array for cursor continuation
- [ ] 3.3 Mark scroll legacy: doc note + deprecation header on `handlers/scroll.rs` routes; no new scroll capabilities (ES anti-goal F-050 #3)
- [ ] 3.4 Deep-paging integration test: seed 50k+ docs, open PIT, page by 500 via search_after while a concurrent writer adds/deletes docs — assert zero duplicates/skips and a stable snapshot until PIT expiry

## 4. Meilisearch response shaping on the simple path (R-05)
- [ ] 4.1 `attributesToRetrieve` (default `["*"]`) on the simple search request — server-side source filtering reusing the `_source`/`fields` machinery
- [ ] 4.2 `attributesToHighlight` with `highlightPreTag`/`highlightPostTag` (defaults `<em>`/`</em>`) producing `_formatted` output over the existing highlighter (`crates/lexum-core/src/search/highlighter.rs`)
- [ ] 4.3 `attributesToCrop` + `cropLength` (default 10 words) + `cropMarker` (default `…`) via Tantivy SnippetGenerator, composing with highlighting in the same `_formatted` object
- [ ] 4.4 `showMatchesPosition: true` returns `_matchesPosition` (per-attribute array of `{start, length}`) for every hit
- [ ] 4.5 Both pagination styles (exclusive): `offset`/`limit` → `estimatedTotalHits`; `page`/`hitsPerPage` → exact `totalHits` + `totalPages` (drives `track_total_hits` internally); golden-response tests for each param above on both styles

## 5. Typo tolerance with Meilisearch defaults (R-04 — greenfield over existing FuzzyQuery plumbing)
- [ ] 5.1 New typo module in `crates/lexum-core/src/search/` that expands `q` terms to `FuzzyTermQuery` with budgets: len < 5 → 0 typos, 5–8 → 1, ≥ 9 → 2; first-letter typo counts as two (prefix-anchored automaton: `prefix_length=1` for the distance-1 tier and explicit handling so a wrong first letter only matches within a 2-typo budget)
- [ ] 5.2 `typoTolerance` per-index setting object: `enabled` (default true), `minWordSizeForTypos: { oneTypo: 5, twoTypos: 9 }`, `disableOnWords: []`, `disableOnAttributes: []`, `disableOnNumbers: false` — stored per index (storage shape agreed with phase4's settings resource, R-03) and enforced at query-expansion time
- [ ] 5.3 Wire typo expansion into the default `q` path in `handlers/search.rs` (and only there — explicit `term`/`phrase`/DSL queries are never fuzzed); exact matches must always rank at/above their fuzzy variants
- [ ] 5.4 Table-driven tests over word lengths 1–20 asserting the exact budget table, first-letter double-count, numbers skipped when `disableOnNumbers`, per-word and per-attribute disables, and custom `minWordSizeForTypos` overrides

## 6. matchingStrategy (R-10 — greenfield)
- [ ] 6.1 Implement `matchingStrategy` on the simple search path: `last` (default — build BooleanQuery variants dropping query words from the end until results exist), `all` (every word required), `frequency` (drop highest-document-frequency words first, using Tantivy term doc_freq)
- [ ] 6.2 Ensure hits matching more words rank above hits matching fewer (word-count-aware boost of the stricter variants)
- [ ] 6.3 Tests: multi-term query with one nonsense trailing term → non-empty under `last`, empty under `all`; under `frequency` the stopword-like term is dropped first; single-term queries unaffected

## 7. Conformance and regression gates
- [ ] 7.1 DSL conformance fixture suite (new `crates/lexum-server/tests/es_search_parity_test.rs`): every scoped clause + nested bool combinations against a seeded index with asserted hits/order/totals — 100% pass is the gate
- [ ] 7.2 Filter-context scoring test: adding a `filter` clause leaves `_score` values byte-identical
- [ ] 7.3 Simple-door test (A-04): every phase feature (typo, matchingStrategy, shaping, both paginations, sort, filter) exercised through flat `q`+params with no LQL and no DSL body
- [ ] 7.4 Full existing search/scroll/PIT suites in `crates/lexum-server/tests/` stay green; `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt`, `cargo test --all-features` pass

## 8. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 8.1 Update or create documentation covering the implementation
- [ ] 8.2 Write tests covering the new behavior
- [ ] 8.3 Run tests and confirm they pass
