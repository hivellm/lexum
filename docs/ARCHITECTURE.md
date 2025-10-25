# Lexum Architecture

This document describes the architecture of Lexum, a distributed full-text search engine written in Rust.

## System Overview

Lexum follows a layered, distributed architecture designed for scalability, fault tolerance, and high performance.

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                              │
│  ┌─────────────┐  ┌──────────┐  ┌──────┐  ┌──────────┐         │
│  │ Lexum GUI   │  │ REST API │  │ MCP  │  │  UMICP   │         │
│  │ (Electron)  │  │ Clients  │  │Client│  │  Client  │         │
│  └─────────────┘  └──────────┘  └──────┘  └──────────┘         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Protocol Layer                               │
│  ┌──────────────┐  ┌──────┐  ┌────────┐  ┌──────────────┐     │
│  │StreamableHTTP│  │ MCP  │  │ UMICP  │  │ WebSocket    │     │
│  └──────────────┘  └──────┘  └────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API Gateway                                 │
│  - Request Routing          - Load Balancing                     │
│  - Authentication           - Rate Limiting                      │
│  - Protocol Translation     - Circuit Breaker                    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Coordination Layer                             │
│  ┌──────────────┐  ┌────────────┐  ┌──────────────┐           │
│  │   Cluster    │  │   Shard    │  │  Replica     │           │
│  │  Management  │  │  Routing   │  │  Management  │           │
│  └──────────────┘  └────────────┘  └──────────────┘           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Query Layer                                 │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐  ┌──────────┐     │
│  │   LQL    │  │  Query   │  │ Aggregation│  │  Result  │     │
│  │  Parser  │  │Planner   │  │  Engine    │  │  Merger  │     │
│  └──────────┘  └──────────┘  └────────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     Index Layer                                  │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐  ┌──────────┐     │
│  │ Indexing │  │ Search   │  │  Document  │  │  Field   │     │
│  │  Engine  │  │ Engine   │  │   Store    │  │  Cache   │     │
│  │(Tantivy) │  │(Tantivy) │  │            │  │          │     │
│  └──────────┘  └──────────┘  └────────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Storage Layer                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │  File System │  │   RocksDB    │  │    S3/Blob   │         │
│  │  (Segments)  │  │  (Metadata)  │  │   (Backup)   │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

## Core Components

### 1. API Gateway

The API Gateway is the entry point for all client requests.

**Responsibilities:**
- Request routing to appropriate nodes
- Load balancing across cluster nodes
- Authentication and authorization
- Rate limiting and throttling
- Protocol translation (HTTP, MCP, UMICP)
- Request/response logging
- Circuit breaker pattern

**Implementation:**
- Built with Axum framework
- Async request handling with Tokio
- TLS termination
- Connection pooling

### 2. Coordination Layer

Manages cluster state and distributed operations.

#### Cluster Manager
- **Node Discovery**: Automatic node registration and heartbeats
- **Health Monitoring**: Track node health and availability
- **Cluster State**: Maintain global cluster metadata
- **Leader Election**: Raft-based leader election

#### Shard Router
- **Shard Assignment**: Distribute shards across nodes
- **Routing Tables**: Maintain shard-to-node mappings
- **Request Routing**: Direct queries to correct shards
- **Rebalancing**: Automatic shard rebalancing

#### Replica Manager
- **Replication**: Synchronize data across replicas
- **Failover**: Automatic failover on node failure
- **Recovery**: Rebuild replicas from primary shards
- **Consistency**: Maintain replica consistency

**Technology:**
- Raft consensus algorithm
- etcd-like distributed key-value store
- gRPC for inter-node communication

### 3. Query Layer

Processes and executes search queries.

#### LQL Parser
Parses Lexum Query Language into query AST.

```rust
// LQL Example
FROM users 
| WHERE age > 18 AND status = "active"
| MATCH "search terms" IN bio
| SORT created_at DESC
| LIMIT 100
```

**Features:**
- Recursive descent parser
- Type checking
- Query validation
- Optimization hints

#### Query Planner
Creates efficient execution plans.

**Optimizations:**
- Filter pushdown
- Predicate reordering
- Index selection
- Statistics-based optimization

#### Aggregation Engine
Handles complex aggregations.

**Supported Aggregations:**
- Terms aggregation
- Stats (min, max, avg, sum)
- Histogram
- Date histogram
- Nested aggregations
- Pipeline aggregations

#### Result Merger
Combines results from multiple shards.

**Operations:**
- Score-based merging
- Distributed sorting
- Top-K selection
- Deduplication

### 4. Index Layer

