# 5. Indexing Pipeline and Storage Model

> Part of the [Meilisearch analysis](README.md) · Previous: [§4 API Design](04-api-design.md) · Next: [§6 Relevancy](06-relevancy.md)

## Storage: LMDB via heed

([storage docs](https://www.meilisearch.com/docs/learn/engine/storage)):

- LMDB chosen over RocksDB and Sled for "the best combination of performance, stability, and features".
- **Memory-mapped**: "all data fetched from LMDB is returned straight from the memory map — no memory allocation or memory copy during data fetches." The OS page cache is the cache; there is no application-level buffer pool.
- ACID transactions; readers never block the writer and vice versa (MVCC with a **single write transaction at a time**).
- Everything lives in one `data.ms` directory.
- Trade-offs Meilisearch accepts and documents honestly:
  - **Space amplification**: an 8.6 MB / 19,553-document JSON dataset → **224 MB on disk**, ~305 MB RAM (indexes for typos, prefixes, proximity, facets are all precomputed).
  - **No space reclamation**: LMDB marks freed pages internally but never shrinks the file; deleting documents doesn't return disk space (compaction requires a copy).
  - Works best when the dataset fits in RAM, but memory-mapping degrades gracefully; NVMe strongly recommended over HDD/network storage.

**F-029 — Meilisearch's storage is LMDB via heed (chosen over RocksDB and Sled): memory-mapped zero-copy reads with the OS page cache as the only cache, ACID MVCC transactions, and a single write transaction at a time**
- Evidence: https://www.meilisearch.com/docs/learn/engine/storage
- Impact: The single-write-transaction constraint shapes Meilisearch's entire write architecture (F-031); understanding it explains why the task-queue pattern exists and why Lexum should copy the pattern, not the storage.
- Confidence: high

**F-030 — Meilisearch accepts and documents ~26× space amplification (8.6 MB / 19,553-doc JSON dataset → 224 MB on disk, ~305 MB RAM) and no space reclamation (LMDB never shrinks the file; deletes don't return disk space; compaction requires a copy)**
- Evidence: https://www.meilisearch.com/docs/learn/engine/storage
- Impact: Concrete cost of precomputing everything (typos, prefixes, proximity, facets); a strong argument against adopting LMDB-style mutable posting lists in Lexum, and a benchmark point Lexum can win on.
- Confidence: high

## Single-writer model + task queue

LMDB's one-write-transaction limit dictates the whole write architecture: since only one write can happen anyway, Meilisearch makes that explicit and productive via the **index-scheduler**: all writes are enqueued as tasks, one processing loop applies them, and **auto-batching** amortizes the per-transaction overhead by merging many document-addition tasks into one giant transaction ([async docs](https://www.meilisearch.com/docs/learn/async/asynchronous_operations)). The batch also amortizes the expensive parts of indexing (prefix/proximity databases are rebuilt once per batch, not per document).

**F-031 — LMDB's single-write-transaction limit dictated the task-queue architecture: since only one write can happen anyway, all writes are enqueued and auto-batching merges many document-addition tasks into one giant transaction, amortizing both transaction overhead and expensive index rebuilds (prefix/proximity databases rebuilt once per batch, not per document)**
- Evidence: https://www.meilisearch.com/docs/learn/async/asynchronous_operations · https://www.meilisearch.com/docs/learn/engine/storage
- Impact: The same economics apply to Lexum — Tantivy commits are expensive exactly like LMDB write transactions, so batching many document additions per commit is the same win (see [execution plan](08-execution-plan.md), R-01).
- Confidence: high

## The indexing pipeline (and its 2024 rewrite)

Clément Renault's ["Meilisearch is too slow"](https://blog.kerollmops.com/meilisearch-is-too-slow) and the ["Indexer edition 2024" PR #4900](https://github.com/meilisearch/meilisearch/pull/4900) document the evolution:

**Old pipeline**: documents were split into ~4 MiB temporary **grenad** files processed by parallel extractors (word docids, word-pair proximity, facets, etc.), each producing sorted files merged and written to LMDB. Problems: O(n log n) sorting with many memcpys and small allocations, long chains of temporary files, and phases where "only a single working thread is active."

**New pipeline (edition 2024)**: exploits the fact that **parallel reads are allowed alongside the single write transaction** in LMDB:

- Extractors read the current index state in parallel (read transactions) while the write transaction stays open.
- Final posting-list bitmaps (roaring bitmaps) are **pre-computed in memory** and written in one batch, using `MDB_APPEND` to write keys in sorted order — minimizing B+-tree rebalancing and fragmentation.
- Streams edited documents instead of materializing intermediate files; per-thread caches minimize I/O.

The vector side got the same treatment: arroy → **hannoy** (DiskANN-inspired HNSW on LMDB) with ~10× faster search and markedly faster incremental indexing ([hannoy blog](https://blog.kerollmops.com/from-trees-to-graphs-speeding-up-vector-search-10x-with-hannoy)), plus **binary quantization** for large embedding sets ([7× faster indexing with BQ](https://blog.kerollmops.com/meilisearch-indexes-embeddings-7x-faster-with-binary-quantization)).

**F-032 — Meilisearch had to rewrite its indexer ("edition 2024", PR #4900) to work around LMDB's single writer: the old grenad temp-file pipeline suffered O(n log n) sorting, memcpy/allocation churn, and single-threaded phases; the new pipeline runs parallel read transactions alongside the open write transaction, pre-computes roaring-bitmap posting lists in memory, and writes keys in sorted order via `MDB_APPEND` to minimize B+-tree rebalancing**
- Evidence: https://blog.kerollmops.com/meilisearch-is-too-slow · https://github.com/meilisearch/meilisearch/pull/4900
- Impact: Years of engineering effort spent recovering parallelism that Tantivy's segment model gives Lexum for free — evidence that Lexum's storage foundation is already the right one for its goals.
- Confidence: high

## Contrast with Lexum's Tantivy model

Tantivy is a Lucene-style engine: immutable **segments**, background merges, near-real-time readers — a fundamentally different storage model from Meilisearch's mutable B+-tree posting lists. Consequences:

- Lexum gets multi-threaded segment writing "for free" (Tantivy indexes in parallel naturally) — Meilisearch had to engineer its way to parallelism against LMDB's single writer.
- Meilisearch gets cheap incremental updates and zero-copy reads; Tantivy pays merge amplification but scales writes better.
- **The lesson is not "adopt LMDB"** — it's that Meilisearch's *coordination layer* (task queue, batching, transactional settings) is what Lexum should replicate, on top of Tantivy's already-good storage engine.

**F-033 — Tantivy's immutable-segment model and Meilisearch's mutable B+-tree posting lists trade opposite costs: Tantivy indexes in parallel naturally but pays merge amplification; Meilisearch gets cheap incremental updates and zero-copy reads but fought for write parallelism — the transferable asset is Meilisearch's coordination layer (task queue, batching, transactional settings), not its storage**
- Evidence: https://blog.kerollmops.com/meilisearch-is-too-slow · https://www.meilisearch.com/docs/learn/engine/storage · Tantivy architecture (Lucene-style segments, Lexum's `lexum-core` dependency, Tantivy 0.25)
- Impact: The central architectural conclusion of this analysis: adopt the index-scheduler pattern on top of Tantivy; do not adopt LMDB/single-writer storage.
- Confidence: high
