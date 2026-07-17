# 5. Distributed Model

> Part of the [Elasticsearch Analysis for Lexum](README.md). Findings continue globally (F-031…).

References: [scalability and resilience docs](https://www.elastic.co/docs/deploy-manage/production-guidance/availability-and-resilience), [shard sizing guidance](https://www.elastic.co/guide/en/elasticsearch/reference/current/size-your-shards.html).

## 5.1 Write and read paths

- **Write**: any node coordinates → routes by `hash(_routing)` to the primary shard's node → primary validates, indexes, appends to translog → forwards the operation **in parallel to all in-sync replicas** → acks to client after all in-sync copies confirm. Not quorum-write: the in-sync set is authoritative and maintained in cluster state by the master; a lagging/failed replica is removed from the set (with master confirmation) rather than blocking writes indefinitely. `wait_for_active_shards` is a pre-flight check only, not a consistency guarantee.
- **Read (search)**: two-phase **query then fetch**. Coordinator fans out to one copy of every relevant shard (primaries or replicas, chosen by **adaptive replica selection** based on response-time EWMAs); each shard returns top-k (id, score/sort keys); coordinator merges and fetches the winning docs in phase 2. `_search_shards`, per-request `preference`, and shard-level `can_match` pre-filtering (skip shards whose ranges can't match) optimize this.

### F-031 — ES writes are not quorum writes: the master-maintained in-sync copy set is authoritative, lagging replicas are evicted rather than blocking writes, and `wait_for_active_shards` is only a pre-flight check
- **Evidence:** [Scalability and resilience docs](https://www.elastic.co/docs/deploy-manage/production-guidance/availability-and-resilience)
- **Impact:** This is the replication availability/consistency trade-off Lexum should copy: primary forwards in parallel to all in-sync replicas, acks after all confirm, and set-membership changes (with master confirmation) handle failures. It avoids both quorum-write latency and indefinite blocking.
- **Confidence:** High

### F-032 — Search is two-phase query-then-fetch with adaptive replica selection (response-time EWMAs) and `can_match` shard pre-filtering
- **Evidence:** [Scalability and resilience docs](https://www.elastic.co/docs/deploy-manage/production-guidance/availability-and-resilience); `_search_shards` and per-request `preference` also steer routing
- **Impact:** The scatter/gather reduce shape (shards return top-k ids + sort keys; coordinator merges then fetches winners) is the design template for Lexum's distributed search. `can_match` range-based shard skipping is what makes time-partitioned indices cheap to query.
- **Confidence:** High

## 5.2 Allocation and rebalancing

The master runs allocation through a chain of **allocation deciders** (disk watermarks 85/90/95%, same-shard-not-on-same-node, awareness attributes like rack/zone, tier filtering, throttles) plus a **balancer** that continuously evens out shard count/disk/write-load per node (rewritten as the desired-balance allocator in 8.6+). Operators steer with `cluster.routing.allocation.*`, shard-allocation filtering, and total-shards-per-node limits. Rebalancing moves whole shards (segment file copies), throttled by recovery bandwidth settings.

### F-033 — Allocation is a decider-chain (disk watermarks 85/90/95%, anti-affinity, awareness, throttles) plus a continuous balancer; rebalancing moves whole shards and competes with serving traffic
- **Evidence:** [Shard sizing guidance](https://www.elastic.co/guide/en/elasticsearch/reference/current/size-your-shards.html); desired-balance allocator rewrite in 8.6+
- **Impact:** Rebalancing-vs-stability is a perpetual tuning surface (see F-036 list). Lexum's allocator should start with the minimal decider set (disk watermark, same-shard anti-affinity, recovery throttle) — the extensible decider-chain pattern is worth copying.
- **Confidence:** High

## 5.3 Consistency model — the honest version

- **Not linearizable**; no read-your-writes for search (refresh interval + replica lag). `GET` by id is read-your-writes (realtime get).
- Durability is real (`translog durability: request`), and the seq-no/primary-term/in-sync machinery (6.x+) closed most of the historical data-loss windows famously documented by **Jepsen** analyses of early Elasticsearch; Elastic tracked these publicly in its resiliency status page ([Jepsen: Elasticsearch](https://aphyr.com/posts/317-jepsen-elasticsearch), [Elastic resiliency status](https://www.elastic.co/guide/en/elasticsearch/resiliency/current/index.html)).
- Elastic's own guidance for years: ES is a **search/analytics engine, not a system of record** — keep a primary datastore, treat ES as rebuildable.

### F-034 — ES is not linearizable and offers no read-your-writes for search (refresh interval + replica lag); only realtime `GET` by id is read-your-writes
- **Evidence:** [Availability and resilience docs](https://www.elastic.co/docs/deploy-manage/production-guidance/availability-and-resilience), [near-real-time docs](https://www.elastic.co/guide/en/elasticsearch/reference/current/near-real-time.html)
- **Impact:** Lexum should adopt the same honest contract and document it, rather than promising stronger consistency it cannot cheaply deliver. Matching ES here is both easier and what ecosystem users already expect.
- **Confidence:** High

### F-035 — Jepsen documented real data-loss windows in early ES; the 6.x seq-no machinery closed most of them, and Elastic's own guidance remains "search engine, not system of record"
- **Evidence:** [Jepsen: Elasticsearch](https://aphyr.com/posts/317-jepsen-elasticsearch), [Elastic resiliency status page](https://www.elastic.co/guide/en/elasticsearch/resiliency/current/index.html)
- **Impact:** Two lessons for Lexum: (1) publish a resiliency-status page style of honesty about known windows; (2) position Lexum as rebuildable-from-primary-store, which lowers the correctness bar the distributed layer must clear at launch. Jepsen-style testing of Lexum's replication is recommended (see F-054).
- **Confidence:** High

## 5.4 What's actually hard (Elastic's scars, Lexum's warnings)

1. **Cluster coordination correctness** — took Elastic ~9 years and a formal-methods effort to get right ([§2.2](02-architecture.md)). Don't hand-roll.
2. **Oversharding** — each shard costs heap/file handles/cluster-state; classic guidance is shards of tens of GB (e.g. 10–50GB) and to cap shards per node ([size your shards](https://www.elastic.co/guide/en/elasticsearch/reference/current/size-your-shards.html)).
3. **Cluster-state scaling** — mappings and index metadata replicated everywhere; mapping explosion (F-019) or 100k indices kill masters.
4. **Rebalancing vs stability** — moving shards competes with serving traffic; disk watermarks, recovery throttles, and hot-spot handling are perpetual tuning surfaces.
5. **Fixed primary count** — hash routing means resharding is a reindex (mitigated by `_split`/`_shrink`/rollover).
6. **In-sync replication edge cases** — primary failover, divergent replicas, retention leases: the seq-no machinery is subtle.

### F-036 — Cluster coordination correctness took Elastic ~9 years and formal methods; Lexum should not hand-roll it
- **Evidence:** [Elastic blog: a new era for cluster coordination](https://www.elastic.co/blog/a-new-era-for-cluster-coordination-in-elasticsearch); Zen-era split-brain history ([§2.2](02-architecture.md), F-007)
- **Impact:** Use a proven Raft crate (e.g. `openraft`) for Lexum's coordination layer; the engineering budget saved should go to the replication/recovery layer, which cannot be bought off the shelf.
- **Confidence:** High

### F-037 — Oversharding is the classic ES operational failure; guidance is shards of tens of GB (10–50GB), and ES cut default primaries from 5 to 1 in 7.0 precisely because users overshard
- **Evidence:** [Size your shards](https://www.elastic.co/guide/en/elasticsearch/reference/current/size-your-shards.html); each shard costs heap/file handles/cluster-state
- **Impact:** Defaults matter: Lexum should default to 1 primary shard and make rollover the growth story (see anti-goals, [§7](07-parity-matrix.md)).
- **Confidence:** High

### F-038 — Fixed primary-shard count means resharding is a reindex; ES mitigates with `_split`/`_shrink`/rollover — alternatives (consistent-hash/range schemes) cost ES compatibility
- **Evidence:** [ES architecture docs](https://www.elastic.co/docs/deploy-manage/distributed-architecture); routing formula in [§2.3](02-architecture.md) (F-009)
- **Impact:** A deliberate design decision point for Lexum's distribution layer: copying ES's model keeps semantics compatible and is simpler; a new-engine scheme (consistent-hash or range-based) avoids the reindex cliff but diverges from the ecosystem contract. The document's recommendation is to adopt ES's model plus rollover-based growth.
- **Confidence:** Medium

### F-039 — ES retrofitted seq-no replication in 6.x, painfully; Lexum should design sequence numbers + checkpoints into its replication protocol from the start
- **Evidence:** Seq-no/primary-term machinery introduced in 6.x ([§2.5](02-architecture.md), F-013); in-sync replication edge cases (primary failover, divergent replicas, retention leases) are where the subtlety lives
- **Impact:** This is the single biggest design-order lesson for Lexum's upcoming distributed layer: operation ordering and checkpointing are foundational, not features to bolt on after basic replication works.
- **Confidence:** High

---

Next: [6. Modern Features](06-modern-features.md)
