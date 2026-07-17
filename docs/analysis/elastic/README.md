# Elasticsearch Analysis for Lexum

> Research analysis informing Lexum's design and roadmap. Lexum is a Rust (Edition 2024, Tokio/axum) distributed full-text search engine built on Tantivy 0.25, explicitly inspired by Elasticsearch.
>
> Last updated: 2026-07. Facts verified against elastic.co documentation, Elastic blog posts, and third-party reporting; sources are cited inline in each file.
>
> Structure: one theme per numbered file; findings are numbered **F-001…F-055 globally** across the whole analysis, each with evidence, impact, and confidence.

## Section index

| § | File | Theme | Findings |
|---|---|---|---|
| §1 | [01-overview-licensing.md](01-overview-licensing.md) | History, licensing arc (Apache 2.0 → SSPL/ELv2 → +AGPLv3), the OpenSearch fork, the Elastic Stack ecosystem | F-001–F-005 |
| §2 | [02-architecture.md](02-architecture.md) | Node roles, cluster coordination, shards/replicas, segments, translog, refresh/flush/merge lifecycle, recovery internals | F-006–F-013 |
| §3 | [03-core-apis.md](03-core-apis.md) | Document CRUD, `_bulk`, `_search`, aggregations, pagination (scroll/`search_after`/PIT), mappings/analyzers, templates/data streams/ILM, snapshots, security, ingest pipelines | F-014–F-024 |
| §4 | [04-query-dsl.md](04-query-dsl.md) | Query vs filter context, `bool`, full-text vs term-level queries, BM25 relevance, ES SQL vs ES\|QL | F-025–F-030 |
| §5 | [05-distributed-model.md](05-distributed-model.md) | Write/read paths, in-sync replication, allocation/rebalancing, Jepsen/consistency honesty, Elastic's hard lessons | F-031–F-039 |
| §6 | [06-modern-features.md](06-modern-features.md) | `dense_vector`/kNN/HNSW/quantization, ELSER/`semantic_text`, runtime fields, TSDB/LogsDB, retrievers, agent/MCP direction | F-040–F-046 |
| §7 | [07-parity-matrix.md](07-parity-matrix.md) | Parity definition, the ~35-row ES-vs-Lexum feature matrix, ten anti-goals (ES complexity traps not to replicate) | F-047–F-050 |
| §8 | [08-execution-plan.md](08-execution-plan.md) | Tantivy advantages/limitations, the phased P0/P1/P2 roadmap, strategic summary, key sources | F-051–F-055 |

## Executive summary

Elasticsearch is a 15+-year-old Lucene-based engine (9.3.x on Lucene 10 as of 2026) whose 2021 relicense triggered the OpenSearch fork — making the **ES 7.10-era REST API a de-facto open standard** implemented by two independent engines and spoken by the entire client/shipper/dashboard ecosystem. That fact anchors the whole analysis: Lexum should define parity as *"drop-in for the 20% of the ES 7.10 API that 95% of clients use, plus 2026-grade vector/hybrid search, minus ES's legacy"* — not endpoint-count parity.

Architecturally, Tantivy already gives Lexum the Lucene-family segment model, BM25, and even an ES-shaped aggregation module — but **no translog**: WAL/durability/refresh semantics ("acked write ⇒ durable", "searchable within refresh_interval") are entirely Lexum's to build, and the distributed layer (seq-no replication, in-sync sets, quorum-free coordination) is both the moat and the largest risk. Elastic's scars are mapped in §5 as explicit warnings: ~9 years to get coordination right, a painful 6.x seq-no retrofit, oversharding, mapping explosion. Ten ES complexity traps are declared anti-goals (§7.3). The phased plan (§8) sequences a small P0 compatibility kernel (led by `_bulk` — the highest-value single endpoint — and a core `_search` DSL subset) that is achievable on the current single-node engine, then a P1 production platform (RBAC, ILM-lite, vectors with quantization + RRF), then P2 differentiation (`semantic_text` shape, ES|QL-style LQL extensions). Lexum's LQL and MCP-first bets are validated by Elastic's own trajectory (ES|QL, Agent Builder MCP server).

### Top findings by impact

