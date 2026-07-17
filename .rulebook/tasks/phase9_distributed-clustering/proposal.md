# Proposal: phase9_distributed-clustering

## Why

This task **supersedes the archived `add-distributed-clustering` task**
(`.rulebook/archive/2026-07-17-add-distributed-clustering/`). That design
was re-planned away for concrete reasons: it put Raft and an
`etcd-client` dependency in the architecture without separating the
metadata plane from the data plane, declared a breaking change ("index
creation now requires shard and replica configuration") that contradicts
the oversharding lesson, and treated correctness testing as absent. The
elastic analysis gives us a better blueprint:

- **F-054** — everything translog/durability/replication is Lexum's to
  build: neither Tantivy nor Lucene ships these primitives, and ES has
  15 years of hardening on top. This is **the single largest engineering
  risk in the plan**; Jepsen-style fault-injection testing must be a
  first-class deliverable, not an afterthought. Tantivy has no WAL —
  a doc acked into an uncommitted Tantivy segment dies with the process
  — so **WAL design is first-class scope of this task**.
- **F-039** — ES retrofitted seq-no replication in 6.x, painfully.
  Sequence numbers + checkpoints go into the replication protocol from
  day one, not bolted on.
- **F-031** — copy ES's availability/consistency trade-off: primary
  forwards in parallel to all **in-sync replicas**, acks after all
  confirm; lagging replicas are evicted from the in-sync set (with
  master confirmation) instead of blocking writes. Not quorum writes.
- **F-036** — cluster-coordination correctness took Elastic ~9 years and
  formal methods. Don't hand-roll: use a proven Raft crate (`openraft`)
  for the **metadata plane only** (cluster state, membership, in-sync
  sets), and spend the saved budget on replication/recovery, which
  cannot be bought off the shelf.
- **F-037/F-038** — default **1 primary shard**, growth via rollover
  (Lexum already has `handlers/rollover.rs` and
  `crates/lexum-core/src/index/rollover.rs`); fixed hash routing,
  resharding is a reindex, mitigated by rollover — the ES-compatible
  choice.
- **F-034/F-035** — adopt and document the honest consistency contract:
  not linearizable, no read-your-writes for search, realtime GET by id;
  publish a resiliency-status page; position Lexum as rebuildable from a
  primary store, which lowers the launch correctness bar.
- **A-02 (Meilisearch F-001)** — sharding/replication being
  Enterprise-only is Meilisearch's biggest community friction point.
  Lexum's distribution ships **fully open-source, never paid-gated** —
  the clearest differentiation opportunity we have.

Reuse, explicitly planned: **phase8 federation is the scatter-gather
query path** (F-032's query-then-fetch maps onto the federated merge
engine — no second distributed-search code path), and **phase1's task
log is the operation log** replication consumes (a task queue is already
an ordered op log; R-01 called this out as the retrofit-proof
foundation). Phase7 provides the inter-node auth model. Hard
dependencies: phase1, phase7, phase8. Phase6's ops surface
(`_cluster/health`, `_cat/*` — currently single-node stubs in
`crates/lexum-server/src/handlers/admin.rs` and `cluster.rs`) becomes
real here.

## What Changes

1. **Design ADRs before code** (recorded via `rulebook decision
   create`): (a) metadata plane on `openraft` vs alternatives, and what
   lives in cluster state (index metadata, shard routing table, in-sync
   sets, node registry) — no etcd/external dependency; (b) shard model:
   `shard = hash(routing) % primaries`, default 1 primary / 1 replica,
   rollover as the growth story; (c) consistency contract document
   (ES-honest: F-034); (d) WAL format review against prior art,
   including VecLite's frozen SPEC-003 (sibling project,
   `e:/HiveLLM/VecLite/docs/specs/SPEC-003-wal-durability.md`: entry
   framing, durability modes, checkpoint/recovery semantics).
2. **Per-shard WAL (translog equivalent)** in lexum-core, valuable on a
   single node before any cluster exists: append-before-ack, CRC-framed
   entries carrying `{ seq_no, primary_term, op }`, durability modes
   (`request` = fsync per request batch, `async` = interval), local and
   global checkpoints, generation rollover trimmed on Tantivy commit,
   full recovery replay on startup.
