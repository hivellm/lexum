## Why

Lexum needs distributed clustering to scale beyond single-node capacity and provide high availability. Without clustering, the system cannot handle large datasets or survive node failures, making it unsuitable for production use at scale.

## What Changes

- Implement Raft-based consensus for cluster coordination
- Add node discovery and heartbeat mechanism
- Implement leader election
- Add shard allocation and routing
- Implement data replication across nodes
- Add cluster state management
- Implement failover and recovery mechanisms
- Add inter-node gRPC communication
- **BREAKING**: Index creation now requires shard and replica configuration

## Impact

- Affected specs: `clustering`, `shard-management`, `replication`, `node-discovery`
- Affected code: Adds to `lexum-core/src/cluster/`:
  - `raft.rs` - Raft consensus
  - `discovery.rs` - Node discovery
  - `shard.rs` - Shard management
  - `replica.rs` - Replication
  - `state.rs` - Cluster state
  - `transport.rs` - gRPC communication
- Dependencies: raft-rs, tonic (gRPC), etcd-client
- Performance target: 3-node cluster should achieve 30K docs/sec indexing
- Breaking change: Existing single-node indices need migration

