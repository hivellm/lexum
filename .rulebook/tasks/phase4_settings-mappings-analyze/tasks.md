## 1. SearchSettings model + persistence (lexum-core)
- [ ] 1.1 New `search_settings` module beside crates/lexum-core/src/index/settings.rs defining `SearchSettings` with every knob and documented defaults: `searchable_attributes` (ordered, default `["*"]`), `filterable_attributes`, `sortable_attributes`, `displayed_attributes` (default `["*"]`), `ranking_rules` (default `["words","typo","proximity","attribute","sort","exactness"]`), `stop_words`, `synonyms` (map word → [synonyms]), `typo_tolerance` (enabled, minWordSizeForTypos {oneTypo: 5, twoTypos: 9}, disableOnWords, disableOnAttributes, disableOnNumbers), `pagination.max_total_hits` (default 1000), `search_cutoff_ms` (default 1500), `mapping.total_fields_limit` (default 1000), `mapping.depth_limit` (default 20)
- [ ] 1.2 Deep-merge PATCH semantics (`apply_update`) and per-knob `reset()` returning documented defaults, with unit tests for partial update, null-resets, and idempotence
- [ ] 1.3 Persist `SearchSettings` + the ES mapping (`ElasticsearchMapping`) as JSON beside each index directory; `IndexManager` writes on every mutation and reloads both at startup (crates/lexum-core/src/index/manager.rs — today `update_index_settings` mutates memory only and everything is lost on restart)
- [ ] 1.4 Flip `IndexSettings::default().number_of_shards` from 5 to 1 (crates/lexum-core/src/index/settings.rs:52, anti-goal F-050 #8; aligns with `TemplateSettings` default 1) and note it in CHANGELOG.md

## 2. Settings REST resource
- [ ] 2.1 New crates/lexum-server/src/handlers/settings.rs: `GET /api/v1/indices/{index}/settings` (full object, defaults filled in), `PATCH` (deep partial update), `DELETE` (reset all) — errors via the uniform error object (R-02/phase1)
- [ ] 2.2 Per-sub-setting routes, each GET/PUT/DELETE: `.../settings/searchable-attributes`, `filterable-attributes`, `sortable-attributes`, `displayed-attributes`, `ranking-rules`, `stop-words`, `synonyms`, `typo-tolerance`, `pagination`, `search-cutoff-ms`
- [ ] 2.3 Keep the legacy `PUT .../settings` (infra knobs) working; register all routes in router.rs and document every knob + default in openapi.rs
- [ ] 2.4 Integration tests: fresh-index GET returns complete defaults; PATCH one knob leaves the rest; DELETE restores defaults; settings survive server restart

## 3. Search-time enforcement
- [ ] 3.1 Default query construction honors `searchable_attributes`: ordered field list with descending boosts replaces the all-text-fields fan-out in crates/lexum-server/src/handlers/search.rs:269-284 and 910-921 (`["*"]` preserves current behavior)
- [ ] 3.2 `displayed_attributes` filters returned `_source`; request-time `fields` param intersects with it
- [ ] 3.3 Filtering/sorting on attributes not declared filterable/sortable returns a descriptive uniform error naming the attribute and the setting to change
- [ ] 3.4 `pagination.max_total_hits` caps offset+limit and page/hitsPerPage traversal; `search_cutoff_ms` bounds execution via time-limited collection in crates/lexum-core/src/search/executor.rs, returning partial results with a `degraded`/timeout marker
- [ ] 3.5 Tests: boost ordering (match in attr 1 outranks identical match in attr 2), hidden fields, undeclared-attribute errors, hit cap, cutoff on a pathological query

## 4. Stop words, synonyms, typo tolerance
- [ ] 4.1 Query-time stopword removal using the per-index `stop_words` list (skip when it would empty the query)
- [ ] 4.2 Query-time synonym expansion: rewrite matched terms into a boolean `should` over synonyms (one-way and mutual mappings), covered by relevance tests
- [ ] 4.3 Wire `typo_tolerance` to Tantivy `FuzzyTermQuery` with R-04 defaults: 0 typos < 5 chars, 1 typo 5-8, 2 typos 9+, first-letter typo counts double, honoring disableOnWords/disableOnAttributes/disableOnNumbers (extends the existing per-query fuzzy machinery in crates/lexum-core/src/query/types.rs and search/executor.rs:578-591); coordinate defaults with phase2 (search-kernel-parity)
- [ ] 4.4 Unit tests per threshold boundary (4/5/8/9 chars), first-letter case, and each disable list

## 5. ES mappings hardening
- [ ] 5.1 Implement `PUT /{index}/_mapping` for additive changes — new fields and new multi-fields become searchable without reindex; reject in-place type changes with a clear error pointing at `_reindex` (replaces the stub at crates/lexum-server/src/handlers/mapping.rs:119-125)
- [ ] 5.2 Enforce `mapping.total_fields_limit` and `mapping.depth_limit` in dynamic detection (`detect_fields_recursive` in crates/lexum-core/src/schema/mapping.rs) and explicit mapping puts — indexing/putting past the cap fails with the uniform error (anti-goal F-050 #2)
- [ ] 5.3 Verify the text/keyword multi-field convention end-to-end: dynamic string detection produces `text` + `.keyword` (ignore_above 256) sub-field, both queryable (F-027)
- [ ] 5.4 Tests: additive PUT roundtrip, type-change rejection, cap enforcement at the limit boundary, multi-field dynamic detection

## 6. _analyze endpoint + real analyzers
- [ ] 6.1 Register a real analyzer set in Tantivy's `TokenizerManager` at index open (crates/lexum-core/src/schema/builder.rs — today only hardcoded `"default"`/`"raw"`): standard, keyword, whitespace, simple, lowercase, rust-stemmers language stemmers (at least english/french/german/spanish/portuguese), ngram + edge-ngram
- [ ] 6.2 Mapping `analyzer` names resolve to registered tokenizers in builder.rs/converter.rs instead of being parsed-then-dropped; unknown analyzer on mapping put → uniform error listing available analyzers
- [ ] 6.3 New crates/lexum-server/src/handlers/analyze.rs: `POST /_analyze` and `POST /{index}/_analyze` accepting `{"analyzer"|"field", "text"}` and returning ES-shaped `{"tokens": [{token, start_offset, end_offset, position, type}]}`
- [ ] 6.4 Tests: token output per analyzer, field-resolution path (`"field"` uses the mapping's analyzer), index+search roundtrip proving a named analyzer actually tokenizes stored data

## 7. Templates stamp settings objects
- [ ] 7.1 Extend `TemplateSettings`/`PutTemplateRequest` (crates/lexum-core/src/index/template.rs, crates/lexum-server/src/handlers/template.rs) to carry the full `SearchSettings` object — same shape as the settings resource
- [ ] 7.2 Index creation merges template-stamped settings by priority alongside the existing mapping merge (crates/lexum-server/src/handlers/index.rs creation path), request-level settings winning over templates
- [ ] 7.3 Test: create index via matching template, GET settings equals the stamped object with defaults filled; two overlapping templates merge by priority

## 8. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 8.1 Update or create documentation covering the implementation
- [ ] 8.2 Write tests covering the new behavior
- [ ] 8.3 Run tests and confirm they pass
