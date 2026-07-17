# 1. Overview and Positioning

> Part of the [Meilisearch analysis](README.md) · Previous: [Index](README.md) · Next: [§2 Architecture](02-architecture.md)

Meilisearch is "a lightning-fast search engine that fits effortlessly into your apps, websites, and workflow" — an open-source, Rust-based search API focused on **instant, user-facing search** rather than analytics ([GitHub](https://github.com/meilisearch/meilisearch)). Started in 2018 by Meili (a French company), it positions itself on three pillars: **performance** (sub-50 ms responses), **relevancy out of the box**, and **developer experience** ([What is Meilisearch](https://www.meilisearch.com/docs/learn/getting_started/what_is_meilisearch)).

Research date: 2026-07-16. Latest Meilisearch release at time of writing: **v1.49.0** (July 6, 2026) ([releases](https://github.com/meilisearch/meilisearch/releases)).

## Licensing and editions

Meilisearch is dual-edition ([GitHub README](https://github.com/meilisearch/meilisearch)):

- **Community Edition** — MIT licensed, fully open source.
- **Enterprise Edition** — commercial license / Business Source License 1.1, gating advanced features such as **sharding/replication** and **S3-streaming snapshots**. Sharding requires Enterprise Edition v1.37+ ([sharding docs](https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview)).

This is a significant strategic data point: Meilisearch kept the single-node engine open and monetizes distribution. Lexum (Apache 2.0, distribution planned as core) can differentiate by keeping sharding/replication open source.

### Findings

**F-001 — Meilisearch gates sharding/replication and S3-streaming snapshots behind the Enterprise Edition (commercial / BSL 1.1)**
- Evidence: https://github.com/meilisearch/meilisearch (README, editions) · https://www.meilisearch.com/docs/resources/self_hosting/sharding/overview (sharding requires Enterprise Edition v1.37+)
- Impact: Lexum's clearest open-source differentiation opportunity — shipping sharding/replication under Apache 2.0 as core targets Meilisearch's biggest community friction point.
- Confidence: high

## vs Elasticsearch

From Meilisearch's own comparison ([docs comparison](https://www.meilisearch.com/docs/resources/comparisons/elasticsearch), [blog](https://www.meilisearch.com/blog/meilisearch-vs-elasticsearch)):

| Dimension | Meilisearch | Elasticsearch |
|---|---|---|
| Setup | Single binary, "ready in minutes", no cluster/shard/replica decisions (standard edition) | Requires nodes, shard count, replicas, heap sizing decisions up front |
| API | "Intuitive REST API can be learned in hours, not months" | Query DSL requiring understanding of analyzers and mappings |
| Relevancy | Zero-config ranking rules, typo tolerance by default | BM25; fuzziness and analysis must be configured |
| Performance | "Under 50ms out-of-the-box" | "Fast with proper tuning" |
| Scale | Single node first; sharding/replication now exists (Enterprise) | Petabyte-scale distributed architecture, hundreds of nodes |
| Resources | Lightweight (LMDB memory-mapped) | Memory-intensive (JVM heap) |
| Sweet spot | App/site search, instant search UX | Log analytics, observability, security analytics, billions of docs |

The essential positioning insight: **Meilisearch does not try to be Elasticsearch**. It deliberately targets the "search bar" use case and refuses the complexity of a general-purpose query DSL, aggregations framework, and cluster management. Elasticsearch's own strength — analytics over massive datasets — is explicitly out of scope.

### Findings

**F-002 — Meilisearch deliberately refuses Elasticsearch's scope (query DSL, aggregations framework, cluster management) and wins on time-to-first-search**
- Evidence: https://www.meilisearch.com/docs/resources/comparisons/elasticsearch · https://www.meilisearch.com/blog/meilisearch-vs-elasticsearch
- Impact: Validates a "small, opinionated API surface with instant-search defaults" as a winning DX strategy; Lexum should not require LQL knowledge for the common path.
- Confidence: high

## vs Typesense and Algolia

([Meilisearch blog comparison](https://www.meilisearch.com/blog/algolia-vs-typesense), [Typesense's comparison](https://typesense.org/typesense-vs-algolia-vs-elasticsearch-vs-meilisearch/)):

- **Algolia**: closed-source, cloud-only, globally distributed (DSN), premium pricing per record/operation; strongest merchandising/e-commerce tooling. Meilisearch is effectively "open-source Algolia".
- **Typesense**: open-source, C++, **fully in-memory** (RAM-bound cost model: pay for RAM + bandwidth), raw speed focus. Meilisearch uses LMDB memory-mapped storage — a middle ground between in-memory speed and disk persistence, so datasets larger than RAM still work (performance degrades gracefully; a RAM-to-disk ratio around 1/3 "does not materially impact performance" per [storage docs](https://www.meilisearch.com/docs/learn/engine/storage)).
- **Meilisearch's edge**: easiest self-hosting experience, AI-native (hybrid search, built-in vector store, model-agnostic embedders), predictable bundled cloud pricing.

### Findings

**F-003 — Meilisearch's memory-mapped LMDB storage is a deliberate middle ground between Typesense's fully in-memory model and disk-bound engines; datasets larger than RAM degrade gracefully (RAM-to-disk ratio ~1/3 "does not materially impact performance")**
- Evidence: https://www.meilisearch.com/docs/learn/engine/storage · https://www.meilisearch.com/blog/algolia-vs-typesense
- Impact: Cost-model positioning matters: Lexum's Tantivy disk-based segments already avoid the RAM-bound trap; worth stating explicitly in Lexum's own comparisons.
- Confidence: high

**F-004 — Meilisearch's current growth edge is being AI-native (hybrid search, built-in vector store, model-agnostic embedders) plus easiest self-hosting**
- Evidence: https://www.meilisearch.com/blog/algolia-vs-typesense · https://typesense.org/typesense-vs-algolia-vs-elasticsearch-vs-meilisearch/
- Impact: Hybrid/vector search is table stakes for a new engine competing in this space; informs the P2 recommendations in the [execution plan](08-execution-plan.md).
- Confidence: medium

## Lexum takeaway

Lexum sits between camps — Tantivy gives it Lucene-class primitives (closer to Elasticsearch DNA), but Meilisearch proves the winning developer experience is a small, opinionated API surface with instant-search defaults. Lexum should offer *both*: LQL power for Elasticsearch-style users, plus a Meilisearch-grade "simple search" path.
