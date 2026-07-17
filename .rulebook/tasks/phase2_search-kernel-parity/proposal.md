# Proposal: phase2_search-kernel-parity

## Why

The Elasticsearch execution plan defines the P0 compatibility kernel
(F-055): a small set of search primitives that "make Lexum usable by the
existing ES ecosystem before any distribution work ships" — the core Query
DSL with correct query/filter context (P0 #2, F-025/F-026),
`search_after` + PIT pagination (P0 #6, F-018, skipping legacy scroll).
The Meilisearch plan supplies the other half of a modern search kernel:
relevancy UX — typo tolerance with proven thresholds (R-04, F-012),
response shaping (R-05, F-025), and `matchingStrategy` (R-10, F-028) —
and warns that the default door must need no query language at all (A-04).

Lexum is closer than the parity matrices assume, but the gaps are real:

- `crates/lexum-core/src/query/types.rs` already has a rich `Query` enum
  (Match, Term, Range, Bool with must/should/must_not/filter, Fuzzy,
  Phrase, Wildcard, Regex, MultiMatch, SimpleQueryString, QueryString…) —
  but in a Lexum-native snake_case wire shape, NOT the ES Query DSL wire
  shape (`{"match": {"title": {"query": "..."}}}`). No ES client or tool
  can talk to it. `terms` (value set), `exists`, `ids`, and first-class
  `prefix` variants are missing entirely.
- `crates/lexum-server/src/handlers/search.rs` `SearchRequest` already has
  `q`, `filter`, `limit/offset`, `sort`, `search_after`, `fields`,
  `highlight`, `min_score`, `aggregations` — the simple door exists (A-04
  is satisfiable), but there is no `from/size`, `_source`,
  `track_total_hits`, ES-shaped `highlight`, and no Meilisearch-shaped
  response params (`attributesToHighlight/Crop/Retrieve`, `cropLength`,
  `cropMarker`, `showMatchesPosition`, `page/hitsPerPage`).
- `search_after` (`crates/lexum-core/src/search/search_after.rs`) and PIT
  (`search/point_in_time.rs`, `handlers/point_in_time.rs`) exist but are
  separate paths; PIT + search_after + deterministic tiebreak as one
  contract (the ES 7.10+ pagination story) is not wired. Scroll exists
  (`handlers/scroll.rs`) — per F-050 anti-goal #3 it stays frozen/legacy.
- Typo tolerance: `FuzzyQuery` exists (flat default distance 2) and
  MultiMatch has `fuzzy_*` knobs, but nothing implements Meilisearch's
  proven word-length heuristics (R-04), nothing applies fuzziness on the
  default `q` path, and there is no `typoTolerance` setting. Parity matrix
  row 3: ❌ — part of the top miss cluster (F-038).
- `matchingStrategy` (parity row 35): ❌ — trivial over Tantivy
  BooleanQuery, huge for search-as-you-type (never zero results mid-word).

## What Changes

1. **ES-compatible `_search` wire adapter.** A DSL layer that parses the
   ES 7.10 query grammar into core `Query`: `bool` (with correct
   query-vs-filter context — filter clauses scoreless and cache-eligible),
   `match`/`multi_match`/`match_phrase`, `term`/`terms`/`range`/`exists`/
   `prefix`/`wildcard`/`ids`, `match_all`. New core variants `Terms`,
   `Exists`, `Ids`, `Prefix` added to `query/types.rs` +
   `query/builder.rs`. The native Lexum shape keeps working unchanged.
2. **ES body params on `_search`**: `from`/`size` (aliasing offset/limit),
   ES-shaped `sort`, `_source` (bool / list / includes+excludes),
   `track_total_hits` (true / false / integer threshold), ES-shaped
   `highlight` mapped onto the existing `HighlightConfig`.
3. **Modern pagination as one contract**: `search_after` + PIT combined
   (PIT id accepted in the search body, implicit doc-id tiebreaker so
   cursors are deterministic), on both the ES path and the native path.
   Scroll endpoints remain but are documented legacy-frozen (F-050 #3);
   no new scroll features.
4. **Meilisearch response shaping on the simple path (R-05)**:
   `attributesToRetrieve`, `attributesToHighlight` (pre/post tags),
   `attributesToCrop` + `cropLength` + `cropMarker`,
   `showMatchesPosition` (positions per matched term), and BOTH
   pagination styles — `offset/limit` (estimatedTotalHits) and
   `page/hitsPerPage` (totalHits/totalPages) — over Tantivy's
   SnippetGenerator and `crates/lexum-core/src/search/highlighter.rs`.
5. **Typo tolerance with Meilisearch's exact defaults (R-04)** over
   Tantivy `FuzzyTermQuery`: 0 typos for words < 5 chars, 1 typo for 5–8,
   2 typos for 9+; a wrong FIRST letter counts as two typos;
   `typoTolerance` setting object with `enabled`,
   `minWordSizeForTypos.oneTypo/twoTypos`, `disableOnWords`,
   `disableOnAttributes`, `disableOnNumbers`. Applied on the `q` path by
   default. Copying these numbers buys years of Meilisearch relevance
   tuning for free (F-012).
6. **`matchingStrategy` = `last` (default) / `all` / `frequency` (R-10)**
   on the simple search path: under `last`, terms are relaxed from the end
   of the query until results exist; `frequency` drops the
   highest-document-frequency terms first.
7. **The simple `q`+`filter`+`sort` door stays first-class (A-04)** —
   every feature above is reachable with flat parameters and zero LQL/DSL;
   LQL remains the power layer, never the only door.

## Impact

- Affected specs: `specs/search-kernel/spec.md`,
  `specs/typo-tolerance/spec.md` (this task)
- Affected code: `crates/lexum-core/src/query/types.rs`,
  `query/builder.rs`, new `query/es_dsl.rs` (wire adapter),
  `crates/lexum-core/src/search/` (executor.rs, search_after.rs,
  point_in_time.rs, highlighter.rs, new typo + matching-strategy modules),
  `crates/lexum-server/src/handlers/search.rs`,
  `handlers/point_in_time.rs`, `handlers/scroll.rs` (freeze note),
  `src/router.rs` (ES-compatible `_search` route alias), `src/openapi.rs`
- Breaking change: NO — purely additive; the native query shape, existing
  params, and scroll keep working. One default changes (typo tolerance ON
  for the `q` path) — called out in the CHANGELOG with an off switch.
- User benefit: ES 7.10-era clients and tools can issue core searches
  against Lexum, and instant-search UIs get Meilisearch-grade typo
  tolerance, highlight/crop shaping, and never-zero-results typing —
  closing the read-side halves of both parity-matrix miss clusters
  (F-038, F-048).

## Dependencies / sequencing

- Independent of phase1 (read path only) — may proceed in parallel with
  the task-queue work.
- `typoTolerance` and `matchingStrategy` are per-index settings: define
  the sub-objects here, storage shape coordinated with
  phase4_settings-mappings-analyze's settings-as-resource model (R-03),
  which mounts them under GET/PATCH `/settings`.
- phase8_multisearch-federation reuses this kernel's response shaping and
  pagination contract unchanged; phase5 aggregations plug into the same
  `_search` body.

## Success criteria (gates)

- DSL conformance suite: a fixture corpus of ES-shaped queries covering
  every clause in scope (incl. nested `bool` mixing query and filter
  context) runs against a seeded index and asserts hits, order, and
  totals; 100% of the scoped grammar passes.
- Filter context provably does not affect scoring: identical `_score`s
  with and without an added `filter` clause, asserted in tests.
- Deep-paging gate: walk ≥50k documents via `search_after` + PIT while
  concurrent writes land — zero duplicates, zero skips (stable snapshot);
  `from/size` and `page/hitsPerPage` return consistent totals.
- Typo gates (table-driven over word lengths 1–20): exact 0/1/2 budgets at
  <5, 5–8, ≥9 chars; first-letter typo on a 5–8 char word does NOT match
  (counts double); `disableOnWords`/`disableOnAttributes`/
  `disableOnNumbers` each verified; defaults overridable via
  `minWordSizeForTypos`.
- `matchingStrategy`: a multi-term query whose last term matches nothing
  returns non-empty results under `last`, empty under `all`, and drops the
  most frequent term first under `frequency` — all tested.
- Response shaping: golden-response tests for `attributesToCrop` +
  `cropLength` + `cropMarker`, `attributesToHighlight`,
  `attributesToRetrieve`, `showMatchesPosition`, on both pagination
  styles.
- Zero regressions in the existing native search suites
  (`crates/lexum-server/tests/`); `cargo clippy -- -D warnings` and
  `cargo fmt` clean.
