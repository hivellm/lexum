# Proposal: phase5_aggregations-facets

## Why

Aggregations are the "highest-leverage gap" in the elastic parity matrix and
P0 item 5 of the ES execution plan (F-055): "`terms`, `date_histogram`,
`histogram`, `range`, `filters` buckets; `min/max/sum/avg/stats/cardinality/
percentiles/top_hits` metrics; nesting. (Tantivy's ES-modeled aggregation
module is the shortcut, F-017.)" The Meilisearch plan simultaneously warns
**A-05**: "Don't ship aggregations Elasticsearch-style — Meilisearch
deliberately offers only facet counts + stats and remains dramatically
easier to use. Start with the facet-distribution/stats model" *(F-013)*, and
**R-14** calls for a facet-search endpoint (search-as-you-type over facet
values) + numeric facet stats, "cheap with Tantivy's aggregations and vital
for e-commerce UIs".

A code audit (2026-07) found that Lexum's existing aggregation framework
(`crates/lexum-core/src/aggregation/`, ~54 files, ~50 types) is structurally
elaborate but **must not be trusted as-is**:

- **Correctness flaw:** aggregations execute over *only the returned page of
  hits* — `search/executor.rs:436-440` passes `result.hits` (bounded by
  `limit`, default 10) to the executor, and every agg iterates
  `hit.source` JSON. Counts, sums, and stats silently reflect ~10 documents
  instead of the full matching set. This is wrong in the way that loses
  user trust permanently.
- It does **not** use `tantivy::aggregation` at all (zero references) and
  ignores fast fields — `FieldCache` is dead code for aggregations.
- Large parts are stubs: the `filters` bucket never evaluates queries
  (filters.rs:51-64, only `MatchAll` counts), all pipeline aggs are
  placeholders that return their input, `scripted_metric` returns a
  hardcoded sum, geo aggs return empty results, and `nested.rs:153,347` /
  `reverse_nested.rs` contain `todo!()` panics reachable at runtime.
- The common bucket aggs (`terms`, `histogram`, `range`, `date_histogram`)
  have **no sub-aggregation support** — the canonical ES "terms → metric per
  bucket" pattern is impossible.
- The request grammar is not ES-compatible (internally-tagged
  `{"type": "terms", ...}` under an `aggregations` key vs ES's
  `{"aggs": {name: {terms: {...}}}}`), the GET-search path hardcodes
  `aggregations: None`, and `search_after` silently drops them.
- **No facet capability exists at all**: no `facets` param, no
  facetDistribution/facetStats, no facet-search endpoint (grep across
  lexum-server: zero hits).

Tantivy 0.25 ships an ES-modeled aggregation module (serde types mirroring
the ES DSL, executed over fast fields across the full matching docset, with
sub-aggregation support) — replacing the page-scoped custom executor with it
is simultaneously the correctness fix and the ES-compatibility shortcut.

## What Changes

1. **Swap the execution engine (correctness first).** Route aggregation
   execution through `tantivy::aggregation` (`AggregationCollector` over the
   full matching docset on fast fields) instead of the page-of-hits custom
   executor. Supported types delegate; the page-scoped path dies.
2. **Facet-first API (A-05, the default door).** A `facets` parameter on the
   search endpoints returning Meilisearch-shaped `facetDistribution`
   (value → count per requested facet, capped by `maxValuesPerFacet`,
   default 100) and `facetStats` (min/max for numeric facets) — implemented
   as terms + stats aggregations under the hood. Facetable fields are the
   `filterableAttributes` from phase4's settings object.
3. **Facet search endpoint (R-14).** `POST
   /api/v1/indices/{index}/facet-search` with `{facetName, facetQuery, q,
   filter}`: search-as-you-type over facet values, returning matching
   `facetHits` with counts, respecting the main query/filter context.
4. **ES-compatible aggs DSL.** Accept ES-shaped `"aggs"`/`"aggregations"`
   blocks (`{name: {terms: {...}, aggs: {...}}}`) on the search endpoint,
   translating directly to Tantivy's aggregation request types: `terms`,
   `histogram`, `date_histogram`, `range` buckets and `min/max/sum/avg/
   stats/extended_stats/count/cardinality/percentiles/top_hits` metrics,
   with nesting. Implement the `filters` bucket (absent in Tantivy) as N
   filtered sub-collections over parsed queries — actually evaluating
   queries, unlike the current stub. Responses use ES wire shape
   (`aggregations.{name}.buckets[].{key,doc_count}` / `.value`), including
   `doc_count_error_upper_bound`/`sum_other_doc_count` for terms. Ship the
   ES DSL behind phase6's `es_aggregations_dsl` experimental flag until
   shapes are validated against real ES clients (R-12).
