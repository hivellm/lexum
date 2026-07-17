# SPEC-011 — Distributed Model: WAL, Replication & Cluster Metadata

| | |
|---|---|
| **Status** | **Draft — pre-ADR.** Phase 9 ADRs 1.1–1.4 (metadata plane, shard model, replication protocol, WAL format) may amend any section of this spec; nothing here is frozen. The WAL section builds on frozen prior art: [VecLite SPEC-003 — WAL & Durability](../../../VecLite/docs/specs/SPEC-003-wal-durability.md). |
| **Phase / tasks** | Phase 9 · tasks 1–8 (`.rulebook/tasks/phase9_distributed-clustering/tasks.md`) |
| **Planning source** | [phase9 proposal](../../.rulebook/tasks/phase9_distributed-clustering/proposal.md); Elastic F-031 (in-sync sets, not quorum), F-032 (query-then-fetch), F-033 (allocation deciders), F-034/F-035 (honest consistency contract, resiliency page), F-036 (don't hand-roll coordination — openraft), F-037/F-038 (1 primary default, rollover growth), F-039 (seq-no from day one), F-054 (everything durability/replication is Lexum's to build; Jepsen-style testing first-class); Meilisearch A-02 (never paid-gated), A-03 (partial results); supersedes archived `add-distributed-clustering` |

Requirement IDs `DST-xxx`. RFC 2119 keywords are normative. Integers little-endian. Writes enter through the SPEC-002 task queue (the task log **is** the replication source); distributed search is SPEC-010's merge engine (no second code path); inter-node calls authenticate per SPEC-009; errors follow the SPEC-003 error contract.

## 1. Model — two planes

```
metadata plane (openraft, small, strongly consistent):
    node registry · index metadata versions · shard routing table
    · in-sync sets · primary terms
data plane (per-shard, seq-no replicated, never waits on Raft):
    client → coordinator → primary(shard) → in-sync replicas
```

- **DST-001** The metadata plane runs `openraft` and holds **only** cluster state (§5). It is **never on the data path**: document writes and searches MUST NOT wait on Raft consensus, except when a write requires an in-sync-set change (replica eviction, DST-044) or a failover (DST-046).
- **DST-002** Single-node remains the zero-config default: no cluster configuration, 1 primary / 0 replicas, no listening cluster transport. Existing indices open unchanged; a WAL is created alongside on first write. The archived task's breaking change ("index creation requires shard/replica config") is explicitly NOT reintroduced.
- **DST-003** Every feature in this spec ships in the default open-source build. No license/edition flag may gate any cluster code path (A-02 — verified by a grep-level CI check).

## 2. Shard model & routing

- **DST-010** `shard = hash(routing_key) % number_of_primaries`; `routing_key` defaults to the document id. `number_of_primaries` is fixed at index creation, **default 1** (F-037). `number_of_replicas` defaults to 0 in single-node mode and 1 when clustering is configured; it is mutable at runtime.
- **DST-011** Resharding is a reindex. The supported growth story is **rollover** (existing `crates/lexum-core/src/index/rollover.rs` + write aliases): new writes roll to a fresh index with more primaries; old data stays put (F-038). No online split/shrink at launch.
- **DST-012** A shard is a full Tantivy index directory plus its WAL (§4) plus any SPEC-012 vector sidecars; shard moves and peer recovery copy the shard as files (§7).

## 3. `seq_no` / `primary_term`

- **DST-020** Every replicated operation carries `(seq_no: u64, primary_term: u64)`. The **primary** assigns `seq_no` densely (+1 per operation) in the order operations leave the SPEC-002 task log for that shard; `primary_term` is assigned by the metadata plane and incremented on every primary promotion of that shard (F-039).
- **DST-021** Per shard copy: `local_checkpoint` = highest `seq_no` such that every operation ≤ it has been durably processed (WAL-appended per the durability mode and applied); `max_seq_no` = highest seq_no seen. The primary computes `global_checkpoint` = min(local checkpoints of the in-sync set) and piggybacks it on replication traffic and heartbeats.
- **DST-022** Replica application is **idempotent by `seq_no`**: an operation with an already-processed `(seq_no, primary_term ≤ current)` is acknowledged without re-application, making primary retries safe.
- **DST-023** Operations are **term-fenced**: a copy MUST reject any replication or checkpoint message whose `primary_term` is lower than the highest term it has acknowledged for that shard (defense against zombie ex-primaries, DST-046).
- **DST-024** `seq_no`/`primary_term` are returned on write responses and honored as `if_seq_no`/`if_primary_term` optimistic-concurrency preconditions (ES-compatible; precondition failure → 409 per SPEC-003).
- **DST-025** `NOOP` operations (§4.2 op 4) exist so a newly promoted primary can fill seq_no gaps left by in-flight operations of the old term, restoring the dense sequence before new writes proceed.

