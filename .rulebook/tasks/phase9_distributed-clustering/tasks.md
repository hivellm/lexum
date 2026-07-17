## 1. Design ADRs (evidence first — no cluster code before these are recorded)
- [ ] 1.1 ADR: metadata plane — `openraft` for cluster state (index metadata, shard routing table, in-sync sets, node registry) with no external etcd/ZooKeeper dependency (F-036); document what is and is NOT in cluster state (mappings yes, per-doc data never) and the cluster-state-scaling limits accepted
- [ ] 1.2 ADR: shard model — `hash(routing) % primaries`, fixed primary count, default 1 primary / 1 replica, rollover (existing `crates/lexum-core/src/index/rollover.rs`) as the growth story; resharding = reindex, documented (F-037/F-038)
- [ ] 1.3 ADR: replication protocol — seq_no + primary_term from day one (F-039), in-sync sets with master-confirmed membership changes instead of quorum writes (F-031), global/local checkpoints, retention leases; state machine sketched with failure transitions
- [ ] 1.4 ADR: WAL format — entry framing, CRC, durability modes (`request`/`async`), generation/rollover/trim rules; review VecLite SPEC-003 (`e:/HiveLLM/VecLite/docs/specs/SPEC-003-wal-durability.md`) as frozen prior art and record what is adopted vs diverged
- [ ] 1.5 Write and publish the consistency contract in `docs/` (F-034/F-035): not linearizable, no read-your-writes for search, realtime GET by id, rebuildable-from-primary-store positioning; create the resiliency-status page skeleton (known windows listed honestly)
- [ ] 1.6 Confirm reuse seams with running code: phase1 task log exposes an ordered, replayable op stream per index (the replication source), and phase8's federated merge engine is callable as a library (the scatter-gather path) — write the two seam tests that pin these interfaces

## 2. WAL / durability (ships value on a single node before any cluster exists)
- [ ] 2.1 Implement `crates/lexum-core/src/wal/`: append-before-ack log with CRC-framed `{ seq_no, primary_term, op }` entries, per-shard directories, generation files
- [ ] 2.2 Durability modes: `request` (fsync before ack, batched across concurrent writers) and `async` (interval fsync, default 5s) as index settings; wire into the phase1 write path so every task-queue apply is WAL-logged before the Tantivy writer sees it
- [ ] 2.3 Checkpoints: local checkpoint (highest contiguous seq_no processed) and persisted commit checkpoint; trim WAL generations whose ops are covered by a durable Tantivy commit
- [ ] 2.4 Recovery: on startup replay WAL from the last commit checkpoint into the index; torn/corrupt tail entries are detected by CRC and truncated with a logged warning
- [ ] 2.5 Crash-loop harness (first fault-injection deliverable): script spawns the server, indexes with acks, kill -9 at randomized points, restarts, asserts every acked doc is searchable — run ≥ 500 iterations in CI (nightly) and ≥ 50 on PRs
- [ ] 2.6 fsync fault test: with `request` durability, an injected fsync failure must fail the ack (no silent data loss); with disk-full, writes fail cleanly and recovery still works
- [ ] 2.7 Benchmark WAL overhead in `benchmark/`: indexing throughput with `async` ≥ 90% of no-WAL baseline; `request` mode measured and documented