Core search and indexing functionality powered by Tantivy.

#### Indexing Engine
- **Document Analysis**: Tokenization, filtering, stemming
- **Index Building**: Inverted index construction
- **Segment Management**: Merge policies, compaction
- **Real-time Indexing**: Near real-time document visibility

**Index Structure:**
```
Index
├── Segment 0
│   ├── Postings (inverted index)
│   ├── Stored Fields (document store)
│   ├── Fast Fields (column store)
│   └── Term Dictionary
├── Segment 1
│   └── ...
└── Metadata
```

#### Search Engine
- **Query Execution**: BM25 scoring
- **Fuzzy Search**: Levenshtein distance
- **Phrase Queries**: Positional matching
- **Range Queries**: Numeric/date ranges
- **Boolean Queries**: Complex logic combinations

#### Document Store
- **Storage**: Compressed document storage
- **Retrieval**: Fast document fetching by ID
- **Updates**: In-place updates when possible
- **Deletion**: Tombstone-based deletion

#### Field Cache
- **Column Store**: Fast field access
- **Sorting**: Efficient sort operations
- **Aggregations**: Quick aggregation computation
- **Filtering**: Fast numeric filtering

### 5. Storage Layer

Persistent data storage.

#### File System Storage
- **Index Segments**: Tantivy segment files
- **Write-Ahead Log**: Transaction log
- **Snapshots**: Point-in-time backups

**Directory Structure:**
```
/data/lexum/
├── indices/
│   ├── index_1/
│   │   ├── shard_0/
│   │   │   ├── segments/
│   │   │   ├── meta.json
│   │   │   └── wal/
│   │   └── shard_1/
│   └── index_2/
├── metadata/
│   └── rocksdb/
└── temp/
```

#### Metadata Storage (RocksDB)
- **Cluster Metadata**: Node info, shard assignments
- **Index Metadata**: Schema, settings, statistics
- **User Data**: Authentication, authorization
- **State**: Cluster state, configuration

#### Backup Storage
- **S3/Blob**: Remote backup storage
- **Incremental Backups**: Delta-based backups
- **Restore**: Point-in-time recovery
- **Replication**: Cross-region replication

## Data Flow

### Indexing Flow

```
Client Request
    │
    ▼
API Gateway
    │
    ▼
Authentication/Authorization
    │
    ▼
Shard Router ──────────────────┐
    │                          │
    ▼                          ▼
Primary Shard            Replica Shards
    │                          │
    ▼                          ▼
Document Analysis        Replication
    │                          │
    ▼                          │
Index Writing                  │
    │                          │
    ▼                          ▼
WAL + Segments           WAL + Segments
    │                          │
    └──────────┬───────────────┘
               ▼
          Acknowledgment
               │
               ▼
          Client Response
```

### Search Flow

```
Client Query
    │
    ▼
API Gateway
    │
    ▼
LQL Parser
    │
    ▼
Query Planner
    │
    ▼
Shard Router ────────────────────┐
    │                            │
    ▼                            ▼
Shard 0 Query              Shard N Query
    │                            │
    ▼                            ▼
Search Engine              Search Engine
    │                            │
    ▼                            ▼
Partial Results            Partial Results
    │                            │
    └──────────┬─────────────────┘
               ▼
         Result Merger
               │
               ▼
        Scoring & Sorting
               │
               ▼
          Aggregations
               │
               ▼
        Final Response
               │
               ▼
        Client (Streamed)
```

## Distributed Architecture

### Sharding Strategy

**Hash-based Sharding:**
```rust
shard_id = hash(document_id) % num_shards
```

**Custom Routing:**
```rust
// Route by specific field
shard_id = hash(document.user_id) % num_shards
```

### Replication Model

**Primary-Replica Replication:**
- One primary shard per shard group
- N-1 replica shards (configurable)
- Synchronous replication for consistency
- Async replication for performance

**Consistency Levels:**
- `ONE`: Return after primary ack
- `QUORUM`: Return after majority ack
- `ALL`: Return after all replicas ack

### Failure Handling

**Node Failure:**
1. Detect failure via heartbeat timeout
2. Mark node as unhealthy
3. Promote replica to primary (if primary failed)
4. Rebalance shards
5. Rebuild failed replicas

**Network Partition:**
1. Detect partition via consensus
2. Majority partition continues
3. Minority partition becomes read-only
4. Merge state after partition heals

## Protocol Support

### StreamableHTTP

HTTP/2 with Server-Sent Events for streaming responses.

**Features:**
- Chunked transfer encoding
- Progressive result delivery
- Connection keep-alive
- Backpressure handling