## 4. Per-shard WAL (translog equivalent)

Prior art: VecLite SPEC-003 (frozen). Adopted from it: CRC framing that covers the **header fields and body** (VecLite WAL-011 closed a real gap where a corrupted header was silently misrouted), torn-tail truncation semantics, atomic-unit batches, "durability tunes freshness, never integrity". Diverged: per-shard generation files instead of a single sidecar; `(seq_no, primary_term)` framing instead of a local `seq`; checkpoint = Tantivy commit, not segment sealing.

### 4.1 Files

- **DST-030** Layout per shard copy:

```
<data>/indices/<index_uuid>/<shard_id>/wal/
    wal-<generation:016x>.log     (append-only entry files)
    wal.ckp                       (checkpoint file, fixed 64 bytes, atomic rewrite)
```

  Each `.log` file begins with a 32-byte header: magic `LXWL` (4) · `format_version u32` (=1) · `index_uuid_prefix [8]` · `shard_id u32` · `generation u64` · `created_primary_term u64`. A WAL whose uuid prefix does not match the owning index MUST be ignored with a logged warning (stale sidecar from a copied directory — VecLite WAL-002 lesson).
- **DST-031** `wal.ckp` holds `{ magic "LXCP", format_version, current_generation, min_retained_generation, local_checkpoint, global_checkpoint, max_seq_no, crc32 }`, rewritten via temp-file + rename + fsync. It is the recovery root: a torn `.log` tail never damages the checkpoint file.

### 4.2 Entry format

```
| entry_len u32 | crc32 u32 | seq_no u64 | primary_term u64 | op u8 | reserved u8[3] | body |
```

- **DST-032** `crc32` covers everything after itself: `seq_no`, `primary_term`, `op`, `reserved`, and `body` (per VecLite WAL-011 — a bit flip in any header field is detected, not only in the body). `entry_len` covers the same span.

| `op` | Name | Body |
|---|---|---|
| 1 | `INDEX_BATCH` | serialized document batch (one SPEC-002 task application — the atomic unit) |
| 2 | `DELETE_BATCH` | document ids / terms |
| 3 | `DELETE_BY_QUERY` | serialized query |
| 4 | `NOOP` | reason string (gap fill, DST-025) |

- **DST-033** The whole entry is atomic: a partially applied batch MUST never be observable in memory or after recovery.

### 4.3 Durability modes

Index setting `wal.durability`, per shard:

| Mode | fsync | Guarantee after OS crash |
|---|---|---|
| `request` (default) | before every ack, group-committed across concurrent writers | every acked write is durable |
| `async` | every `wal.sync_interval` (default 5 s) and at Tantivy commit | ≤ interval of acked writes may be lost; files never corrupt |

- **DST-034** In `request` mode an fsync failure MUST fail the ack (no silent loss); the shard copy marks itself failed and re-enters via recovery. "Never corrupt" holds in **both** modes: durability tunes freshness, not integrity (VecLite WAL-020).
- **DST-035** Append-before-ack ordering is unconditional: the Tantivy `IndexWriter` MUST NOT see an operation before its WAL append returns (a doc acked into an uncommitted Tantivy segment dies with the process — F-054; the WAL is what makes the ack honest).

### 4.4 Generations, trimming, recovery

- **DST-036** The active generation rolls at a size threshold (default 512 MiB) and at every durable Tantivy commit. Each durable commit records the `(max_seq_no, generation)` it covers; generations whose entries are all covered by a durable commit AND all retention leases (DST-071) are deleted.
- **DST-037** Recovery on shard open: read `wal.ckp` → replay generations from the last committed checkpoint in order, applying entries with `seq_no` > the commit's covered `max_seq_no` idempotently → a CRC failure or short read terminates replay as the **torn tail**: that entry and everything after it in the file are discarded with a logged warning; entries before it are kept (append-only writing makes mid-file corruption followed by valid entries impossible — VecLite WAL-011 semantics).
- **DST-038** Recovery replay MUST be idempotent with respect to the committed index (DST-022 keying); replayed state ≡ the state a clean run would have produced (property-tested at every entry boundary).

## 5. Metadata plane

