## 1. Normalized ranking score (R-06 — prerequisite for everything below)
- [ ] 1.1 Write the normalization spec first (in `specs/`): define the 0–1 mapping from Tantivy BM25 (normalize against a computable per-query bound, NOT the top hit — record why top-hit normalization breaks cross-shard/cross-node merging), determinism guarantees, and the score-details schema
- [ ] 1.2 Implement normalized score computation in `crates/lexum-core/src/search/executor.rs` and surface it on `SearchHit` (`crates/lexum-core/src/search/result.rs`) behind `showRankingScore`
- [ ] 1.3 Implement `showRankingScoreDetails` (per-field BM25 contribution + weight breakdown; structured for phase2 criteria to plug into later)
- [ ] 1.4 Implement `rankingScoreThreshold` (post-scoring filter applied before pagination)
- [ ] 1.5 Property tests: score always in [0,1]; monotonic with raw `_score` within a query; same doc+query+settings → same score on a rebuilt index (cross-node stability proxy)

## 2. POST /multi-search — non-federated (one round trip, per-query errors)
- [ ] 2.1 Add `crates/lexum-server/src/handlers/multi_search.rs` with the `{ queries: [ { indexUid, ...full search request } ] }` body and register `POST /multi-search` in `crates/lexum-server/src/router.rs` + OpenAPI
- [ ] 2.2 Execute queries concurrently (bounded parallelism) against `MultiSearchExecutor`/`SearchExecutor`; preserve request order in the response
- [ ] 2.3 Per-query error objects (A-03): a failing query yields `{ indexUid, error: { message, code, type, link } }` in its slot, HTTP 200 overall; no all-or-nothing failure path exists
- [ ] 2.4 Enforce phase7 privileges per sub-query via `AuthContext` (unauthorized index → per-query 403-style error object, others unaffected)
- [ ] 2.5 Integration test: 5 queries, 1 malformed + 1 unauthorized → 3 results + 2 error objects

## 3. Federated merging (single ranked list)
- [ ] 3.1 Implement the merge engine in `crates/lexum-core/src/search/multi_search.rs`: k-way merge by `weight * normalizedScore`, per-query `federationOptions.weight` (default 1.0), federation-level `limit`/`offset`
- [ ] 3.2 Dedup identical (indexUid, docId) hits keeping the highest weighted score; attach `_federation: { indexUid, queriesPosition, weightedRankingScore }` to every hit
- [ ] 3.3 Reject incompatible per-query options in federated mode (per-query `offset`/`limit`/`sort`) with a clear error, matching the spec written in 1.1
- [ ] 3.4 Deterministic fixture tests: 3-index federation ordering; weight change reorders; tie-break rule is stable and documented
- [ ] 3.5 Wire `rankingScoreThreshold` and `showRankingScore(Details)` through federated responses

## 4. Remote federation (R-07 — the scatter-gather seed for phase9)
- [ ] 4.1 Add the `network` config resource: `GET/PATCH /network` (`self`, `remotes: { name: { url, searchApiKey } }`), persisted with server config; handler `crates/lexum-server/src/handlers/network.rs`
- [ ] 4.2 Implement the remote proxy client in `crates/lexum-server/src/services/` (HTTP, connection-pooled, per-remote timeout + budget), sending sub-queries to the remote's `/multi-search` with its `searchApiKey`
- [ ] 4.3 Merge remote hits by their returned normalized scores through the SAME merge engine as 3.1 (no separate remote merge path — this is the phase9 reuse guarantee)
- [ ] 4.4 Remote failure handling: timeout/unreachable/auth-failure become per-query error objects with partial results (A-03); add `remoteErrors` metadata to the federated response
- [ ] 4.5 Two-process integration test: spawn two Lexum servers, configure each as the other's remote, federate a query across both and assert merged ordering + partial-result behavior when one is killed mid-test

## 5. ES-compat alias + hardening
- [ ] 5.1 Re-route `POST /api/v1/_msearch` (`crates/lexum-server/src/handlers/query_ops.rs`) onto the same execution core as `/multi-search` (one engine, two wire formats); keep the existing JSON contract green via regression tests
- [ ] 5.2 Accept NDJSON `_msearch` bodies (header/body line pairs) for ES shipper compatibility
- [ ] 5.3 Cap `queries` count per request (configurable, default e.g. 20 non-federated / 30 federated) with a clear error; apply `query_complexity` middleware budget across sub-queries
- [ ] 5.4 Benchmark in `benchmark/`: 10-query multi-search ≤ 1.3x slowest single query wall time; federation merge overhead measured and recorded

## 6. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 6.1 Update or create documentation covering the implementation
- [ ] 6.2 Write tests covering the new behavior
- [ ] 6.3 Run tests and confirm they pass