3. **Replication (primary/backup, seq-no based)**: primary assigns
   seq_nos, forwards ops in parallel to all in-sync replicas, acks after
   all confirm; replica eviction and re-join via master-confirmed
   in-sync set changes; primary failover promotes the replica with the
   highest global checkpoint; peer recovery = segment file copy + WAL
   replay from the recovery checkpoint (retention leases keep needed WAL
   generations).
4. **Allocation and rebalancing**: minimal decider chain (F-033) — disk
   watermarks, same-shard anti-affinity, recovery throttle — plus a
   simple count-based balancer; whole-shard moves.
5. **Distributed search** = phase8 federation internally: coordinator
   scatter to one copy of each shard, top-k (id + sort keys) merge via
   the federated merge engine, then fetch phase (F-032). Per-shard
   failures degrade to partial results with error metadata (A-03).
6. **Fault-injection harness as a deliverable** (F-054): a
   `tests/`-level cluster harness that spawns N-node clusters in-process
   or as child processes and injects crashes (kill -9), network
   partitions, message delay/loss, and disk-full/fsync faults, checking
   invariants: no acked-write loss, no divergent replicas after
   convergence, no split-brain metadata. Runs in CI on every PR touching
   cluster code.
7. **Ops surface made real**: `_cluster/health` (green/yellow/red from
   actual shard states), `_cat/shards`, node join/leave, replicated by
   the metadata plane. Everything ships in the open-source build (A-02).

## Impact

- Affected specs: `.rulebook/tasks/phase9_distributed-clustering/specs/`
  (WAL format, replication protocol, cluster-state model, consistency
  contract, fault-injection invariants)
- Affected code:
  - New `crates/lexum-core/src/wal/` (WAL, checkpoints, recovery)
  - New `crates/lexum-cluster/` (metadata plane on `openraft`, shard
    routing, replication, allocation, node transport)
  - `crates/lexum-core/src/index/manager.rs` (shard-aware index
    lifecycle), `crates/lexum-core/src/snapshot/` (shard-aware
    snapshots)
  - `crates/lexum-server/src/handlers/cluster.rs`, `admin.rs`,
    `health.rs` (real cluster state), `crates/lexum-server/src/router.rs`
  - Reused: phase8 merge engine (`crates/lexum-core/src/search/
    multi_search.rs`), phase1 task log, phase7 inter-node auth
  - `docker-compose.cluster.yml`, `helm/lexum/` (StatefulSet already
    scaffolded), `k8s/` manifests
- Breaking change: NO (single-node stays the zero-config default —
  1 primary, 0 replicas, no cluster config required; clustering is
  opt-in; existing indices open unchanged with a WAL created alongside)
- User benefit: horizontal scale and HA in the open-source build —
  the moat vs Meilisearch (Enterprise-gated sharding) and the risk
  retired with evidence instead of hope.

## Success criteria

- WAL (single-node): kill -9 at any point after an acked write; on
  restart the document is searchable after recovery replay — verified by
  an automated crash-loop test (hundreds of iterations, randomized kill
  points); `request` durability fsyncs before ack (fault-injected fsync
  proves it).
- 3-node cluster: kill -9 the node holding a primary while indexing at
  sustained load → zero acked-write loss, failover completes, cluster
  returns to green; harness asserts this across ≥ 100 randomized runs.
- Partition tests: metadata plane never accepts two primaries for the
  same shard in the same primary term (no split-brain); divergent
  replica scenario (partition during replication) converges to
  byte-identical searchable state.
- Distributed search returns results identical to a single-node index
  of the same corpus (parity test), with partial-results + error
  metadata when a shard copy is down.
- Fault-injection suite is a CI job, green on merge.
- Benchmarks recorded in `benchmark/`: 3-node / 3-shard indexing
  throughput ≥ 2x single-node on the bench corpus; replicated (1
  replica) indexing ≥ 60% of unreplicated; query p50 overhead of
  coordination ≤ 20% vs single node on single-shard queries.
- Consistency contract + resiliency-status page published in `docs/`.
