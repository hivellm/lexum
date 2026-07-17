# SPEC-006 — Settings & Mappings

| | |
|---|---|
| **Status** | Draft |
| **Phase / tasks** | Phase 4 · phase4_settings-mappings-analyze §1–§8 ([tasks](../../.rulebook/tasks/phase4_settings-mappings-analyze/tasks.md)) |
| **Planning source** | [Meilisearch plan R-03, R-04 (F-012, F-026, F-037)](../analysis/meilisearch/08-execution-plan.md) · [Elastic plan P0 #4 (F-027, F-050 #2/#8, F-055)](../analysis/elastic/08-execution-plan.md) · phase4 code audit (settings not persisted; `_analyze` absent; analyzer names parsed-then-dropped; no dynamic-field cap) |

Requirement IDs `SET-xxx`. RFC 2119 keywords are normative. JSON wire names are camelCase. Errors use the uniform error object of SPEC-003. Search-time behavior driven by these settings is specified in SPEC-004; facetable-field stamping feeds SPEC-007; dumps export this object per SPEC-013.

## 1. Model

Each index owns exactly one **settings object** — the Meilisearch-style single resource holding every relevance and safety knob — plus one **ES-compatible mapping** describing fields. The two are complementary, not redundant:

1. The settings object is the *behavioral* contract: which fields are searchable/filterable/sortable/displayed, ranking, typo tolerance, stop words, synonyms, pagination and time budgets, mapping-explosion caps.
2. The mapping is the *structural* contract: field names, types, analyzers, multi-fields — readable and (additively) writable by ES tooling.

- **SET-001** Every knob MUST have a documented default (§2), MUST be individually retrievable, patchable, and resettable (§3), and MUST be persisted durably (§5). A fresh index with no explicit settings behaves identically to one whose settings were explicitly set to the defaults.
- **SET-002** Unknown keys in any settings write body MUST be rejected with a SPEC-003 error naming the offending key (no silent-ignore, no dynamic extension of the object).

## 2. The settings object

`GET .../settings` returns the full object with defaults filled in — never a sparse object:

```json
{
  "searchableAttributes": ["*"],
  "filterableAttributes": [],
  "sortableAttributes": [],
  "displayedAttributes": ["*"],
  "rankingRules": ["words", "typo", "proximity", "attribute", "sort", "exactness"],
  "stopWords": [],
  "synonyms": {},
  "typoTolerance": {
    "enabled": true,
    "minWordSizeForTypos": { "oneTypo": 5, "twoTypos": 9 },
    "disableOnWords": [],
    "disableOnAttributes": [],
    "disableOnNumbers": false
  },
  "pagination": { "maxTotalHits": 1000 },
  "searchCutoffMs": 1500,
  "mapping": { "totalFieldsLimit": 1000, "depthLimit": 20 },
  "defaultPipeline": null
}
```

### 2.1 Defaults and semantics (normative)

| Knob | Type | Default | Semantics |
|---|---|---|---|
| `searchableAttributes` | ordered `[string]` | `["*"]` | Fields searched by default queries. **Order is rank**: earlier attributes carry a higher field boost (descending). `["*"]` = all text fields, equal weight (preserves pre-SPEC behavior). |
| `filterableAttributes` | `[string]` | `[]` | Attributes allowed in `filter` expressions. Also defines facetable fields (SPEC-007) and stamps Tantivy fast fields. |
| `sortableAttributes` | `[string]` | `[]` | Attributes allowed in `sort`. Stamps fast fields. |
| `displayedAttributes` | `[string]` | `["*"]` | Attributes returned in hit `_source`. Request-time field selection intersects with this set. |
| `rankingRules` | ordered `[string]` | `["words","typo","proximity","attribute","sort","exactness"]` | Interpreted initially as BM25 plus declared tie-breakers; full bucket semantics per SPEC-004. |
| `stopWords` | `[string]` | `[]` | Removed from queries at query time. Removal MUST be skipped when it would empty the query. |
| `synonyms` | `{word: [string]}` | `{}` | Query-time expansion into a boolean `should` (one-way per entry; mutual synonymy = two entries). |
| `typoTolerance.enabled` | bool | `true` | Master switch for fuzzy matching. |
| `typoTolerance.minWordSizeForTypos.oneTypo` | int | `5` | 0 typos allowed below 5 chars; 1 typo at 5–8 (R-04). |
| `typoTolerance.minWordSizeForTypos.twoTypos` | int | `9` | 2 typos at 9+ chars. A first-letter typo counts double (R-04). |
| `typoTolerance.disableOnWords` | `[string]` | `[]` | Exact-match-only words. |
| `typoTolerance.disableOnAttributes` | `[string]` | `[]` | Exact-match-only attributes. |
| `typoTolerance.disableOnNumbers` | bool | `false` | When `true`, numeric tokens never fuzzy-match. |
| `pagination.maxTotalHits` | int | `1000` | Hard cap on `offset + limit` and on `page × hitsPerPage` traversal. Exceeding requests are clamped, and the response indicates exhaustiveness accordingly (SPEC-004). |
| `searchCutoffMs` | int (ms) | `1500` | Query execution budget via time-limited collection. On cutoff the response MUST return partial results with a `degraded`/timeout marker — never a 5xx. |
| `mapping.totalFieldsLimit` | int | `1000` | Hard cap on total mapped fields (explicit + dynamic + multi-fields). See SET-041. |
| `mapping.depthLimit` | int | `20` | Hard cap on object-nesting depth for dynamic detection and explicit puts. |
| `defaultPipeline` | string \| null | `null` | Ingest pipeline applied to writes when no `?pipeline=` param is given (SPEC-013 §5). |

- **SET-010** The defaults in the table above are normative. Changing a default is a breaking change and MUST be called out in `CHANGELOG.md`.
- **SET-011** Typo-tolerance thresholds MUST implement R-04 exactly: 0 typos for words < 5 chars, 1 typo for 5–8, 2 typos for ≥ 9, first-letter typo counted as two edits, and the three `disableOn*` escape hatches honored.
- **SET-012** Referencing an attribute in `filter`/`sort`/`facets` that is not declared in the corresponding `*Attributes` list MUST return a SPEC-003 error that names both the attribute and the setting to change (e.g. `invalid_search_filter`, "attribute `price` is not filterable; add it to `filterableAttributes`").

### 2.2 Infrastructure settings (legacy `IndexSettings`)

The pre-existing infra knobs remain a separate section, still writable via the legacy `PUT .../settings` body:

| Knob | Default | Note |
|---|---|---|
| `number_of_shards` | **1** | Flipped from 5 (anti-goal F-050 #8 — ES itself abandoned default-5 in 7.0; growth story is rollover, SPEC-013). Metadata-only until SPEC-011. |
| `number_of_replicas` | 1 | Metadata-only until SPEC-011. |
| `refresh_interval` | 1 s | Consumed by the write path (SPEC-002/SPEC-005 `wait_for`). |
| `storage.enable_memory_mapped_storage` | true | Existing behavior (`crates/lexum-core/src/index/settings.rs`). |

- **SET-015** `IndexSettings::default().number_of_shards` MUST be 1. Existing indices keep their stored value.

## 3. Routes

Base: `/api/v1/indices/{index}/settings`.

| Route | Methods | Semantics |
|---|---|---|
| `.../settings` | `GET` | Full object, defaults filled in (§2). |
| `.../settings` | `PATCH` | **Deep partial update**: only present keys change; `null` for a key resets that key to its default; absent keys are untouched. Returns the resulting full object. |
| `.../settings` | `DELETE` | Reset the entire object to defaults. |
| `.../settings` | `PUT` | Legacy infra-knob write (§2.2) — retained, unchanged shape. |
| `.../settings/{sub}` | `GET` / `PUT` / `DELETE` | Per-sub-setting: get the value, replace it wholesale, reset it to default. |

- **SET-020** Sub-setting path segments are kebab-case: `searchable-attributes`, `filterable-attributes`, `sortable-attributes`, `displayed-attributes`, `ranking-rules`, `stop-words`, `synonyms`, `typo-tolerance`, `pagination`, `search-cutoff-ms`. Each MUST support all three methods.
- **SET-021** PATCH MUST be idempotent and MUST NOT clobber sibling keys (test: patch one knob, all others byte-identical). PATCH of an invalid value (wrong type, unknown ranking rule, negative cutoff) MUST fail atomically — no partial application.
- **SET-022** All settings routes MUST return SPEC-003 errors: `index_not_found` (404), `invalid_settings_<sub>` (400) with the offending key/value named.
- **SET-023** Settings mutations that trigger reindexing (§7) ride the SPEC-002 task queue once it exists: the write returns a `taskUid`; until then they apply synchronously.

## 4. ES mappings compatibility

The existing mapping model (`crates/lexum-core/src/schema/mapping.rs`: multi-fields, `dynamic: true|false|strict`, `dynamic_templates`, `copy_to`, type auto-detection) is the substrate; this section hardens it.

- **SET-030** `GET /{index}/_mapping` MUST return the effective mapping including dynamically-detected fields, in ES 7.x wire shape (`{index: {mappings: {properties: {...}}}}`).
- **SET-031** `PUT /{index}/_mapping` MUST accept **additive** changes only: new fields and new multi-fields on existing fields become searchable without reindex. An in-place type change (or analyzer change on an existing field) MUST be rejected with a SPEC-003 error that points at `_reindex` as the remedy. (Replaces the current stub at `crates/lexum-server/src/handlers/mapping.rs:119-125`.)
- **SET-032** Dynamic string detection MUST produce the ES multi-field convention: `text` field + `.keyword` sub-field with `ignore_above: 256`, both queryable (F-027).
- **SET-033** Analyzer/normalizer names in mappings MUST resolve to tokenizers actually registered in Tantivy's `TokenizerManager` (§5 of phase4: standard, keyword, whitespace, simple, lowercase, language stemmers, ngram, edge-ngram). Naming an unregistered analyzer on a mapping put MUST fail with a SPEC-003 error listing the available analyzers. Silently dropping the name (current `schema/builder.rs` behavior) is forbidden.

### 4.1 Mapping-explosion caps

- **SET-040** Dynamic mapping MUST be bounded (anti-goal F-050 #2). Unbounded field detection (`detect_fields_recursive` today) is forbidden.
- **SET-041** `mapping.totalFieldsLimit` (default 1000) counts every mapped field: explicit, dynamic, and multi-field sub-fields. Indexing a document — or putting a mapping — that would exceed the cap MUST fail with a SPEC-003 error stating the limit and the setting name; the schema MUST NOT be partially extended.
- **SET-042** `mapping.depthLimit` (default 20) bounds object nesting for both dynamic detection and explicit puts, with the same failure contract.

## 5. Persistence

The phase4 audit's central finding: settings and mappings live only in memory today (`Index` fields in `index/manager.rs`; `update_index_settings` mutates a HashMap) — everything but Tantivy's `meta.json` is lost on restart. This is non-conformant.

- **SET-050** The settings object (§2), infra settings (§2.2), and the ES mapping MUST be persisted as JSON beside the index directory on **every** mutation, before the mutating request is acknowledged.
- **SET-051** Persistence MUST be atomic per mutation (write temp file + rename, or equivalent): a crash mid-write leaves either the previous or the new settings file, never a torn one.
- **SET-052** `IndexManager` MUST reload settings + mappings for every index at startup. A missing settings file (pre-upgrade index) loads as the default object. A corrupt settings file MUST fail loudly at startup with the file path — not silently fall back to defaults.
- **SET-053** Round-trip invariant: `GET .../settings` after a full server restart returns byte-equivalent JSON to the pre-restart `GET` (integration-tested per SPEC-016).

## 6. `_analyze` endpoint

- **SET-060** `POST /_analyze` and `POST /{index}/_analyze` MUST exist. Request body: `{"analyzer": "<name>", "text": "..."}` or (index-scoped only) `{"field": "<field>", "text": "..."}` — `field` resolves the analyzer through the index mapping. `text` MAY be a string or an array of strings.
- **SET-061** Response is ES-shaped:

```json
{ "tokens": [ { "token": "quick", "start_offset": 4, "end_offset": 9, "position": 1, "type": "word" } ] }
```

Offsets are byte offsets into the input as UTF-8 (matching Tantivy); `position` is the token position ordinal.

- **SET-062** Unknown `analyzer` or unmapped `field` MUST return a SPEC-003 error listing registered analyzers / stating the field. Providing both `analyzer` and `field`, or neither, is a 400.
- **SET-063** The token stream returned by `_analyze` MUST be produced by the same registered tokenizer pipeline used at indexing time for that analyzer name — `_analyze` is a window into real analysis, not a simulation.

## 7. Settings-change semantics: live vs reindex

- **SET-070** Every knob is classified below; the classification is normative. "Live" changes take effect for the next query/write with no data rewrite. "Reindex" changes require rebuilding stored per-document data and MUST run as a SPEC-002 task (SET-023).

| Change | Class | Rationale |
|---|---|---|
| `searchableAttributes`, `displayedAttributes`, `rankingRules` | Live | Query construction / response shaping only. |
| `stopWords`, `synonyms`, `typoTolerance.*` | Live | Applied at query time (§2.1). |
| `pagination.maxTotalHits`, `searchCutoffMs`, `defaultPipeline` | Live | Execution budgets / write-path routing. |
| `filterableAttributes`, `sortableAttributes` — **adding** a field not yet fast | Reindex | Requires stamping `fast: true` into the Tantivy schema and rebuilding fast-field data. |
| `filterableAttributes`, `sortableAttributes` — **removing** a field | Live | Enforcement is a query-time check; fast-field data may linger until the next reindex/merge. |
| Mapping: new field / new multi-field | Live (additive) | SET-031; old documents simply lack the field. |
| Mapping: type or analyzer change on an existing field | Rejected → `_reindex` | SET-031. |
| `mapping.totalFieldsLimit` / `depthLimit` — raising | Live | Cap check only. |
| `mapping.totalFieldsLimit` / `depthLimit` — lowering below current usage | Rejected | MUST fail with the current count; existing fields are never dropped. |
| Infra: `number_of_shards` | Rejected after creation | Immutable per index (ES semantics); growth is rollover (SPEC-013). |
| Infra: `refresh_interval`, `number_of_replicas`, `storage.*` | Live | Metadata / scheduler inputs. |

- **SET-071** While a reindex-class settings task is running, reads MUST continue to be served with the old settings; the new settings become visible atomically when the task succeeds. A failed task leaves the old settings in force and reports the failure via the task (SPEC-002).

## 8. Index templates stamp settings objects

Templates (`crates/lexum-core/src/index/template.rs`) already stamp `TemplateSettings` + mappings at creation; they are the chassis for the new object.

- **SET-080** `TemplateSettings`/`PutTemplateRequest` MUST carry an optional full settings object (§2), **same wire shape** as the settings resource — no template-specific dialect.
- **SET-081** At index creation, matching templates merge by priority (higher wins per key, same algorithm as the existing mapping merge), then explicit request-level settings win over all templates, then defaults fill the gaps. `GET .../settings` on the created index MUST equal that merge result.
- **SET-082** ILM policy attachment via templates (SPEC-013 §2) uses this same stamping path.

## 9. Acceptance criteria

1. **Defaults round-trip**: fresh index → `GET .../settings` equals the §2 object exactly; `PATCH` of one knob leaves all others untouched; `DELETE` restores §2; each sub-setting route passes GET/PUT/DELETE.
2. **Restart survival** (SET-050/053): set settings + mapping, restart the server, `GET` both — byte-equivalent.
3. **Enforcement**: ordered `searchableAttributes` ranks a field-1 match above an identical field-2 match; `displayedAttributes` hides undeclared fields; filter/sort on an undeclared attribute returns the SET-012 error; `maxTotalHits` caps pagination; a pathological query respects `searchCutoffMs` with a degraded marker.
4. **Typo thresholds**: unit tests at 4/5/8/9-char boundaries, first-letter double-count, all three `disableOn*` lists (SET-011).
5. **Mappings**: additive `PUT _mapping` makes a new field searchable without reindex; type change rejected pointing at `_reindex`; the 1001st field fails with the cap error and no partial schema (SET-041); dynamic strings produce `text` + `.keyword` (SET-032).
6. **`_analyze`**: every registered analyzer returns tokens with positions/offsets; a mapping naming an analyzer actually tokenizes with it (index + search round-trip, SET-063).
7. **Templates**: index created via template returns the stamped settings; two overlapping templates merge by priority with request-level winning (SET-081).