- **DST-050** Cluster state (versioned, replicated by openraft): node registry (id, address, roles), index metadata (settings + mapping **versions**, not bulk mapping bodies where avoidable), shard routing table, per-shard in-sync sets and primary terms. Cluster state MUST NOT contain documents, task payloads, or any per-document data. Cluster-state scaling limits (mapping explosion, very high index counts) are accepted and documented (F-036 scars).
- **DST-051** No external coordination dependency: no etcd, no ZooKeeper. openraft state persists locally on master-eligible nodes.
- **DST-052** Membership: static seed-list discovery (config/env); an explicit initial master-eligible set (no quorum guessing); joins/leaves and heartbeats thereafter. A partitioned minority MUST NOT commit cluster-state changes (openraft guarantee, spot-checked by the harness).
- **DST-053** Inter-node transport (HTTP/2 or gRPC — ADR 1.1 decides) authenticates every call with SPEC-009 node credentials; there is no unauthenticated cluster port.
- **DST-054** At most one primary per shard per `primary_term`, enforced by the metadata plane as the single writer of in-sync sets and terms. This is the anti-split-brain guarantee the harness attacks (INV-3).
- **DST-055** `_cluster/health` (green/yellow/red derived from actual shard states), `_cluster/state`, and `_cat/shards` are served from real cluster state, replacing the single-node stubs in `crates/lexum-server/src/handlers/cluster.rs` / `admin.rs`.

## 6. Replication protocol

### 6.1 Primary write path

- **DST-040** State machine per replicated operation (F-031 — in-sync sets, **not** quorum writes):

```
coordinator ──route by DST-010──▶ primary
primary:  validate → assign (seq_no, term)        [from the SPEC-002 task-log order]
          → WAL append (durability mode)
          → apply to local Tantivy writer
          → forward IN PARALLEL to every in-sync replica
replica:  term fence (DST-023) → WAL append → apply → ack(local_checkpoint)
primary:  when ALL in-sync replicas acked → ack client
          → advance & broadcast global_checkpoint
```

- **DST-041** The ack to the client therefore means: the operation is WAL-durable (per mode) on **every** in-sync copy. `wait_for_active_shards`-style options are pre-flight checks only, never consistency guarantees.
- **DST-042** Replica apply failures and timeouts never block writes indefinitely: after the replication timeout (default 30 s) the primary requests **eviction** of the lagging replica from the in-sync set via the metadata plane; only after the master confirms the new in-sync set does the primary ack without that replica (DST-044).
- **DST-043** Application is exactly-once relative to the task log: each SPEC-002 task application maps to exactly one `INDEX_BATCH`/`DELETE_*` entry with one `seq_no`, and idempotent replay (DST-022/038) guarantees no lost and no duplicated application (INV-4).

### 6.2 In-sync set management

- **DST-044** In-sync membership changes (evict, re-add) are cluster-state transactions confirmed by the master. An evicted replica MUST NOT be acked against; it re-enters the in-sync set only after recovering to ≥ the current global checkpoint (§7).
- **DST-045** The in-sync set is the authoritative durability set: any member of it is a valid failover target with zero acked-write loss.

### 6.3 Failover & fencing

- **DST-046** On primary loss the metadata plane promotes an in-sync replica (preferring the highest local checkpoint), increments `primary_term`, and publishes the new routing. The new primary fills seq_no gaps with `NOOP` (DST-025), re-establishes the in-sync set, and resumes.
- **DST-047** Writes from the deposed primary carrying the old term are rejected by every copy (DST-023) and by the metadata plane — the zombie-primary test in the harness MUST show zero old-term operations accepted after promotion.

## 7. Recovery

- **DST-070** Peer recovery (replica bootstrap / shard move) = two phases: (1) copy shard files from a snapshot of the source (reusing `crates/lexum-core/src/snapshot/`, including SPEC-012 vector sidecars) while the source keeps serving; (2) replay the source's WAL from the recovery checkpoint to catch up, then request in-sync admission (DST-044).
- **DST-071** **Retention leases**: each registered recovering/rejoining follower holds a lease pinning WAL generations from its recovery checkpoint onward; DST-036 trimming respects all live leases; leases expire (default 12 h) so a dead follower cannot pin the WAL forever.
- **DST-072** Concurrent recoveries are throttled per node (decider, DST-061) and bandwidth-throttled; recovery MUST NOT starve serving traffic.

## 8. Allocation & rebalancing

- **DST-060** Allocation runs on the elected master through an extensible **decider chain** (F-033); a shard copy is placed only where every decider votes yes.
- **DST-061** Launch decider set (minimal, deliberately): disk watermarks — low 85 % (no new allocations), high 90 % (move shards away), flood 95 % (index becomes write-blocked with a clear SPEC-003 error, auto-released below the watermark); same-shard anti-affinity (a primary and its replica never co-locate on one node); recovery throttle (max concurrent recoveries per node, default 2).
- **DST-062** Balancer: shard-count-per-node evening with hysteresis; whole-shard moves via §7 peer recovery. No write-load or disk-size balancing at launch (recorded as future work).

