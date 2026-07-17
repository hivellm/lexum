# Proposal: phase4_settings-mappings-analyze

## Why

**R-03** (Meilisearch plan, P0): "Settings as a REST resource with defaults
— Meilisearch's per-index `settings` object (every knob GET/PATCH/resettable,
documented defaults) should be Lexum's model rather than Elasticsearch's
split between mappings, settings, and cluster state" *(F-026, F-037)*. The
Meilisearch parity matrix rows 31-32 rate Lexum 🟡 on settings-as-resource
and ❌ on synonyms/stop words. On the Elasticsearch side, P0 item 4 (F-055)
requires mappings compatibility: "explicit + dynamic mapping, text/keyword
multi-field convention (F-027), `_mapping` GET/PUT, `_analyze`" — with
anti-goal F-050 #2 explicitly warning against ES's unbounded dynamic mapping
("cluster-state death"; ES bolted on `total_fields.limit: 1000` after the
fact).

A code audit (2026-07) shows the split personality is already here, and both
halves have holes:

- `IndexSettings` (`crates/lexum-core/src/index/settings.rs`) is
  infrastructure-only: `number_of_shards` (default **5** — the exact default
  ES itself abandoned in 7.0, anti-goal F-050 #8), `number_of_replicas`,
  `refresh_interval`, `storage`. **None** of the relevance knobs exist: no
  searchableAttributes, filterableAttributes, sortableAttributes,
  displayedAttributes, rankingRules, stopWords, synonyms, typoTolerance,
  pagination.maxTotalHits, or searchCutoffMs.
- The only settings route is `PUT /api/v1/indices/{name}/settings`
  (router.rs:66) — no GET, no PATCH, no reset-to-default.
- **Settings and mappings are not persisted**: `Index` holds them as
  in-memory fields (`index/manager.rs:25-29`); `update_index_settings`
  mutates a HashMap under a lock and never writes to disk. Only Tantivy's
  own `meta.json` survives a restart.
- The ES mapping model (`crates/lexum-core/src/schema/mapping.rs`, 3262
  lines) is genuinely strong — multi-fields, `dynamic: true/false/strict`,
  `dynamic_templates`, `copy_to`, type auto-detection — but `PUT _mapping`
  is a stub returning "Mapping updates are not yet supported"
  (`handlers/mapping.rs:119-125`), and there is **no dynamic-field cap** of
  any kind (`detect_fields_recursive` expands arbitrarily).
- **`_analyze` does not exist** (no route, handler, or request type), and
  analyzer/normalizer names from mappings are parsed but never applied to
  Tantivy — `schema/builder.rs` hardcodes `"default"`/`"raw"` tokenizers and
  no custom tokenizer is ever registered.
- The search handler ignores per-index configuration entirely: a plain query
  fans out over *all* text fields with no boosts (`handlers/search.rs:
  269-284`), there is no field-selection, no result cap, no cutoff budget.
  No stopword or synonym machinery exists anywhere; fuzziness is per-query
  only (`FuzzyQuery`, `query/types.rs:242`) with no index-level defaults.
- Index templates (`index/template.rs`) already stamp settings + mappings at
  creation — the right chassis for stamping the *new* settings object too.

## What Changes

1. **`SearchSettings` object in lexum-core** (new module beside
   `index/settings.rs`): `searchableAttributes` (ordered list = descending
   field boost; default `["*"]`), `filterableAttributes`,
   `sortableAttributes`, `displayedAttributes` (default `["*"]`),
   `rankingRules` (default `["words","typo","proximity","attribute","sort",
   "exactness"]`, initially interpreted as BM25 + declared tie-breakers —
   full bucket semantics deferred to phase2's relevance work),
   `stopWords`, `synonyms`, `typoTolerance` (`enabled`, `minWordSizeForTypos
   {oneTypo: 5, twoTypos: 9}`, `disableOnWords`, `disableOnAttributes`,
   `disableOnNumbers` — Meilisearch's exact defaults per R-04),
   `pagination.maxTotalHits` (default 1000), `searchCutoffMs` (default
   1500). Every knob has a documented default and a `reset` semantic.
2. **Settings as a REST resource.** `GET/PATCH/DELETE
   /api/v1/indices/{index}/settings` (GET returns the *full* object with
   defaults filled in; PATCH is a deep partial update; DELETE resets all) +
   per-sub-setting routes (`.../settings/searchable-attributes` etc., each
   GET/PUT/DELETE). Errors use the uniform error object (R-02, phase1).
3. **Durable settings + mappings.** Persist the settings object and the ES
   mapping to disk beside the index (JSON in the index directory), reload at
   startup — closing the audit's "everything is lost on restart" hole.
4. **Search-time enforcement.** Default query construction uses
   `searchableAttributes` order for field selection + boosts (replacing the
   all-text-fields fan-out); `displayedAttributes` filters returned
   `_source`; filtering/sorting on undeclared attributes returns a
   descriptive uniform error; `pagination.maxTotalHits` caps
   offset+limit/page pagination; `searchCutoffMs` bounds query execution
   (Tantivy time-limited collection); stopWords and synonyms are applied at
   query time (synonym expansion into the boolean query; stopword filtering
   in analysis); typoTolerance drives `FuzzyTermQuery` wiring with the R-04
   thresholds (0 typos < 5 chars, 1 typo 5-8, 2 typos 9+, first-letter typo
   counts double).
5. **ES mappings compatibility hardening.** Implement `PUT _mapping` for
   additive changes (new fields, new multi-fields; reject in-place type
   changes with a clear error pointing at reindex); enforce
   mapping-explosion caps as first-class settings (`mapping.totalFields.
   limit` default 1000, `mapping.depth.limit` default 20, applied to dynamic
   detection and explicit puts — anti-goal F-050 #2); flip the
   `IndexSettings` shard default from 5 to 1 (anti-goal F-050 #8, matching
   `TemplateSettings` which already defaults to 1).
6. **`_analyze` endpoint + real analyzer registration.** `POST /_analyze`
   and `POST /{index}/_analyze` (`{"analyzer"|"field", "text"}` → token
   list with positions/offsets); register a real analyzer set in Tantivy's
   `TokenizerManager` (standard, keyword, whitespace, simple, lowercase +
   rust-stemmers language stemmers, ngram/edge-ngram), and make mapping
   `analyzer` names resolve to registered tokenizers in
   `schema/builder.rs`/`converter.rs` instead of being silently dropped.
7. **Templates stamp settings objects (same shape).** Extend
   `TemplateSettings`/`PutTemplateRequest` so templates carry the full
   `SearchSettings` object; index creation merges template settings by
   priority exactly as mappings merge today (`handlers/index.rs` creation
   path).

Cross-phase dependencies: depends on **phase1** (uniform error object R-02;
settings changes that trigger reindexing become tasks once the queue
exists). Coordinates with **phase2** (search-kernel-parity consumes
typoTolerance defaults and rankingRules interpretation). Feeds **phase5**
(`filterableAttributes` defines facetable fields and drives fast-field
stamping), **phase7** (key-scoped settings actions), and **phase11** (dumps
export the settings object; templates stamp it).

## Impact

- Affected specs: `.rulebook/tasks/phase4_settings-mappings-analyze/specs/`
  (settings-resource spec: full knob list, defaults, PATCH/reset semantics;
  mappings-compat spec: additive PUT rules, caps; analyze spec)
- Affected code: `crates/lexum-core/src/index/settings.rs` (+ new
  `search_settings` module), `crates/lexum-core/src/index/manager.rs`
  (persistence + reload), `crates/lexum-core/src/schema/{mapping.rs,
  builder.rs, converter.rs}`, `crates/lexum-core/src/index/{template.rs,
  template_manager.rs}`, `crates/lexum-core/src/search/executor.rs`,
  `crates/lexum-server/src/handlers/{index.rs, mapping.rs, search.rs,
  template.rs}`, new `handlers/settings.rs` and `handlers/analyze.rs`,
  `crates/lexum-server/src/router.rs`,
  `crates/lexum-server/src/openapi.rs`, `docs/api/API_REFERENCE.md`
- Breaking change: NO for existing endpoints (PUT settings keeps working;
  new routes are additive). Two default changes are called out in the
  changelog: `number_of_shards` default 5 → 1 (metadata-only today), and
  plain-query field selection now honors `searchableAttributes` (default
  `["*"]` preserves current behavior).
- User benefit: every relevance knob is discoverable, documented, and
  resettable through one resource (the Meilisearch DX that made it
  loved); settings survive restarts; ES tooling can read *and* extend
  mappings and inspect analysis; mapping explosion is impossible by
  default.

## Success criteria

- `GET .../settings` on a fresh index returns the complete documented
  default object; `PATCH` of a single knob leaves others untouched;
  `DELETE` restores defaults — all three verified in integration tests,
  including per-sub-setting routes.
- Settings and mappings survive a full server restart (integration test:
  set, restart, get).
- Ordered `searchableAttributes` measurably boosts earlier fields (test: doc
  matching in field 1 ranks above identical match in field 2);
  `displayedAttributes` hides undeclared fields; filter/sort on an
  undeclared attribute returns the uniform error object naming the
  attribute; `maxTotalHits` caps pagination; a pathological query respects
  `searchCutoffMs`.
- Typo tolerance follows R-04 defaults exactly (unit tests per threshold,
  incl. first-letter double-count and disableOnWords/Attributes/Numbers).
- `PUT _mapping` adds a new field that is searchable without reindex;
  in-place type change is rejected with a descriptive error; indexing a
  document that would exceed `mapping.totalFields.limit` (default 1000)
  fails with the uniform error instead of exploding the schema.
- `POST /{index}/_analyze` returns tokens with positions/offsets for every
  registered analyzer; a mapping that names a registered analyzer actually
  tokenizes with it (indexing + search roundtrip test).
- Templates carrying a settings object stamp new indices with it (create
  via template, GET settings, compare).
- `cargo check`, `cargo clippy -- -D warnings`, `cargo fmt --check`, and
  the workspace test suite pass.
