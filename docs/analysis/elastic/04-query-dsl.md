# 4. Query DSL Deep Dive

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-025…).

Reference: [Query DSL docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html).

## 4.1 Query context vs filter context

The single most important DSL concept:

- **Query context** — "how well does this match?" → contributes to `_score`.
- **Filter context** — "does this match, yes/no?" → no scoring, **cacheable** (bitset caching of frequently used filters per segment), faster.

Filters live in `bool.filter` / `bool.must_not` (both non-scoring). Since ES 2.0 there is no separate filter DSL — context is positional.

### F-025 — Query vs filter context is both a semantic and a performance contract: filter context is non-scoring and bitset-cached per segment
- **Evidence:** [Query DSL docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html); since ES 2.0 there is no separate filter DSL — context is positional (`bool.filter` / `bool.must_not` are non-scoring)
- **Impact:** Any Lexum parity effort must preserve this: identical queries in different positions must score differently *and* Lexum should implement an equivalent of filter-bitset caching, since users' performance expectations are built on it.
- **Confidence:** High

## 4.2 `bool` — the workhorse

```json
{ "bool": {
    "must":     [ { "match": { "title": "rust search" } } ],
    "should":   [ { "match": { "tags": "engine" } } ],
    "must_not": [ { "term": { "status": "draft" } } ],
    "filter":   [ { "range": { "created": { "gte": "2025-01-01" } } } ],
    "minimum_should_match": 1
} }
```

`must`/`should` score (should adds optional boost; becomes required if no must/filter present unless `minimum_should_match` says otherwise), `filter`/`must_not` don't. Other compound queries: `dis_max`, `boosting`, `constant_score`, `function_score`/`script_score` (custom ranking), `rank_feature`.

### F-026 — `bool` is the compound-query workhorse, with subtle `should` semantics (`should` becomes required when no `must`/`filter` is present, modulated by `minimum_should_match`)
- **Evidence:** [Query DSL docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html)
- **Impact:** Nearly every real-world ES query is a `bool`. Lexum's ES-compatible `_search` must reproduce these clause semantics exactly — including the `should`-promotion quirk — or existing queries will silently return different results.
- **Confidence:** High

## 4.3 Full-text vs term-level queries

- **Full-text** (analyzed at query time): `match`, `match_phrase`, `match_phrase_prefix`, `multi_match` (types: `best_fields`, `most_fields`, `cross_fields`, `phrase`), `query_string` (full Lucene syntax, error-prone on user input), `simple_query_string` (safe user-facing variant), `intervals`, `combined_fields`.
- **Term-level** (exact, not analyzed): `term`, `terms`, `terms_set`, `range`, `exists`, `prefix`, `wildcard`, `regexp`, `fuzzy`, `ids`.
- Others: `nested` (query nested docs with inner_hits), `geo_*`, `percolate` (reverse search: store queries, match documents), `span_*`/`intervals` (proximity), `more_like_this`.

### F-027 — The classic ES footgun: a `term` query against a `text` field silently fails to match — the origin of the `text` + `keyword` multi-field convention
- **Evidence:** [Query DSL docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/query-dsl.html); the field was analyzed at index time but the term query is not analyzed at query time
- **Impact:** Lexum must adopt the `text`/`keyword` multi-field convention (`fields: { raw: { type: keyword } }`) for schema compatibility — clients and tools assume `field.keyword` exists for exact matching, sorting, and aggregations.
- **Confidence:** High

## 4.4 Relevance scoring

- Default similarity is **BM25** (since ES 5.0, replacing TF/IDF), with `k1 = 1.2`, `b = 0.75` configurable per field via the `similarity` setting ([similarity docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-similarity.html)).
- Scores are computed **per shard** (IDF from shard-local statistics) — with well-distributed data this is fine; `dfs_query_then_fetch` fetches global term statistics first when precision matters on small/skewed indices.
- `_explain` API decomposes a score; `function_score`/`script_score` for boosting by recency/popularity; `rescore` for a second, more expensive ranking pass over top-N; since 8.14 **`retriever`** trees standardize multi-stage retrieval (incl. **RRF** — reciprocal rank fusion — for hybrid lexical+vector ranking).

### F-028 — BM25 (k1=1.2, b=0.75) is the default similarity since ES 5.0, computed per shard from shard-local IDF; `dfs_query_then_fetch` exists for skewed/small indices
- **Evidence:** [Similarity docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-similarity.html)
- **Impact:** Per-shard scoring is an accepted approximation in the ES contract — Lexum's distributed search can adopt the same simplification. The `dfs_query_then_fetch` escape hatch matters only for small/skewed indices.
- **Confidence:** High

### F-029 — Tantivy also implements BM25, so raw scoring parity is natural for Lexum; the gaps are per-field similarity tuning and `_explain` output
- **Evidence:** [Tantivy README](https://github.com/quickwit-oss/tantivy); [ES similarity docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-similarity.html)
- **Impact:** Default relevance should match ES closely out of the box. `_explain` (even approximate) is worth building for debuggability (P1); per-field `similarity` configuration is lower priority.
- **Confidence:** High

## 4.5 ES|QL and SQL

- **Elasticsearch SQL** (6.3+): SQL-to-DSL translation layer, `POST /_sql`, JDBC/ODBC drivers. Useful but limited (no joins, subset of SQL).
- **ES|QL** — a new **piped query language** with its own compute engine (not translated to Query DSL), GA in **8.14** (2024): `FROM logs | WHERE status >= 500 | STATS count = COUNT(*) BY host | SORT count DESC | LIMIT 10` ([GA announcement](https://www.elastic.co/blog/whats-new-elastic-8-14-0), [ES|QL reference](https://www.elastic.co/docs/reference/query-languages/esql)). `LOOKUP JOIN` arrived with 8.18/9.0 ([9.0 blog](https://www.elastic.co/blog/whats-new-elastic-platform-9-0-0)). Elastic is positioning ES|QL as the long-term primary query interface for analytics.

### F-030 — ES|QL (piped language, own compute engine, GA 8.14) shows Elastic itself concluded that a pipe/SQL-style language beats nested JSON for humans and LLMs — Lexum's LQL is convergent evolution, not a deviation
- **Evidence:** [ES|QL GA announcement](https://www.elastic.co/blog/whats-new-elastic-8-14-0), [ES|QL reference](https://www.elastic.co/docs/reference/query-languages/esql), [9.0 blog (`LOOKUP JOIN`)](https://www.elastic.co/blog/whats-new-elastic-platform-9-0-0); Elastic positions ES|QL as the long-term primary analytics interface
- **Impact:** Strategic validation: Lexum should keep LQL as the flagship human/agent interface and treat JSON Query DSL compatibility as the machine/ecosystem interface. ES|QL's `STATS ... BY` shape is the model for extending LQL with aggregations (P2).
- **Confidence:** High

---

Next: [5. Distributed Model](05-distributed-model.md)