| # | Impact | Theme | Finding | Evidence |
|---|---|---|---|---|
| F-004 | Critical | §1 Overview | The ES 7.10-era REST API is a de-facto open standard (ES + OpenSearch both implement it); compatibility buys Lexum the whole tooling ecosystem for free | [Wikipedia: OpenSearch](https://en.wikipedia.org/wiki/OpenSearch_(software)) |
| F-012 | Critical | §2 Architecture | Tantivy has no translog — WAL, durability, and refresh-interval semantics are entirely Lexum's responsibility; the single most important ES behavior to replicate faithfully | [Translog docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html) |
| F-015 | Critical | §3 Core APIs | `_bulk` (exact NDJSON framing, per-item errors, `?refresh`) is the highest-value single endpoint for compatibility — every shipper and client uses it | [Bulk API docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-bulk.html) |
| F-016 | Critical | §3 Core APIs | Aggregations — more than full-text search — made ES the analytics backbone; they are Lexum's highest-leverage missing feature | [Aggregations docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/search-aggregations.html) |
| F-017 | High | §3 Core APIs | Tantivy ships an aggregation module intentionally modeled on ES aggregation JSON — Lexum can expose it almost directly | [Tantivy aggregation docs](https://docs.rs/tantivy/latest/tantivy/aggregation/index.html) |
| F-039 | Critical | §5 Distributed | ES retrofitted seq-no replication painfully in 6.x — Lexum must design sequence numbers + checkpoints into its replication protocol from day one | [Resiliency status](https://www.elastic.co/guide/en/elasticsearch/resiliency/current/index.html) |
| F-036 | High | §5 Distributed | Cluster coordination correctness took Elastic ~9 years + formal methods — use a proven Raft crate, never expose a user-set quorum | [Elastic coordination blog](https://www.elastic.co/blog/a-new-era-for-cluster-coordination-in-elasticsearch) |
| F-030 | High | §4 Query DSL | ES\|QL (GA 8.14) proves pipe/SQL-style languages beat nested JSON for humans and LLMs — Lexum's LQL is convergent evolution; keep it flagship, treat JSON DSL as the compatibility layer | [ES\|QL GA](https://www.elastic.co/blog/whats-new-elastic-8-14-0) |
| F-041 | High | §6 Modern | Quantization is ES's default (int8 / BBQ ~32x with float rescoring) — Lexum vector support must ship quantization from day one | [BBQ docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/bbq) |
| F-047 | Critical | §7 Parity | Parity = "drop-in for the 20% of the ES 7.10 API that 95% of clients use, plus 2026-grade vector/hybrid search, minus ES's legacy" — not endpoint-count parity | Synthesis (F-004, F-015, F-041/F-042) |
| F-050 | High | §7 Parity | Ten ES complexity traps declared anti-goals; the most dangerous: unbounded dynamic mapping and in-engine scripting (pre-5.0 RCE history) | [Removal of types](https://www.elastic.co/guide/en/elasticsearch/reference/master/removal-of-types.html) |
| F-052 | High | §8 Plan | Multilingual analysis is Lexum's biggest raw-engine gap: Lucene's 30+ language analyzers/ICU/Kuromoji dwarf Tantivy's built-ins | [Tantivy README](https://github.com/quickwit-oss/tantivy) |
| F-053 | High | §8 Plan | Tantivy 0.25 has no built-in HNSW — Lexum must integrate its own ANN layer (VecLite / HNSW crates) and solve vector-index-in-segment-lifecycle itself | [Tantivy README](https://github.com/quickwit-oss/tantivy), [dense_vector docs](https://www.elastic.co/docs/reference/elasticsearch/mapping-reference/dense-vector) |
| F-054 | Critical | §8 Plan | All durability/replication machinery is Lexum's to build and test — Jepsen-style fault-injection testing should be a first-class deliverable | [Jepsen: Elasticsearch](https://aphyr.com/posts/317-jepsen-elasticsearch) |

### Reading guide

- Building the REST compatibility layer? Start with [§3](03-core-apis.md) and [§4](04-query-dsl.md), then the P0 list in [§8](08-execution-plan.md).
- Designing the distributed layer? [§2](02-architecture.md) and [§5](05-distributed-model.md) are the reference designs and the warnings.
- Prioritizing the roadmap? [§7](07-parity-matrix.md) (matrix + anti-goals) and [§8](08-execution-plan.md) (phases P0/P1/P2).
- Strategic/positioning questions? [§1](01-overview-licensing.md) (licensing/ecosystem) and [§6](06-modern-features.md) (vector/semantic/agent direction).

Full key-source list: see the end of [08-execution-plan.md](08-execution-plan.md).
