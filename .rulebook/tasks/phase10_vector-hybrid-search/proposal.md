# Proposal: phase10_vector-hybrid-search

## Why

Vector + hybrid search is table stakes for a 2026 search engine (ES P1
item 11) and Meilisearch's AI-native pivot is its growth engine (R-13,
F-004/F-009/F-017/F-018). Lexum has zero vector support today — no
vector field type in `crates/lexum-core/src/schema/`, no kNN query in
`crates/lexum-core/src/query/`, no ANN structure anywhere.

- **F-053** — Tantivy 0.25 has **no built-in vector/HNSW index** (Lucene
  has native HNSW). This is therefore an **integration project, not a
  wiring task**: Lexum must bring its own ANN layer, and the genuinely
  hard part is binding vector indexes to Tantivy's segment lifecycle —
  build at commit, merge on segment merge, respect deletes. Candidates
  to evaluate: **VecLite** (sibling project at `e:/HiveLLM/VecLite` — a
  Rust vector library with frozen storage/WAL specs, HNSW via
  `hnsw_rs`, quantization, and RRF hybrid fusion already specified in
  SPEC-007), **hannoy** (LMDB-based, MIT, proven at Meilisearch scale),
  **usearch**, and raw HNSW crates.
- **F-041** — ES learned float32 vectors dominate memory and made
  quantization the default; **int8 quantization ships from day one**,
  not as a follow-up.
- **R-13** — embedders as index settings: the generic **`rest` embedder
  covers every provider**, so `rest` + `userProvided` + Ollama first
  (no per-provider integrations needed for launch); `documentTemplate`
  to render docs into embedding inputs; embedding cache keyed by
  document hash so re-indexing unchanged docs never re-embeds (the #1
  cost/latency lever).
- **R-17** — a `/similar` (more-like-this) endpoint becomes nearly free
  once vectors exist.
- **R-12** — ship the whole surface behind the phase6
  **experimental-features gate**, exactly how Meilisearch de-risked
  vector search (v1.3 experimental → v1.13 GA).

Dependencies: **phase8 is a hard prerequisite** — hybrid fusion
(`semanticRatio`) is only meaningful over normalized 0–1 scores, and
vector scores merge through the same federated merge machinery. Phase6
provides the experimental gate. Phase3 (settings as a resource) provides
the settings surface embedders live in. Phase9 interacts: per-segment
vector sidecars must survive shard snapshot/recovery — kept compatible
by design here, exercised there.

## What Changes

1. **ANN layer decision ADR.** Benchmark VecLite, hannoy, usearch, and
   `hnsw_rs` on a fixed corpus (1M x 768d) for recall@10, QPS, build
   time, memory, int8 support, incremental build/merge fit with a
   segment lifecycle, license and maintenance health. Record the choice
   and the exit strategy (trait-abstracted `VectorIndex` so the backend
   is swappable).
2. **`dense_vector` field type** in schema/mapping: `dims`, `similarity`
   (cosine/dot/l2), `index: true|false`, `quantization: none|int8`
   (default int8, F-041). ES-compatible mapping shape.
3. **Segment lifecycle binding (the hard part, F-053).** Per-segment
   vector index sidecar files keyed by segment id: built when a segment
   is committed, rebuilt/merged when Tantivy merges segments, filtered
   by the segment's alive-docs bitset at query time (deletes respected
   without index rebuild), included in snapshots and phase9 shard
   recovery.
4. **kNN query**: `knn: { field, queryVector | q (embedded), k,
   numCandidates, filter }` on the search request — filter applied as
   Tantivy pre-filter with ANN over the surviving candidates, falling
   back to exact scan under a selectivity threshold.
5. **Hybrid fusion over phase8 scores**: `hybrid: { semanticRatio }`
   (weighted blend of normalized lexical and vector scores) and **RRF**
   (`rank_fusion: rrf`, ES-compatible) — both implemented over the
   phase8 merge engine; `showRankingScoreDetails` breaks out
   lexical vs semantic contributions.
6. **Embedders as index settings (R-13)**: `embedders: { name: { source:
   rest|userProvided|ollama, url, apiKeyEnv, documentTemplate,
   dimensions } }`; `rest` speaks a configurable JSON request/response
   mapping; embedding cache keyed by hash of the rendered
   documentTemplate output; embedding happens in the phase1 task queue
   (async, batched, rate-limit aware) — never inline in the HTTP write
   path.
7. **`/similar` endpoint (R-17)**: `GET/POST /indexes/{index}/similar`
   with `id`, `limit`, `filter` — kNN seeded by the stored vector of the
   given document.
8. **Experimental gate (R-12)**: everything above behind the phase6
   `vectorStore` experimental flag; disabled → vector mapping/query/
   settings return a clear `feature_not_enabled` error.

## Impact

- Affected specs: `.rulebook/tasks/phase10_vector-hybrid-search/specs/`
  (vector field + kNN API, segment-lifecycle binding, embedder settings,
  hybrid fusion semantics)
- Affected code:
  - New `crates/lexum-core/src/vector/` (`VectorIndex` trait, chosen
    backend, quantization, per-segment sidecar store, embedder clients,
    embedding cache)
  - `crates/lexum-core/src/schema/` (dense_vector field type, mapping
    converter), `crates/lexum-core/src/query/` (knn query node),
    `crates/lexum-core/src/search/executor.rs` + `multi_search.rs`
    (hybrid fusion over the phase8 merge engine),
    `crates/lexum-core/src/index/manager.rs` + settings (embedders,
    segment hooks), `crates/lexum-core/src/snapshot/` (sidecar files)
  - `crates/lexum-server/src/handlers/search.rs` (knn/hybrid params),
    new `crates/lexum-server/src/handlers/similar.rs`,
    `crates/lexum-server/src/router.rs`, `openapi.rs`
- Breaking change: NO (new field type, new opt-in query surface, all
  behind an experimental flag)
- User benefit: semantic and hybrid search with bring-your-own or
  server-managed embeddings — the feature that decides whether Lexum is
  a 2026 engine or a 2016 one — without ES's JVM or Meilisearch's
  single-writer storage.

## Success criteria

- ADR merged with benchmark evidence for the ANN choice; `VectorIndex`
  is trait-abstracted (a second backend compiles against it in a test).
- Recall/perf gate on a public dataset (e.g. glove-100 or sift-1m):
  recall@10 ≥ 0.9 at ≥ 500 QPS single-node for the chosen configuration;
  int8 cuts vector memory ≥ 3x with recall@10 drop ≤ 0.02 vs float32.
- Lifecycle correctness: after heavy update/delete/merge churn
  (randomized test), kNN never returns a deleted doc and never misses a
  live doc that exact search finds in the top-k (parity harness vs
  brute-force scan).
- Hybrid: `semanticRatio: 0.0` reproduces pure lexical ordering,
  `1.0` pure vector ordering (fixture tests); RRF output matches the
  published formula on hand-computed fixtures; on a labeled eval set,
  hybrid nDCG@10 ≥ max(pure lexical, pure vector).
- Embedding cache: re-indexing an unchanged document performs zero
  embedder HTTP calls (mock-verified); `rest` embedder works against a
  mocked provider and a live Ollama in an opt-in integration test.
- `/similar` returns neighbors excluding the seed document itself.
- With the `vectorStore` flag off, all vector surfaces return the
  uniform `feature_not_enabled` error and nothing else regresses (full
  suite green).
- Snapshots taken with vectors restore with working kNN (round-trip
  test).
