# 2. Architecture

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-006…).

References: [Elasticsearch architecture docs](https://www.elastic.co/docs/deploy-manage/distributed-architecture), [node roles](https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-node.html), [near-real-time search](https://www.elastic.co/guide/en/elasticsearch/reference/current/near-real-time.html), [translog](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html).

## 2.1 Node roles

Every node has one or more roles (`node.roles`):

- **master-eligible** — can be elected master; the elected master owns cluster state (index metadata, mappings, shard allocation table) and is the only node that mutates it. `voting_only` variants participate in elections without becoming master.
- **data** — holds shards; tiered variants: `data_content`, `data_hot`, `data_warm`, `data_cold`, `data_frozen` (frozen = searchable-snapshot-only, cache-backed).
- **ingest** — runs ingest pipelines (pre-index document transforms).
- **coordinating-only** (no roles) — routes requests, does the scatter/gather reduce phase for searches.
- **ml**, **transform**, **remote_cluster_client** — machine learning jobs, continuous transforms, cross-cluster search client.

### F-006 — Every ES node can coordinate any request; roles exist to isolate heavy responsibilities on large clusters
- **Evidence:** [Node roles docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/modules-node.html); design intent stated in ES architecture docs
- **Impact:** Small clusters run all roles on every node — Lexum should follow the same pattern: a symmetric default topology, with role separation as an opt-in scaling tool rather than a deployment requirement (see also the anti-goal on premature tier proliferation, [§7](07-parity-matrix.md)).
- **Confidence:** High

## 2.2 Cluster coordination

- Pre-7.0: **Zen Discovery**, with the infamous `minimum_master_nodes` setting — misconfiguring it caused **split-brain** data loss and was a top operational footgun.
- 7.0+: a rewritten coordination layer (Raft-like leader election + quorum-based cluster-state publication, with formally modeled safety) that computes voting configurations automatically — no quorum setting to misconfigure ([Elastic blog: a new era for cluster coordination](https://www.elastic.co/blog/a-new-era-for-cluster-coordination-in-elasticsearch)).
- Cluster state is versioned and published from master to all nodes (deltas); every node has a full copy.

### F-007 — ES's pre-7.0 user-set quorum (`minimum_master_nodes`) caused split-brain data loss; 7.0 replaced it with auto-computed voting configurations
- **Evidence:** [Elastic blog: a new era for cluster coordination](https://www.elastic.co/blog/a-new-era-for-cluster-coordination-in-elasticsearch)
- **Impact:** Lexum must implement quorum configuration automatically from day one and never expose a user-set quorum. Elastic spent multiple engineer-years plus formal methods on this layer — Lexum should consider embedding an existing Raft implementation (e.g. `openraft`) rather than writing coordination from scratch.
- **Confidence:** High

### F-008 — Cluster state is fully replicated to every node, so large cluster states (huge mappings, many indices) slow the whole cluster
- **Evidence:** [ES architecture docs](https://www.elastic.co/docs/deploy-manage/distributed-architecture); versioned master-published deltas, full copy on every node
- **Impact:** A hard scaling ceiling that motivates keeping metadata small. For Lexum: cap dynamic mapping growth (see F-019) and keep per-index metadata compact by design.
- **Confidence:** High

## 2.3 Indices, shards, replicas

- An **index** is split into `number_of_shards` **primary shards**, fixed at creation time (changing requires `_split`/`_shrink`/reindex). Each primary has `number_of_replicas` **replica shards** (changeable live).
- Each shard is a full **Lucene index**. Document routing: `shard = hash(_routing) % number_of_primary_shards` (with a `routing_factor` refinement to support `_split`); `_routing` defaults to `_id`.
- Replicas serve reads and provide HA; primaries handle writes and replicate operations to the **in-sync copy set** tracked in cluster state (allocation IDs + primary terms + sequence numbers).

### F-009 — A shard is a full Lucene index; primary count is fixed at creation and routing is `hash(_routing) % number_of_primary_shards`
- **Evidence:** [ES architecture docs](https://www.elastic.co/docs/deploy-manage/distributed-architecture); `_routing` defaults to `_id`, with a `routing_factor` refinement to support `_split`
- **Impact:** This is the shard model Lexum's planned distribution layer should mirror if it wants ES-compatible semantics: shard-per-Tantivy-index, hash routing, live-changeable replica count, fixed primary count (with the resharding consequences covered in F-038).
- **Confidence:** High

## 2.4 Lucene segments and the write lifecycle

A shard (Lucene index) is a collection of immutable **segments**. Each segment contains: inverted index (terms → postings), **doc values** (columnar per-field storage for sorting/aggregations), stored fields (`_source`), norms, points (BKD trees for numeric/date/geo ranges), and optionally vectors (HNSW graphs).

The write lifecycle every ES engineer must know — and Lexum must reproduce a version of:

1. **Index** — a document write goes to the in-memory indexing buffer **and** is appended to the **translog** (write-ahead log). By default `index.translog.durability: request` — the translog is fsynced before acknowledging the write, so acknowledged writes survive crashes ([translog docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html)).
2. **Refresh** — every `index.refresh_interval` (default **1s**), the in-memory buffer is written as a new searchable segment (no fsync). This is why ES is "**near** real-time": docs are searchable ~1s after indexing, not immediately. `GET` by `_id` bypasses this and is realtime (served from translog if needed). ([near-real-time docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/near-real-time.html))
3. **Flush** — a Lucene **commit**: fsync all segments to disk, then trim the translog. Triggered by translog size/age thresholds. After a flush, crash recovery doesn't need to replay those ops.
4. **Merge** — background merging (Lucene TieredMergePolicy) combines small segments into larger ones and physically expunges deleted docs (deletes/updates only tombstone the old doc via a live-docs bitmap; an update = delete + reindex). Merging is the main background I/O/CPU cost of a healthy index. `_forcemerge` exists for read-only indices.

### F-010 — ES's durability contract: translog is fsynced *before* acknowledging a write by default (`index.translog.durability: request`)
- **Evidence:** [Translog docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html)
- **Impact:** "Acked write ⇒ durable across crashes" is a contract users depend on. Lexum's write acknowledgment semantics must offer the same guarantee (or an explicit, documented weaker mode).
- **Confidence:** High

### F-011 — ES is near-real-time by design: refresh (default 1s) makes docs searchable without fsync; realtime `GET` by `_id` bypasses refresh
- **Evidence:** [Near-real-time docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/near-real-time.html)
- **Impact:** The refresh/flush split (searchability vs durability as separate events) is what makes high-throughput ingest cheap. Lexum must reproduce both halves: `refresh_interval`-style searchability *and* realtime get served from the WAL if needed.
- **Confidence:** High

### F-012 — Tantivy has no translog: WAL, durability, and refresh-interval semantics are entirely Lexum's responsibility
- **Evidence:** Tantivy shares Lucene's segment/commit/merge model (it is a Lucene re-take) but ships no write-ahead log; [translog docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/index-modules-translog.html) describe what ES layers on top
- **Impact:** This is the single most important piece of ES behavior for Lexum to replicate faithfully: users depend on "acked write ⇒ durable" plus "searchable within refresh_interval". Lexum must own this layer itself (which it must already partially do via Tantivy commits — see the parity matrix row on consistency/durability).
- **Confidence:** High

## 2.5 Recovery and replication internals

- **Sequence numbers + primary terms** (6.x+) uniquely order every operation on a shard. Each copy tracks a **local checkpoint** (max seq_no below which all ops are processed) and the primary tracks a **global checkpoint** (safe point across all in-sync copies).
- **Peer recovery**: a starting replica recovers from the primary either by **operation replay** (using **soft deletes** retained in Lucene, guarded by **retention leases**) when it's only slightly behind, or by **file-based copy** of segments when too far behind.
- Failed nodes ⇒ master promotes an in-sync replica to primary (never a stale copy — allocation IDs prevent it) and schedules new replicas elsewhere.

### F-013 — ES replication correctness rests on seq-nos + primary terms + local/global checkpoints, with dual-mode peer recovery (op replay vs file copy) and stale-copy-safe promotion
- **Evidence:** [ES architecture docs](https://www.elastic.co/docs/deploy-manage/distributed-architecture); seq-no machinery introduced in 6.x, retention leases guard op-replay recovery, allocation IDs prevent stale-primary promotion
- **Impact:** This is the reference design for Lexum's replication protocol: operation ordering (seq-nos), safe-point tracking (checkpoints), cheap catch-up (op replay with retention) and a fallback (segment copy). Designing these in from day one avoids ES's painful 6.x retrofit (F-039).
- **Confidence:** High

---

Next: [3. Core APIs](03-core-apis.md)