## 9. Distributed search — SPEC-010 reuse

- **DST-080** A distributed search is a scatter to **one copy of each relevant shard** followed by SPEC-010's `FederatedMergeEngine` over normalized scores (query-then-fetch, F-032): shards return top-k `(docId, sort keys, normalized score)`; the coordinator merges (FED-031/032 ordering) and fetches winning documents in a second phase. There is no second distributed-search code path.
- **DST-081** Copy selection: round-robin over available copies at launch; an adaptive-replica-selection (EWMA) hook is reserved by ADR.
- **DST-082** A failed shard copy is retried on another copy, then degrades to **partial results** (A-03): responses carry `_shards: { total, successful, failed, failures: [...] }`; a fully unreachable shard never turns the response into a hard error unless the caller sets `allow_partial_search_results=false`.
- **DST-083** Parity: an identical corpus indexed single-node and 3-shard/3-node MUST return identical hits and normalized scores for the query suite, modulo the documented FED-032 tie-break (this is why FED-011 bans result-set-relative normalization).

## 10. Correctness invariants — the fault-injection contract

The Jepsen-style harness (child-process clusters + fault proxy: kill -9, pause/resume, partitions, message delay/loss/duplication, fsync failure, disk-full) verifies **four named invariants** on every run. They are the release gate (F-054); a violation is a release blocker, and every window found is published (DST-101).

- **DST-090 · INV-1 Acked-write durability** — every operation acknowledged to a client is present and searchable after any sequence of faults and recoveries (in `request` durability; in `async`, bounded by the documented interval).
- **DST-091 · INV-2 No divergent replicas** — after convergence, all copies of a shard expose identical searchable state (verified by content hashing over doc ids + versions), regardless of partition/failover history.
- **DST-092 · INV-3 Monotonic seq_no per shard** — per shard, `seq_no` assignment is dense and monotonic within a term, terms are monotonic across promotions, and at most one primary exists per term (no split-brain); no copy ever accepts an operation that violates the fence (DST-023).
- **DST-093 · INV-4 No lost/duplicated task application** — the mapping SPEC-002 task ⇄ replicated operation is exactly-once on every copy: no acked task is missing, and no task's effects are applied twice (idempotency keying DST-022).
- **DST-094** Harness cadence: smoke subset on every PR touching `lexum-cluster`/`wal`; full randomized scenario suite (primary kill under load, replica kill during recovery, symmetric and asymmetric partitions, rolling restart, disk-full, ≥ 100 randomized runs each) nightly, with logs and op histories retained as failure artifacts.

## 11. Consistency contract — honest version (F-034/F-035)

Published verbatim in user docs; Lexum MUST NOT market stronger guarantees.

- **DST-100** Lexum guarantees: acked-write durability per §10 INV-1; realtime `GET` by id served via the primary is read-your-writes; per-shard operation order (seq_no); replica convergence (INV-2).
- **DST-101** Lexum does **NOT** guarantee: linearizability (of anything outside the metadata plane); read-your-writes for **search** (refresh interval + replica lag — an acked doc becomes searchable only after a refresh); cross-shard or cross-index transactions or snapshot isolation; monotonic reads across replicas; visibility ordering matching ack ordering across shards. Lexum is a search engine, **not a system of record**: deployments SHOULD keep a primary datastore and treat Lexum as rebuildable. A **resiliency-status page** in `docs/` lists every known window honestly, including found-then-fixed harness discoveries.

## 12. Acceptance criteria

1. **Single-node WAL crash loop**: kill -9 at randomized points after acked writes; on restart every acked doc searchable (≥ 500 iterations nightly, ≥ 50 on PR); injected fsync failure in `request` mode fails the ack (DST-034); torn-tail fuzz at every byte offset of the last entry recovers cleanly (DST-037).
2. **3-node failover**: kill -9 the primary's node under sustained indexing → zero acked-write loss, failover completes, cluster returns green (≥ 100 randomized runs); zombie old-term writes rejected (DST-047).
3. **Partitions**: minority never commits metadata (DST-052); at most one primary per term under asymmetric partition (INV-3); divergent-replica scenario converges to identical state (INV-2).
4. **Search parity & partial results**: DST-083 parity suite green; downed shard copy yields partial results with `_shards` failure metadata (DST-082).
5. **Non-functional**: 3-node/3-shard indexing ≥ 2× single-node; 1-replica indexing ≥ 60 % of unreplicated; coordination overhead ≤ 20 % query p50 on single-shard queries; `async` WAL ≥ 90 % of no-WAL throughput (all in `benchmark/`); single-node zero-config regression suite green (DST-002); no edition gate (DST-003); consistency contract + resiliency page published (DST-100/101).
