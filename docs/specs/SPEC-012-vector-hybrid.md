# SPEC-012 — Vector & Hybrid Search

| | |
|---|---|
| **Status** | Draft — entire surface experimental-gated until GA (VEC-001); the ANN backend choice is an open ADR (VEC-030) |
| **Phase / tasks** | Phase 10 · tasks 1–6 (`.rulebook/tasks/phase10_vector-hybrid-search/tasks.md`) |
| **Planning source** | [phase10 proposal](../../.rulebook/tasks/phase10_vector-hybrid-search/proposal.md); Elastic F-053 (Tantivy has no HNSW — integration project, segment lifecycle is the hard part), F-041 (quantization default from day one), F-040/F-042 (kNN + hybrid RRF); Meilisearch R-13 / F-004/F-009/F-017/F-018 (embedders as settings, cache, `semanticRatio`), R-17 (`/similar`), R-12 (experimental gate); prior art [VecLite SPEC-007 — Hybrid Search](../../../VecLite/docs/specs/SPEC-007-hybrid-search.md) (RRF semantics) |

Requirement IDs `VEC-xxx`. RFC 2119 keywords are normative. Vector scores obey the SPEC-010 normalized-score contract (FED-010..012) and hybrid fusion executes through the SPEC-010 merge engine; embedding jobs run inside the SPEC-002 task queue; errors follow the SPEC-003 error contract; sidecar files ride SPEC-011 snapshots and peer recovery unchanged.

## 1. Experimental gate

- **VEC-001** Every surface in this spec — the `dense_vector` mapping type, `embedders` settings, `knn`, `hybrid`, `/similar` — is gated behind the phase6 experimental flag `vectorStore` (`GET/PATCH /experimental-features`). With the flag off, each surface returns the uniform SPEC-003 error `feature_not_enabled` (400) and **no vector code executes on any hot path**: no sidecar writes, no settings acceptance, no cache population.
- **VEC-002** Enabling the flag is per-process and dynamic; disabling it hides the surfaces but MUST NOT delete existing sidecars or cached embeddings (re-enabling resumes cleanly).

## 2. `dense_vector` field type

- **VEC-010** Mapping shape (ES-compatible):

```json
"embedding": {
  "type": "dense_vector",
  "dims": 768,
  "similarity": "cosine",
  "index": true,
  "quantization": "int8"
}
```

  `dims`: 1..=4096, required. `similarity`: `cosine` | `dot_product` | `l2_norm` (default `cosine`). `index`: default `true` (false = store/rescore only). `quantization`: `int8` (default, F-041) | `none`.
- **VEC-011** Ingest validation: vector length MUST equal `dims` (`invalid_vector_dimensions`), values finite (`invalid_vector_values`); `dot_product` additionally requires unit-normalized vectors (rejected otherwise), which is what makes its score bound computable (VEC-051).
- **VEC-012** The raw float32 vector is always stored in the document store, regardless of `index`/`quantization` — it is the source of truth for re-indexing, re-quantization, merge rebuilds, final-stage rescoring (VEC-072), and `/similar` seeding.
- **VEC-013** `dims`, `similarity`, and `quantization` are immutable per field once documents exist; changing them is a reindex (same rule as analyzer changes).

## 3. `VectorIndex` trait & ANN backend ADR

- **VEC-030** The ANN backend is selected by ADR (`rulebook decision create`) after benchmarking the four candidates behind the trait on a fixed corpus (1M × 768d + glove-100): **VecLite** (sibling project; SPEC-001 HNSW/quantization and SPEC-007 RRF reviewed for direct reuse), **hannoy** (LMDB-based, MIT, proven at Meilisearch scale), **usearch**, **hnsw_rs**. Measured: recall@10, QPS, build time, peak memory, int8 support, merge/rebuild cost, license + maintenance health. Gate before deeper integration: recall@10 ≥ 0.9 at ≥ 500 QPS on the benchmark corpus.
- **VEC-031** The backend hides behind a swappability contract in `crates/lexum-core/src/vector/`:

```rust
pub trait VectorIndex: Send + Sync {
    fn build(vectors: impl Iterator<Item = (DocId, &[f32])>, cfg: &VectorFieldConfig) -> Result<Self>;
    fn search(&self, query: &[f32], k: usize, num_candidates: usize,
              alive: Option<&AliveBitSet>) -> Vec<(DocId, f32)>;   // raw similarity
    fn write(&self, w: impl Write) -> Result<()>;
    fn open(bytes: OwnedBytes, cfg: &VectorFieldConfig) -> Result<Self>; // mmap-friendly
    fn merge(parts: &[(&Self, &DocIdMapping)], cfg: &VectorFieldConfig) -> Result<Self>;
    fn memory_usage(&self) -> usize;
}
```

  The exit strategy is the trait: a second backend MUST compile and pass the conformance tests against it (verified in CI with a brute-force reference implementation).
- **VEC-032** `search` MUST honor the `alive` bitset as a hard filter (never return a dead doc) and MUST treat `num_candidates` as the ANN beam width, returning ≤ k alive results.

