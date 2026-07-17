## 1. ANN layer evaluation (evidence before integration — F-053)
- [ ] 1.1 Define the `VectorIndex` trait in a new `crates/lexum-core/src/vector/` module: build-from-batch, incremental add, search(query, k, candidates, alive-docs bitset filter), serialize/deserialize, merge; this trait is the swappability contract
- [ ] 1.2 Benchmark candidates behind the trait on a fixed corpus (1M x 768d + glove-100): VecLite (`e:/HiveLLM/VecLite`, review SPEC-001 HNSW/quantization and SPEC-007 RRF for direct reuse), hannoy, usearch, raw `hnsw_rs` — measure recall@10, QPS, build time, peak memory, int8 support, merge/rebuild cost, license + maintenance health
- [ ] 1.3 Record the decision as an ADR (`rulebook decision create`) including the exit strategy (trait keeps the backend swappable) and what, if anything, is reused from VecLite vs vendored vs depended on
- [ ] 1.4 Gate check: chosen backend hits recall@10 ≥ 0.9 at ≥ 500 QPS single-threaded-build/multi-threaded-search on the benchmark corpus before any deeper integration proceeds

## 2. dense_vector field + segment lifecycle binding (the hard part)
- [ ] 2.1 Add the `dense_vector` field type to `crates/lexum-core/src/schema/` and the ES mapping converter (`dims`, `similarity: cosine|dot|l2`, `index`, `quantization: none|int8` default int8 per F-041); raw vectors stored in the doc store for re-index/re-quantize
- [ ] 2.2 Implement per-segment vector sidecar files keyed by segment id, written when a segment commits (hook into the phase1 task-queue commit path in `crates/lexum-core/src/index/manager.rs`)
- [ ] 2.3 Handle Tantivy segment merges: on merge, build the merged segment's vector index from the source segments (re-add surviving docs), delete orphaned sidecars; document the docid remapping approach
- [ ] 2.4 Respect deletes at query time via the segment alive-docs bitset (no rebuild on delete); compaction happens naturally at merge
- [ ] 2.5 Int8 quantization path: quantize at index time, keep float32 originals in doc store; measure memory ≥ 3x reduction and recall@10 drop ≤ 0.02 on the benchmark corpus
- [ ] 2.6 Include sidecars in snapshots (`crates/lexum-core/src/snapshot/`) with a restore round-trip test; verify the format is shard-copyable for phase9 peer recovery
- [ ] 2.7 Lifecycle correctness harness: randomized add/update/delete/merge churn, then assert kNN parity vs brute-force scan (never returns deleted docs, never misses live top-k docs)

## 3. kNN query
- [ ] 3.1 Add the `knn` query node to `crates/lexum-core/src/query/` and the search request in `crates/lexum-server/src/handlers/search.rs`: `{ field, queryVector, k, numCandidates, filter }`, ES-compatible shape
- [ ] 3.2 Filtered kNN: evaluate the Tantivy filter first, pass the resulting bitset into `VectorIndex::search`; below a selectivity threshold fall back to exact scan over matching docs (correctness > speed for tiny candidate sets)
- [ ] 3.3 Multi-segment merge: per-segment top-k merged to global top-k with normalized (0–1) vector similarity scores consistent with the phase8 score contract
- [ ] 3.4 Tests: exact vs ANN agreement gates, filter + kNN combinations, k > matching-docs edge cases, dims-mismatch and unindexed-field errors via the uniform error object

## 4. Hybrid fusion (over phase8 normalized scores)
- [ ] 4.1 Implement `hybrid: { semanticRatio }` — weighted blend `(1-r)*lexical + r*semantic` over normalized scores, executed as two branches merged through the phase8 merge engine in `crates/lexum-core/src/search/multi_search.rs`
- [ ] 4.2 Implement RRF (`rank_fusion: rrf` with `rank_constant`, ES-compatible; align semantics with VecLite SPEC-007 where they coincide) as the alternative fusion mode
- [ ] 4.3 Extend `showRankingScoreDetails` to break out lexical vs semantic contributions per hit
- [ ] 4.4 Fixture tests: `semanticRatio: 0.0` == pure lexical ordering, `1.0` == pure vector ordering; RRF matches hand-computed fixtures
- [ ] 4.5 Relevance eval: on a labeled dataset (BEIR subset or equivalent), hybrid nDCG@10 ≥ max(pure lexical, pure vector); record the eval harness in `benchmark/`

## 5. Embedders as index settings (R-13)
- [ ] 5.1 Add `embedders` to index settings: `{ name: { source: rest|userProvided|ollama, url, apiKeyEnv, documentTemplate, dimensions, requestMapping/responseMapping (rest only) } }`, settable via the phase3/phase4 settings surface
- [ ] 5.2 Implement the generic `rest` embedder (configurable JSON request/response paths — this alone covers OpenAI/Cohere/vertex/anything) and the Ollama embedder; `userProvided` accepts vectors in documents (`_vectors` field)
- [ ] 5.3 `documentTemplate` rendering (doc fields → embedding input string) with a length cap and a documented default template
- [ ] 5.4 Embedding cache keyed by hash of the rendered template output: unchanged docs re-indexed produce zero embedder calls (mock-verified test)
- [ ] 5.5 Run embedding inside the phase1 task queue: batched requests, provider rate-limit/backoff handling, per-task failure states surfaced through task status — never inline in the HTTP write path
- [ ] 5.6 Opt-in live integration test against a local Ollama (skipped when unavailable); mocked-provider tests always on

## 6. /similar endpoint + experimental gate
- [ ] 6.1 Add `crates/lexum-server/src/handlers/similar.rs`: `POST /api/v1/indices/{index}/similar` with `{ id, limit, offset, filter, embedder }` — kNN seeded by the stored vector of the seed doc, seed excluded from results (R-17)
- [ ] 6.2 Gate every vector surface (mapping, settings, knn, hybrid, /similar) behind the phase6 `vectorStore` experimental flag; disabled → uniform `feature_not_enabled` error (R-12)
- [ ] 6.3 Regression: with the flag off, the full existing test suite passes and no vector code executes on the hot path (no sidecar writes, no settings acceptance)
- [ ] 6.4 Update OpenAPI (`crates/lexum-server/src/openapi.rs`) for all new surfaces, marked experimental

## 7. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 7.1 Update or create documentation covering the implementation
- [ ] 7.2 Write tests covering the new behavior
- [ ] 7.3 Run tests and confirm they pass
