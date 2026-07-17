## 1. Engine swap — full-docset execution via tantivy::aggregation
- [ ] 1.1 Write the failing correctness test first: 10,000 docs, search with `limit: 10` + terms aggregation, assert bucket doc_counts sum to the full matching count (documents that crates/lexum-core/src/search/executor.rs:436-440 currently aggregates only the returned page via `hit.source`)
- [ ] 1.2 Route `search_with_aggregations` through `tantivy::aggregation::AggregationCollector` over the full matching docset on fast fields (single searcher pass alongside top-docs collection), honoring phase4's `search_cutoff_ms` budget
- [ ] 1.3 Translate the existing internally-tagged `AggregationSpec` variants that Tantivy supports (terms, histogram, date_histogram, range; min/max/sum/avg/stats/extended_stats/value_count/cardinality/percentile/top_hits) into Tantivy aggregation requests — the page-scoped `AggregationTrait::execute(hits, ...)` path dies for these types
- [ ] 1.4 Wire sub-aggregations (nesting) through Tantivy's native support so terms → per-bucket metrics works — currently impossible (TermsAggregation has no aggs field)
- [ ] 1.5 Re-run the 1.1 gate plus metric-value equivalence tests (min/max/sum/avg/stats against hand-computed fixtures)

## 2. Facets — Meilisearch shape (A-05)
- [ ] 2.1 Add `facets: Option<Vec<String>>` to `SearchRequest` (crates/lexum-server/src/handlers/search.rs) on both POST and GET search; validate each facet against phase4's `filterable_attributes`, erroring with the setting name otherwise
- [ ] 2.2 Compute `facetDistribution` (value → count per facet, terms agg under the hood, capped by `maxValuesPerFacet` setting, default 100) and `facetStats` (min/max for numeric facets) and return them as top-level response fields matching Meilisearch's documented shape
- [ ] 2.3 Integration tests: distribution counts match a hand-built fixture under an active filter, numeric facets produce facetStats, maxValuesPerFacet truncates, non-filterable facet errors

## 3. Facet-search endpoint (R-14)
- [ ] 3.1 New crates/lexum-server/src/handlers/facet_search.rs: `POST /api/v1/indices/{index}/facet-search` accepting `{facetName, facetQuery, q, filter}` — prefix/contains match over the facet's values within the current query/filter context, returning `{facetHits: [{value, count}], facetQuery, processingTimeMs}`
- [ ] 3.2 Register the route + OpenAPI entry; validate facetName against `filterable_attributes`
- [ ] 3.3 Tests: prefix matching narrows as facetQuery grows, counts respect the `q`/`filter` context, empty facetQuery returns top values by count

## 4. ES-compatible aggregations DSL
- [ ] 4.1 Accept ES-shaped `"aggs"`/`"aggregations"` blocks (`{name: {terms: {field, size}, aggs: {...}}}` — type-keyed, no `"type"` discriminator) on the search endpoint, deserializing into Tantivy's ES-modeled aggregation request types; gate behind phase6's `es_aggregations_dsl` experimental flag (R-12)
- [ ] 4.2 Implement the `filters` bucket (absent in Tantivy) by parsing each filter as a real query and collecting per-filter sub-aggregations — replacing the stub at crates/lexum-core/src/aggregation/filters.rs:51-64 that only counts MatchAll
- [ ] 4.3 Return ES wire-shape responses: `aggregations.{name}.buckets[].{key, key_as_string?, doc_count}` for buckets, `.value`/`.values` for metrics, `doc_count_error_upper_bound` + `sum_other_doc_count` for terms, nested sub-agg results inside their parent buckets
- [ ] 4.4 Compatibility test: an ES 7.x client library (or recorded ES 7.10 response fixtures) parses Lexum responses for terms, date_histogram (fixed + calendar interval), histogram, range, filters, cardinality, percentiles, stats, and terms→avg+top_hits nesting without error

## 5. Retire the untrustworthy surface
- [ ] 5.1 Remove reachable `todo!()` panics (crates/lexum-core/src/aggregation/nested.rs:153,347; reverse_nested.rs:156,300) and the placeholder implementations: all pipeline aggs (bucket_script/selector/sort, cumulative_*, derivative, moving_*, serial_differencing, normalize, pipeline.rs), scripted_metric, and the empty-result geo aggs (geo_bounds, geo_centroid, geo_distance, geohash_grid, geo_line) — each now returns the uniform "aggregation type not supported" error (R-02), never a placeholder number
- [ ] 5.2 Keep the legacy internally-tagged request shape working for one release by mapping it onto the new engine, emitting a deprecation warning in logs and a response header; document the migration in CHANGELOG.md
- [ ] 5.3 Fix the silently-degraded paths in handlers/search.rs: GET search supports `facets` (currently hardcodes `aggregations: None` at search.rs:1002); `search_after` + aggs returns an explicit error (search.rs:412 currently drops them silently)
- [ ] 5.4 Grep gate: no `todo!`/`unimplemented!`/`placeholder` remains in crates/lexum-core/src/aggregation/; delete dead code the engine swap orphaned (page-scoped executor paths, unused `FieldCache` aggregation methods)

## 6. Fast-field plumbing
- [ ] 6.1 Phase4's `filterable_attributes`/`sortable_attributes` stamp `fast: true` on the corresponding schema fields at index build (crates/lexum-core/src/schema/builder.rs — `FieldConfig.fast` exists, defaults false)
- [ ] 6.2 Aggregating or faceting a field without a fast field returns a descriptive uniform error naming the field and the setting to declare (instead of empty/wrong results)
- [ ] 6.3 Test: field declared filterable aggregates correctly; same field undeclared errors; existing indices without fast fields get the error, not a panic

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