## 3. Metadata plane and membership
- [ ] 3.1 Create `crates/lexum-cluster/` with the `openraft`-backed cluster-state store: typed state (nodes, indices, shard routing table, in-sync sets, primary terms) with versioned, replicated updates
- [ ] 3.2 Node lifecycle: static seed-list discovery (config/env), join/leave, health heartbeats, node attributes (roles reserved for later); no auto-quorum-guessing — explicit initial master-eligible set, auto-configured elections thereafter
- [ ] 3.3 Inter-node transport (HTTP/2 or gRPC — decide in 1.1's ADR) authenticated with phase7 API keys / node credentials
- [ ] 3.4 Metadata partition tests: a partitioned minority never commits cluster-state changes; primary-term uniqueness invariant (at most one primary per shard per term) asserted by the harness
- [ ] 3.5 Real `_cluster/health` / `_cluster/state` / `_cat/shards` from actual cluster state (replace stubs in `crates/lexum-server/src/handlers/cluster.rs` and `admin.rs`)

## 4. Replication (seq-no based, in-sync sets)
- [ ] 4.1 Primary write path: primary assigns `seq_no` (from the phase1 task log ordering), applies locally (WAL + index), forwards in parallel to all in-sync replicas, acks after all confirm (F-031)
- [ ] 4.2 Replica apply path: idempotent apply keyed by seq_no (retries safe), replica-local WAL, local checkpoint reporting to primary; global checkpoint advancement and broadcast
- [ ] 4.3 In-sync set management: primary requests eviction of an unresponsive replica through the metadata plane (master confirms) so writes never block indefinitely; evicted replica re-joins only after recovery to the global checkpoint
- [ ] 4.4 Primary failover: on primary loss the metadata plane promotes an in-sync replica (highest checkpoint), bumping primary_term; old-primary zombie writes with a stale term are rejected (term fencing test)
- [ ] 4.5 Peer recovery: file-based copy of a shard snapshot (reuse `crates/lexum-core/src/snapshot/`) + WAL replay from the recovery checkpoint; retention leases keep WAL generations alive until every registered follower passes them
- [ ] 4.6 Optimistic concurrency surfaced to the API: `seq_no`/`primary_term` returned on writes and honored as `if_seq_no`/`if_primary_term` preconditions (ES P0 item 3 compatibility)

## 5. Allocation and rebalancing
- [ ] 5.1 Decider chain (F-033), minimal set: disk watermarks (85/90/95%), same-shard anti-affinity (primary and replica never co-located), recovery throttle (max concurrent recoveries per node); extensible trait so later deciders slot in
- [ ] 5.2 Balancer: shard-count-per-node evening with hysteresis; whole-shard moves via the peer-recovery mechanism; rebalancing bandwidth throttled and observable
- [ ] 5.3 Allocation tests: node loss triggers replica re-allocation respecting deciders; disk-watermark breach stops new allocations and (at flood stage) blocks writes with a clear error

## 6. Distributed search (reuse phase8 — no second code path)
- [ ] 6.1 Coordinator routes a search to one copy of each relevant shard and merges top-k (docId + sort keys + normalized score) through the phase8 federated merge engine; fetch phase retrieves winning docs (F-032 query-then-fetch)
- [ ] 6.2 Shard-copy selection: primaries or replicas round-robin first; leave an ADR-tracked hook for adaptive replica selection (EWMA) later
- [ ] 6.3 Partial results: a downed shard copy retries the other copy, then degrades to partial results with per-shard error metadata (A-03), surfaced as `_shards: { total, successful, failed }`
- [ ] 6.4 Parity test: identical corpus indexed single-node and 3-shard/3-node returns identical hits and normalized scores for the query test suite (modulo documented tie-break rules)

## 7. Fault-injection harness (Jepsen-style — first-class, gates the release)
- [ ] 7.1 Build the cluster test harness in `tests/`: spawns N-node clusters as child processes with a controllable proxy layer for network faults (partition, delay, loss, duplicate) and process faults (kill -9, pause/resume), plus the §2 disk faults
- [ ] 7.2 Invariant checkers: (a) no acked-write loss — every acked op is present after convergence; (b) no divergent replicas — replicas of a shard converge to identical searchable state; (c) no split-brain — at most one primary per shard per term; (d) metadata linearizability via openraft's own guarantees, spot-checked
- [ ] 7.3 Scenario suite: primary kill during load, replica kill during recovery, symmetric partition, asymmetric partition (primary isolated from master but not from replicas), rolling restart, disk-full on a replica — each ≥ 100 randomized runs nightly
- [ ] 7.4 Wire the harness into CI: smoke subset on every PR touching `lexum-cluster`/`wal`; full randomized suite nightly with failure artifacts (logs + op histories) retained
- [ ] 7.5 Update the resiliency-status page with every window the harness finds (found-then-fixed entries stay documented, F-035 honesty lesson)

## 8. Benchmarks, packaging, release gate
- [ ] 8.1 Benchmarks in `benchmark/`: 3-node/3-shard indexing ≥ 2x single-node; 1-replica indexing ≥ 60% of unreplicated; coordination overhead ≤ 20% query p50 on single-shard queries; publish alongside methodology
- [ ] 8.2 Update `docker-compose.cluster.yml`, `helm/lexum/` (StatefulSet, headless service already scaffolded) and `k8s/` for real cluster bootstrap; document the 3-node quickstart
- [ ] 8.3 Verify every cluster feature ships in the default open-source build with no edition gate (A-02) — grep-level check that no license/edition flag guards cluster code paths
- [ ] 8.4 Single-node regression: full existing test suite + benchmarks unchanged with clustering not configured (zero-config default preserved, archived task's breaking-change requirement explicitly NOT reintroduced)

## 9. Tail (docs + tests — check or waive with tailWaiver)
- [ ] 9.1 Update or create documentation covering the implementation
- [ ] 9.2 Write tests covering the new behavior
- [ ] 9.3 Run tests and confirm they pass