## 4. Per-segment sidecar lifecycle (the hard part — F-053)

Tantivy 0.25 has no vector index; Lexum binds ANN structures to Tantivy's segment lifecycle as **per-segment sidecar files**. This binding — build at commit, rebuild at merge, filter at delete — is the acknowledged core difficulty of this spec.

- **VEC-040** One sidecar per (segment, indexed vector field): `<segment_id>.<field>.vidx` in the index directory, containing a 16-byte header (magic `LXVI`, format version, similarity, quantization) + the `VectorIndex::write` payload + footer crc32.
- **VEC-041** **Commit**: sidecars are built synchronously inside the SPEC-002 task-queue commit path — when a task application commits new Tantivy segments, the commit is not acknowledged as `succeeded` until every new segment's sidecars are built and fsynced. A segment without its sidecars MUST never become searchable with kNN (a missing sidecar at query time is `vector_index_unavailable`, never a silent empty result).
- **VEC-042** **Merge**: when Tantivy merges segments, the merged segment's sidecar is built by `VectorIndex::merge` over the source sidecars with the merge's docid remapping (deleted docs dropped — merge is where dead vectors are physically compacted). Source sidecars are deleted together with their segments; sidecar GC at index open removes orphans whose segment id no longer exists in the Tantivy meta.
- **VEC-043** **Delete**: deletes never touch sidecars. At query time the segment's alive-docs bitset is passed to `VectorIndex::search` (VEC-032), so kNN reflects deletes immediately with no rebuild.
- **VEC-044** Sidecars are included in snapshots (`crates/lexum-core/src/snapshot/`) and are plain shard files for SPEC-011 peer recovery (DST-012/070) — a restored or peer-recovered shard has working kNN with no rebuild step (round-trip tested).
- **VEC-045** Lifecycle correctness harness: under randomized add/update/delete/merge churn, kNN MUST never return a deleted document and never miss a live document that a brute-force scan puts in the top-k (parity gate, run in CI).

## 5. kNN query

- **VEC-050** Search-request shape (ES-compatible):

```json
"knn": {
  "field": "embedding",
  "queryVector": [0.1, ...],        // XOR: "q": "text to embed", "embedder": "default"
  "k": 10,
  "numCandidates": 100,
  "filter": "category = 'shoes'"
}
```

  Exactly one of `queryVector` / `q` MUST be present (`q` renders through the named embedder, VEC-080). Defaults: `k` 10; `numCandidates` `max(100, 3·k)`, cap 10 000. Dims mismatch, unindexed field, unknown embedder → SPEC-003 errors (`invalid_vector_dimensions`, `vector_field_not_indexed`, `unknown_embedder`).
- **VEC-051** Raw similarity maps to the SPEC-010 normalized score by a fixed, result-set-independent function per metric (FED-011 compliant — never normalized against the top hit):

| `similarity` | normalized score |
|---|---|
| `cosine` | `(1 + cos) / 2` |
| `dot_product` | `(1 + dot) / 2` (unit vectors enforced, VEC-011) |
| `l2_norm` | `1 / (1 + d)` |

- **VEC-052** Filtered kNN: the `filter` is evaluated first as a Tantivy query producing a per-segment bitset that is intersected with alive docs and passed to `VectorIndex::search`. When the filtered candidate count for a segment is ≤ `max(10·k, 1000)`, execution MUST fall back to an exact scan over the matching docs (correctness over speed for tiny candidate sets — ANN recall degrades badly under highly selective filters).
- **VEC-053** Multi-segment execution: per-segment top-k results merge to a global top-k by normalized score with the FED-032 deterministic tie-break. A pure `knn` request is a valid single-branch search (usable standalone, without `hybrid`).

## 6. Hybrid fusion (over SPEC-010 normalized scores)

