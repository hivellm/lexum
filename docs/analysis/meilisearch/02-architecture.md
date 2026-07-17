# 2. Architecture: Workspace, Crates, and Layering

> Part of the [Meilisearch analysis](README.md) · Previous: [§1 Overview](01-overview-positioning.md) · Next: [§3 Core Features](03-core-features.md)

Meilisearch is a Rust workspace (100% Rust) organized under `crates/` plus `external-crates/` for vendored MIT crates ([GitHub](https://github.com/meilisearch/meilisearch), [DeepWiki](https://deepwiki.com/meilisearch/meilisearch)).

## Crate map

| Crate | Layer | Responsibility |
|---|---|---|
| **meilisearch** | HTTP | The binary; actix-web REST API, routes, request validation, analytics |
| **index-scheduler** | Coordination | Async task queue: enqueues all writes as tasks, auto-batches, orchestrates processing, webhook notifications |
| **milli** | Engine | The core search/indexing engine: parallel extraction pipeline, inverted indexes, query execution, ranking rules |
| **meilisearch-auth** | Security | API keys, permission scoping (actions × indexes × expiry) |
| **meilisearch-types** | Shared | Type definitions shared across crates (tasks, settings, errors) |
| **dump** | Data | Version-portable export/import ("dumps") |
| **file-store** | Data | Persistence of update files (payloads awaiting processing) |
| **meilitool** | Ops | Offline CLI administration utilities |
| **build-info** | Infra | Compilation metadata |
| **tracing-trace** | Infra | Observability instrumentation |
| **xtask** | Infra | Build automation |

## Foundation libraries (separate repos)

- **[heed](https://github.com/meilisearch/heed)** — "a fully typed LMDB/MDBX wrapper with minimum overhead". All persistent state goes through heed → **LMDB**: memory-mapped ACID transactions, B+ tree, zero-copy reads ([DeepWiki](https://deepwiki.com/meilisearch/meilisearch)).
- **[charabia](https://github.com/meilisearch/charabia)** — the tokenizer: segmentation + normalization for 11+ language groups. Latin (CamelCase splitting, lowercasing), Chinese via **jieba**, Japanese/Korean via **lindera**, Arabic (definite-article segmentation), Hebrew, Greek, Cyrillic, Georgian, Thai, Khmer. Throughput ranges ~2 MiB/s (Korean) to ~36 MiB/s (Arabic). MIT licensed.
- **[arroy → hannoy](https://blog.kerollmops.com/from-trees-to-graphs-speeding-up-vector-search-10x-with-hannoy)** — vector store on LMDB. Originally **arroy** (a Rust port of Spotify's Annoy: random-projection trees on LMDB, see ["Patching LMDB: 3× faster vector store"](https://www.meilisearch.com/blog/3xfaster-vector-store)); replaced by **hannoy**, "a DiskANN-inspired HNSW implementation with LMDB-backed storage" delivering ~10× faster vector search. Hannoy was stabilized as the default (and only) vector store around v1.37, with automatic migration of arroy indexes ([PR #5767](https://github.com/meilisearch/meilisearch/pull/5767), [changelog](https://www.meilisearch.com/docs/changelog/changelog)).
- **grenad** — sorted-string-table-style temporary files used by the (older) indexing pipeline ([blog: Meilisearch is too slow](https://blog.kerollmops.com/meilisearch-is-too-slow)).

## Layering

```
┌────────────────────────────────────────────────┐
│  meilisearch (HTTP server, actix-web)          │  routes, auth middleware, analytics
├────────────────────────────────────────────────┤
│  index-scheduler (task queue)                  │  every write = task; auto-batching;
│                                                │  single processing loop; webhooks
├──────────────┬─────────────────────────────────┤
│ meilisearch- │  milli (engine)                 │  extraction pipeline, inverted index,
│ auth         │   ├─ charabia (tokenizer)       │  ranking rules, filters, facets,
│              │   └─ hannoy (vectors, HNSW)     │  vector search
├──────────────┴─────────────────────────────────┤
│  heed → LMDB (memory-mapped, ACID, B+tree)     │  one environment per index + task DB
└────────────────────────────────────────────────┘
```

Key properties of this layering:

1. **The engine (milli) is a library**, not a server. It has no HTTP or async-runtime dependency. The HTTP layer is a thin, replaceable shell. This is why Meilisearch could add an MCP-ish "chat with your index" layer, dumps, and a CLI tool (`meilitool`) without touching engine internals.
2. **All writes flow through one coordinator** (index-scheduler). There is no code path that mutates an index except via a task. This makes crash recovery, batching, observability (`/tasks`, `/batches`), and dumps trivial to reason about.
3. **Storage is embedded, not a service**. LMDB means zero external dependencies — one process, one directory (`data.ms`).

## Findings

**F-005 — The engine (milli) is a pure library with no HTTP or async-runtime dependency; the HTTP layer is a thin, replaceable shell**
- Evidence: https://github.com/meilisearch/meilisearch · https://deepwiki.com/meilisearch/meilisearch
- Impact: This separation is what let Meilisearch add chat/RAG endpoints, dumps, and `meilitool` without touching engine internals; Lexum's lexum-core / server / CLI split already mirrors it and should be preserved strictly.
- Confidence: high

**F-006 — Every index mutation in Meilisearch goes through a single coordinator (index-scheduler); there is no code path that mutates an index except via a task**
- Evidence: https://deepwiki.com/meilisearch/meilisearch · https://www.meilisearch.com/docs/learn/async/asynchronous_operations
- Impact: Makes crash recovery, auto-batching, observability (`/tasks`, `/batches`), and dumps trivial to reason about. This is the single most important architectural pattern for Lexum to replicate.
- Confidence: high

**F-007 — Storage is embedded (heed → LMDB): zero external dependencies, one process, one `data.ms` directory**
- Evidence: https://github.com/meilisearch/heed · https://www.meilisearch.com/docs/learn/engine/storage
- Impact: The zero-dependency, single-directory deployment story is a major part of Meilisearch's self-hosting appeal; Lexum (Tantivy, also embedded) already matches this — keep it that way.
- Confidence: high

**F-008 — charabia handles segmentation + normalization for 11+ language groups (jieba for Chinese, lindera for Japanese/Korean, Arabic definite-article segmentation, etc.), MIT licensed, throughput ~2 MiB/s (Korean) to ~36 MiB/s (Arabic)**
- Evidence: https://github.com/meilisearch/charabia
- Impact: Multi-language tokenization out of the box is a core Meilisearch DX feature; Lexum can reach parity via Tantivy's tokenizer ecosystem (lindera, jieba crates) exposed as per-index/language configuration.
- Confidence: high

**F-009 — Meilisearch replaced its arroy vector store (Annoy-style random-projection trees) with hannoy, a DiskANN-inspired HNSW on LMDB, delivering ~10× faster vector search; hannoy became the default (and only) vector store around v1.37 with automatic index migration**
- Evidence: https://blog.kerollmops.com/from-trees-to-graphs-speeding-up-vector-search-10x-with-hannoy · https://github.com/meilisearch/meilisearch/pull/5767 · https://www.meilisearch.com/docs/changelog/changelog
- Impact: hannoy (MIT, LMDB-based) is a proven-at-scale Rust vector store candidate for Lexum's future hybrid search; evaluate it first (see [execution plan](08-execution-plan.md), R-13).
- Confidence: high

**F-010 — Lexum lacks an index-scheduler equivalent: there is no task/queue layer between the REST API and Tantivy, while all Meilisearch writes are asynchronous tasks**
- Evidence: Lexum crate layout (lexum-core / server / CLI) vs Meilisearch's index-scheduler crate (https://deepwiki.com/meilisearch/meilisearch); Lexum current feature set per [§7 parity matrix](07-parity-matrix.md) rows 21–24
- Impact: Highest-impact architectural gap. Without it, batching, crash recovery, dumps, webhooks, and replication (a task log is already an operation log) all become harder to retrofit; retrofit cost grows with every new write endpoint.
- Confidence: high

## Lexum takeaway

Lexum's crate split (lexum-core / server / CLI) already mirrors this. The missing piece is the **index-scheduler equivalent**: a first-class task/queue crate between the REST layer and Tantivy, making every write asynchronous, batchable, and observable.