5. **Retire the untrustworthy surface.** Delete or quarantine the stubbed
   custom types (pipeline placeholders, scripted_metric, geo stubs,
   `todo!()` panic paths); keep the legacy internally-tagged request shape
   working for one release where it maps 1:1 onto the new engine, with a
   deprecation warning; anything that cannot be computed correctly returns
   a "not supported" uniform error instead of a silently wrong number.
6. **Fast-field plumbing.** Aggregatable/facetable fields require Tantivy
   fast fields: phase4's `filterableAttributes`/`sortableAttributes` stamp
   `fast: true` in the schema (`schema/builder.rs` already supports it,
   default false); aggregating a non-fast field returns a descriptive error
   naming the setting. Fix the dropped-aggregation paths: GET search gains
   `facets`; `search_after` + aggs returns an explicit error (ES semantics:
   aggregations belong on the first page).

Cross-phase dependencies: depends on **phase4** (`filterableAttributes`
defines facetable fields and stamps fast fields; uniform errors via
phase1 R-02) and **phase6** (`es_aggregations_dsl` experimental flag).
Coordinates with **phase2** (the ES `_search` endpoint carries the same
`aggs` block). Feeds **phase8** (multi-search/federation must merge
facetDistribution and aggregation results across queries/shards) and
**phase9** (distributed aggregation merging reuses Tantivy's intermediate
aggregation results, which are designed for cross-segment/cross-shard
merges).

## Impact

- Affected specs: `.rulebook/tasks/phase5_aggregations-facets/specs/`
  (facets spec: request/response shapes, maxValuesPerFacet; aggregations
  spec: supported types, ES wire-shape mapping, unsupported-type errors)
- Affected code: `crates/lexum-core/src/aggregation/` (executor swap,
  stub retirement, ES-shape translation), `crates/lexum-core/src/search/
  executor.rs` (search_with_aggregations, time-limited full-docset
  collection), `crates/lexum-core/src/search/result.rs`,
  `crates/lexum-core/src/schema/builder.rs` (fast-field stamping),
  `crates/lexum-server/src/handlers/search.rs` (facets param, ES aggs
  parsing, GET/search_after paths), new `handlers/facet_search.rs`,
  `crates/lexum-server/src/router.rs`, `crates/lexum-server/src/openapi.rs`,
  `crates/lexum-core/tests/aggregation_integration_tests.rs`
- Breaking change: YES (documented): aggregation results change from
  page-scoped to full-matching-set values — the old numbers were wrong, and
  correct results are the breaking change; unsupported stub types now error
  instead of returning placeholders. The legacy request *shape* keeps
  working for one release with deprecation warnings.
- User benefit: aggregation numbers become correct; e-commerce/instant-search
  UIs get Meilisearch-grade facets + facet search; Kibana-style tooling and
  ES clients get real ES-shaped aggregations; the framework stops panicking
  (`todo!()`) on exotic nesting.

## Success criteria

- Correctness gate: index 10,000 docs, search with `limit: 10` + a terms
  aggregation → bucket doc_counts sum to the full matching count, not 10
  (this test fails on the current engine before the swap and passes after).
- `facets` param returns `facetDistribution` + `facetStats` matching
  Meilisearch's documented response shape; `maxValuesPerFacet` respected;
  facet on a non-filterable attribute errors with the setting named.
- Facet-search endpoint returns prefix-matched `facetHits` with counts,
  constrained by `q`/`filter`, in < 50 ms on the 10k-doc fixture.
- ES-shaped request with nested aggs (`terms` → `avg` + `top_hits` per
  bucket, plus `date_histogram`, `histogram`, `range`, `filters`,
  `cardinality`, `percentiles`, `stats`) returns ES wire-shape responses
  that an ES 7.x client library parses without error; `filters` buckets
  actually evaluate their queries.
- No `todo!()`/`unimplemented!()` remains reachable in
  `crates/lexum-core/src/aggregation/`; every retired type returns the
  uniform "not supported" error, never a placeholder number.
- Legacy-shape requests still execute (mapped to the new engine) and emit a
  deprecation warning in the response/logs.
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and the
  workspace test suite pass, including rewritten
  `aggregation_integration_tests.rs` asserting full-docset semantics.