- **VEC-060** `hybrid: { "semanticRatio": 0.5, "embedder": "default" }` on a search request executes two branches — lexical (the request's `q`/query) and semantic (kNN with the embedded `q`) — and merges them through the SPEC-010 `FederatedMergeEngine` as a two-query federation with weights `(1 − r)` and `r`. `semanticRatio ∈ [0,1]`, default 0.5. `0.0` MUST reproduce pure lexical ordering and `1.0` pure vector ordering exactly (fixture-tested). Documents found by both branches dedup per FED-033 semantics with score `(1−r)·lex + r·sem` (sum, not max, for hybrid — both contributions count).
- **VEC-061** Rank-based alternative: `"rankFusion": "rrf"` with `"rankConstant"` (default 60) replaces the weighted blend with Reciprocal Rank Fusion:

  `score(d) = Σ_branches w_b · 1 / (rankConstant + rank_b(d))`

  1-based ranks within each branch; a document absent from a branch contributes 0 for it; `w_b` from `semanticRatio` as above. Semantics align with VecLite SPEC-007 HYB-020/021 (pure rank-based RRF, deterministic) and the ES-compatible `rank_constant` parameter. Ties break by semantic-branch rank, then docId bytewise — fully deterministic.
- **VEC-062** RRF scores are themselves ≤ `Σ w_b / (rankConstant + 1)` and are renormalized by that bound into [0,1] so hybrid hits remain mergeable by SPEC-010 federation and SPEC-011 distributed search (FED-016).
- **VEC-063** `showRankingScoreDetails` (FED-014) gains `vector` and `fusion` entries breaking out the lexical vs semantic contributions and the fusion arithmetic per hit.

## 7. Quantization — int8 from day one (F-041)

- **VEC-070** With `quantization: "int8"` (the default) sidecars store scalar-quantized int8 vectors with per-segment quantization parameters computed at build/merge time; float32 originals stay in the doc store (VEC-012). Memory/disk for the indexed structure MUST shrink ≥ 3× vs float32 with recall@10 drop ≤ 0.02 on the benchmark corpus (gate).
- **VEC-071** Re-quantization happens only at merge/rebuild from the stored float32 originals — quantization error never compounds through repeated merges.
- **VEC-072** Final-stage rescoring: the global top-k candidates from quantized search are rescored against their float32 originals from the doc store before scores are returned (default on; opt-out `"rescore": false` per knn clause).

## 8. Embedders as index settings (R-13)

- **VEC-080** Index settings gain:

```json
"embedders": {
  "default": {
    "source": "rest",                       // "rest" | "userProvided" | "ollama"
    "url": "https://api.openai.com/v1/embeddings",
    "apiKeyEnv": "OPENAI_API_KEY",          // env var NAME — secrets never stored in settings
    "dimensions": 768,
    "documentTemplate": "{{doc.title}}: {{doc.description}}",
    "request":  { "input": ["{{text}}"], "model": "text-embedding-3-small" },
    "response": { "path": ["data", "*", "embedding"] }
  }
}
```

  Launch sources, deliberately minimal: `rest` (configurable JSON request/response mapping — this alone covers OpenAI/Cohere/Vertex/anything), `userProvided` (vectors supplied in documents under `_vectors.<embedder>`; no server-side embedding), `ollama` (thin preset over the REST shape). Settable via the standard settings surface; `apiKeyEnv` is an environment-variable name and raw secrets MUST never appear in stored settings or GET responses.
- **VEC-081** `documentTemplate` renders document fields into the embedding input string (missing fields render empty), truncated at `documentTemplateMaxBytes` (default 400 bytes of rendered text). Default template: the concatenation of the index's searchable text fields in schema order.
- **VEC-082** **Embedding cache**: keyed by `SHA-256(embedder_name ‖ embedder_config_revision ‖ rendered_template_output)`, persisted under the index data directory. Re-indexing a document whose rendered template output is unchanged MUST perform zero embedder HTTP calls (mock-verified — the #1 cost/latency lever). Changing the embedder config bumps the revision and naturally invalidates.
- **VEC-083** Embedding executes **inside the SPEC-002 task queue only** — batched requests, provider rate-limit/backoff handling (429/5xx exponential backoff with jitter), per-task failure states surfaced through task status. Embedding never happens inline in the HTTP write path; a search-time `q` embedding (VEC-050) is the single synchronous exception and is bounded by the request budget.

## 9. `/similar` endpoint (R-17)

- **VEC-090** `POST /api/v1/indices/{index}/similar` with `{ "id": "doc-42", "embedder": "default", "limit": 10, "offset": 0, "filter": "...", "showRankingScore": true }`: kNN seeded by the stored vector of document `id` for the named embedder/field. The seed document itself MUST be excluded from results. Unknown `id` → 404 `document_not_found`; document without a stored vector → `document_missing_vector`.
- **VEC-091** `/similar` counts as a search endpoint for SPEC-009: it requires the `search` action, accepts tenant tokens, and AND-combines forced filters (SEC-055/057).

## 10. Acceptance criteria

1. **ADR + gates**: backend ADR merged with benchmark evidence; recall@10 ≥ 0.9 at ≥ 500 QPS (VEC-030); int8 ≥ 3× memory reduction with ≤ 0.02 recall drop (VEC-070); a second `VectorIndex` backend compiles and passes conformance (VEC-031).
2. **Lifecycle**: VEC-045 churn harness green (no deleted doc returned, no live top-k doc missed vs brute force); snapshot and peer-recovery round trips restore working kNN (VEC-044).
3. **Hybrid semantics**: `semanticRatio` 0.0/1.0 reproduce pure orderings; RRF matches hand-computed fixtures at `rankConstant` 60; on a labeled eval set hybrid nDCG@10 ≥ max(pure lexical, pure vector) (recorded in `benchmark/`).
4. **Embedders**: unchanged doc re-index performs zero embedder calls (VEC-082); `rest` embedder passes against a mocked provider; opt-in live Ollama integration test; embedding failures surface as task failure states, never HTTP-write failures (VEC-083).
5. **Gate discipline**: with `vectorStore` off, every vector surface returns `feature_not_enabled`, the full existing suite passes, and no vector code executes on the hot path (VEC-001); `/similar` excludes its seed and honors tenant forced filters (VEC-090/091).
