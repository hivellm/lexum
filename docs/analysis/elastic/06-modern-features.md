# 6. Modern Features

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-040…).

## 6.1 Vector search: `dense_vector` + kNN

- **`dense_vector`** field type: float (also byte/bit) vectors up to 4096 dims; indexed by default into **HNSW** graphs; `element_type`, `similarity` (cosine, dot_product, l2_norm, max_inner_product) ([dense_vector docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/dense-vector)).
- **Quantization** is now the default: **`int8_hnsw`** default for < 384 dims, **`bbq_hnsw`** (Better Binary Quantization, ~32x compression, GA and default for ≥ 384 dims in the 9.x line) — original float vectors are kept for optional **rescoring** of top-k ([BBQ docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/bbq), [9.0 announcement](https://www.elastic.co/blog/whats-new-elastic-search-9-0-0), [kNN docs](https://www.elastic.co/docs/solutions/search/vector/knn)).
- **Search**: top-level `knn` section or `knn` query clause; approximate kNN (HNSW, per-segment graphs) with `num_candidates`, filtered kNN (filter applied during graph traversal), plus exact brute-force via `script_score` when needed.

### F-040 — `dense_vector` fields (up to 4096 dims) are HNSW-indexed by default, with per-segment graphs, filtered kNN during graph traversal, and a brute-force `script_score` fallback
- **Evidence:** [dense_vector docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/dense-vector), [kNN docs](https://www.elastic.co/docs/solutions/search/vector/knn)
- **Impact:** Defines the API shape Lexum's vector support should target: a vector field type in the mapping, a top-level `knn` section / `knn` query clause with `num_candidates`, and filter-aware traversal (not post-filtering).
- **Confidence:** High

### F-041 — Quantization is the ES default, not an option: `int8_hnsw` (< 384 dims) and BBQ (~32x compression, GA and default for ≥ 384 dims in 9.x), with float originals kept for top-k rescoring
- **Evidence:** [BBQ docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/bbq), [9.0 announcement](https://www.elastic.co/blog/whats-new-elastic-search-9-0-0)
- **Impact:** Lexum should ship quantization (at least int8) from day one of vector support rather than treating it as an optimization — memory cost of unquantized HNSW is the practical adoption blocker, and the quantize-then-rescore pattern is now the industry default.
- **Confidence:** High

### F-042 — Hybrid search (BM25 + kNN fused via RRF retriever or linear boosts) is the standard modern relevance pattern
- **Evidence:** [kNN docs](https://www.elastic.co/docs/solutions/search/vector/knn); retriever trees incl. RRF since 8.14 ([§4.4](04-query-dsl.md))
- **Impact:** Vector search alone is not the target — the target is lexical+vector fusion. Lexum's P1 vector milestone should include RRF, since that is what "vector support" means to users in 2026.
- **Confidence:** High

## 6.2 Semantic search: ELSER and `semantic_text`

- **ELSER** (Elastic Learned Sparse EncodeR): Elastic-trained **sparse** retrieval model (learned term expansion, ~30k-dim sparse vectors stored in `sparse_vector` fields, queried with the `sparse_vector` query). Good zero-shot English relevance without tuning ([ELSER docs](https://www.elastic.co/docs/explore-analyze/machine-learning/nlp/ml-nlp-elser), [sparse_vector query](https://www.elastic.co/docs/reference/query-languages/query-dsl/query-dsl-sparse-vector-query)).
- **Inference endpoints** (`_inference` API): unified abstraction over embedding/rerank/completion models — Elastic-hosted (ELSER, E5), uploaded via Eland, or external providers (OpenAI, Cohere, etc.).
- **`semantic_text`** field type: the "easy mode" — declare the field, point it at an inference endpoint; ES handles chunking, embedding at index and query time, storage format; query with a plain `semantic` (or `match`) query ([semantic_text docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/semantic-text), [Search Labs](https://www.elastic.co/search-labs/blog/semantic-search-simplified-semantic-text)).

### F-043 — ELSER shows learned sparse retrieval (term expansion into `sparse_vector` fields) delivers good zero-shot English relevance without tuning
- **Evidence:** [ELSER docs](https://www.elastic.co/docs/explore-analyze/machine-learning/nlp/ml-nlp-elser), [sparse_vector query](https://www.elastic.co/docs/reference/query-languages/query-dsl/query-dsl-sparse-vector-query)
- **Impact:** Sparse learned retrieval is a storage-side feature (a sparse-vector field type + query), not necessarily an engine-hosted ML feature — the model can run externally. Relevant if Lexum adds a `sparse_vector`-style field later.
- **Confidence:** High

### F-044 — `semantic_text` + inference endpoints is the design to copy in *shape* only: engine orchestrates chunking/embedding/storage, model runs behind a pluggable endpoint
- **Evidence:** [semantic_text docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/semantic-text), [Search Labs](https://www.elastic.co/search-labs/blog/semantic-search-simplified-semantic-text); `_inference` API abstracts Elastic-hosted and external providers (OpenAI, Cohere, etc.)
- **Impact:** Lexum should not embed ML in the engine. A `semantic_text`-like field type + pluggable external inference endpoints (HTTP) gets ~90% of the value at ~5% of the complexity, and fits Lexum's MCP/agent orientation. (Sibling project VecLite may cover the vector store itself.)
- **Confidence:** High

## 6.3 Runtime fields

Introduced in **7.11** (beta): **schema-on-read** fields defined by a Painless script in the mapping or per-query (`runtime_mappings`), evaluated at query time; queryable/aggregatable like real fields, at CPU cost; can later be "promoted" to indexed fields ([Elastic blog](https://www.elastic.co/blog/introducing-elasticsearch-runtime-fields), [7.11 release blog](https://www.elastic.co/blog/whats-new-elasticsearch-7-11-0-schema-on-read-is-here)). Used heavily in observability for parsing-at-query-time.

### F-045 — Runtime fields require a sandboxed scripting language (Painless); for Lexum they are low-priority and hard — computed fields in LQL expressions are a cheap 80% substitute
- **Evidence:** [Introducing runtime fields](https://www.elastic.co/blog/introducing-elasticsearch-runtime-fields), [7.11 release blog](https://www.elastic.co/blog/whats-new-elasticsearch-7-11-0-schema-on-read-is-here)
- **Impact:** Skip Painless-style schema-on-read; extend LQL with computed expressions instead. This also aligns with the anti-goal of never embedding a general-purpose scripting VM in the engine ([§7](07-parity-matrix.md)).
- **Confidence:** Medium

## 6.4 Other modern bits (context, lower priority)

- **TSDB mode** (time_series indices, 8.7+) and **LogsDB mode** (8.17+): specialized storage modes cutting time-series/log storage dramatically.
- **Data streams + downsampling**, **frozen tier** on object storage.
- **Retrievers** framework (standard/knn/rrf/text_similarity_reranker) as composable ranking pipelines.
- **`_inference` + Playground + Agent Builder**: Elastic is racing toward agent/RAG workflows, also shipping its own **MCP server** work in 9.x-era Agent Builder.

### F-046 — Elastic is racing toward agent/RAG workflows (Playground, Agent Builder, its own MCP server in the 9.x era) — validating Lexum's MCP-first plan
- **Evidence:** `_inference` + Playground + Agent Builder direction; Elastic shipping MCP server work in 9.x-era Agent Builder; TSDB (8.7+) and LogsDB (8.17+) storage modes as the observability-side context
- **Impact:** Lexum's MCP + UMICP orientation is not speculative — the incumbent is converging on the same interface. Lexum can be MCP-native first rather than retrofitting, a genuine differentiation window.
- **Confidence:** Medium

---

Next: [7. Feature Parity Matrix and Anti-Goals](07-parity-matrix.md)