**Example:**
```
GET /my_index/_search/stream?q=search+terms
Accept: text/event-stream

data: {"doc": {"id": 1, "title": "First result"}}

data: {"doc": {"id": 2, "title": "Second result"}}

data: {"done": true, "total": 2}
```

### MCP (Model Context Protocol)

Integration with AI/LLM systems.

**Operations:**
- `search`: Semantic search
- `retrieve`: Document retrieval
- `aggregate`: Analytics queries
- `stream`: Streaming results

**Example:**
```json
{
  "method": "mcp.search",
  "params": {
    "index": "knowledge_base",
    "query": "What is Lexum?",
    "k": 10
  }
}
```

### UMICP (Universal Model Interchange Communication Protocol)

High-performance binary protocol for model communication.

**Features:**
- Binary serialization (bincode)
- Connection multiplexing
- Flow control
- Compression (zstd)

**Operations:**
- Bulk operations
- Batch queries
- Streaming ingestion
- Pub/sub events

## Performance Optimizations

### Caching
- **Query Cache**: Cache query results
- **Field Cache**: Cache field values for sorting/aggregations
- **Filter Cache**: Cache filter bitsets
- **Request Cache**: HTTP response caching

### Concurrent Processing
- **Parallel Search**: Search shards in parallel
- **Async I/O**: Non-blocking disk I/O
- **Thread Pool**: Separate pools for CPU/IO tasks
- **Lock-free Structures**: Where applicable

### Memory Management
- **Arena Allocation**: Reduce allocation overhead
- **Object Pooling**: Reuse expensive objects
- **Memory Mapping**: mmap index files
- **Compression**: Compress stored fields

### Network Optimization
- **Connection Pooling**: Reuse connections
- **Batching**: Batch small requests
- **Compression**: gzip/zstd response compression
- **HTTP/2**: Multiplexing and header compression

## Security Architecture

### Authentication
- **API Keys**: Token-based authentication
- **OAuth 2.0**: External identity providers
- **TLS Certificates**: mTLS for inter-node auth

### Authorization
- **RBAC**: Role-based access control
- **Document-level Security**: Per-document permissions
- **Field-level Security**: Field masking
- **Audit Logging**: Comprehensive audit trail

### Encryption
- **At Rest**: Encrypted storage (LUKS, dm-crypt)
- **In Transit**: TLS 1.3
- **Backup**: Encrypted backups

## Scalability

### Vertical Scaling
- Add more CPU cores (parallel query execution)
- Add more RAM (larger caches)
- Faster disks (SSDs, NVMe)

### Horizontal Scaling
- Add more nodes
- Increase shard count
- Increase replica count
- Geo-distributed clusters

### Limits
- Max shards per node: 1000
- Max index size: Unlimited (distributed)
- Max document size: 100MB (configurable)
- Max query size: 10MB

## Monitoring & Observability

### Metrics
- Request rates and latencies
- Indexing throughput
- Search performance
- Resource utilization
- Shard statistics

### Tracing
- Distributed request tracing
- Query execution breakdown
- Inter-node communication
- Slow query logging

### Logging
- Structured JSON logs
- Log levels (trace, debug, info, warn, error)
- Log aggregation ready
- Correlation IDs

See [TELEMETRY.md](./TELEMETRY.md) for detailed telemetry documentation.

## Technology Decisions

### Why Rust?
- Memory safety without garbage collection
- Zero-cost abstractions
- Fearless concurrency
- Excellent performance
- Strong type system

### Why Tantivy?
- Pure Rust implementation
- Proven performance (Lucene-inspired)
- Active development
- Rich feature set
- Embeddable

### Why Tokio?
- Industry-standard async runtime
- Excellent performance
- Rich ecosystem
- Mature and stable
- Great tooling

### Why Axum?
- Type-safe routing
- Tower middleware ecosystem
- Minimal boilerplate
- Excellent performance
- Strong typing

## Future Enhancements

- **Vector Search**: Semantic search with embeddings
- **Machine Learning**: ML-based ranking
- **Graph Queries**: Relationship traversal
- **Time-series**: Specialized time-series support
- **Multi-tenancy**: Improved tenant isolation
- **Geo-spatial**: Enhanced geo queries
- **Columnar Storage**: Apache Arrow integration
- **SQL Support**: Standard SQL interface

## References

- [Tantivy Documentation](https://docs.rs/tantivy)
- [Tokio Documentation](https://tokio.rs)
- [Axum Documentation](https://docs.rs/axum)
- [Raft Consensus](https://raft.github.io)
- [OpenTelemetry](https://opentelemetry.io)

