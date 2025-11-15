# Component Dependencies (DAG)

Directed Acyclic Graph of component dependencies for the Lexum project.

**Last Updated**: 2024-10-25  
**Version**: 0.1.0 (Planning)

## Overview

This document describes the dependency relationships between Lexum components. Understanding these dependencies is critical for:

- **Development Planning**: Know what to build first
- **Integration Testing**: Test components in the correct order
- **Debugging**: Trace issues through dependency chains
- **Architecture Review**: Identify coupling and potential circular dependencies

## Dependency Levels

Components are organized into levels based on their dependencies:

- **Level 0**: No dependencies (foundational)
- **Level 1**: Depends only on Level 0
- **Level 2**: Depends on Level 0 and/or Level 1
- **Level 3+**: Depends on lower levels

## Visual DAG

```
Level 0 (Foundation)
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Config     │  │   Logging    │  │    Types     │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 1 (Core)            ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│    Error     │  │   Storage    │  │  Serializer  │
│   Handling   │  │              │  │              │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 2 (Engine)          ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│    Index     │  │   Document   │  │    Schema    │
│   Manager    │  │    Store     │  │   Manager    │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 3 (Query)           ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│     LQL      │  │    Query     │  │ Aggregation  │
│    Parser    │  │   Executor   │  │   Engine     │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 4 (Distribution)    ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   Cluster    │  │    Shard     │  │   Replica    │
│   Manager    │  │   Manager    │  │   Manager    │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 5 (API)             ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│   REST API   │  │     MCP      │  │    UMICP     │
│              │  │   Handler    │  │   Handler    │
└──────────────┘  └──────────────┘  └──────────────┘
       │                  │                  │
       └──────────────────┴──────────────────┘
                          │
Level 6 (Gateway)         ▼
                 ┌──────────────┐
                 │  API Gateway │
                 │    Router    │
                 └──────────────┘
```

## Component Details

### Level 0: Foundation

#### Config
- **Purpose**: Configuration management
- **Dependencies**: None
- **Provides**: Configuration parsing, validation, defaults
- **Files**: `lexum-core/src/config/`

#### Logging
- **Purpose**: Structured logging
- **Dependencies**: None
- **Provides**: Log levels, formatters, outputs
- **Files**: `lexum-core/src/logging/`

#### Types
- **Purpose**: Common type definitions
- **Dependencies**: None
- **Provides**: DocumentId, IndexName, Score, etc.
- **Files**: `lexum-core/src/types/`

### Level 1: Core Services

#### Error Handling
- **Purpose**: Error types and handling
- **Dependencies**: Types, Logging
- **Provides**: LexumError, Result types
- **Files**: `lexum-core/src/error/`

#### Storage
- **Purpose**: Persistent storage layer
- **Dependencies**: Config, Logging, Types
- **Provides**: Key-value store, metadata storage
- **Files**: `lexum-core/src/storage/`

#### Serializer
- **Purpose**: Data serialization
- **Dependencies**: Types, Error Handling
- **Provides**: JSON, bincode, custom formats
- **Files**: `lexum-core/src/serializer/`

### Level 2: Search Engine

#### Index Manager
- **Purpose**: Index lifecycle management
- **Dependencies**: Storage, Error Handling, Types
- **Provides**: Create, delete, configure indices
- **Files**: `lexum-core/src/index/`

#### Document Store
- **Purpose**: Document storage and retrieval
- **Dependencies**: Storage, Serializer, Types
- **Provides**: Store, get, update, delete documents
- **Files**: `lexum-core/src/document/`

#### Schema Manager
- **Purpose**: Index schema management
- **Dependencies**: Types, Error Handling
- **Provides**: Schema validation, field types
- **Files**: `lexum-core/src/schema/`

### Level 3: Query Processing

#### LQL Parser
- **Purpose**: Parse Lexum Query Language
- **Dependencies**: Types, Error Handling
- **Provides**: Lexer, parser, AST
- **Files**: `lexum-core/src/query/lql/`

#### Query Executor
- **Purpose**: Execute search queries
- **Dependencies**: Index Manager, Document Store, LQL Parser
- **Provides**: Query execution, result merging
- **Files**: `lexum-core/src/query/executor/`

#### Aggregation Engine
- **Purpose**: Compute aggregations
- **Dependencies**: Query Executor, Types
- **Provides**: Terms, stats, histogram aggregations
- **Files**: `lexum-core/src/aggregation/`

### Level 4: Distribution

#### Cluster Manager
- **Purpose**: Cluster state and coordination
- **Dependencies**: Config, Storage, Error Handling
- **Provides**: Node discovery, leader election, health
- **Files**: `lexum-core/src/cluster/`

#### Shard Manager
- **Purpose**: Sharding and routing
- **Dependencies**: Cluster Manager, Index Manager
- **Provides**: Shard allocation, routing tables
- **Files**: `lexum-core/src/shard/`

#### Replica Manager
- **Purpose**: Replication and failover
- **Dependencies**: Cluster Manager, Shard Manager
- **Provides**: Replication, consistency, recovery
- **Files**: `lexum-core/src/replica/`

### Level 5: Protocol Handlers

#### REST API
- **Purpose**: HTTP REST API
- **Dependencies**: Query Executor, Index Manager, Document Store
- **Provides**: HTTP endpoints, request handling
- **Files**: `lexum-server/src/api/rest/`

#### MCP Handler
- **Purpose**: Model Context Protocol
- **Dependencies**: Query Executor, Serializer
- **Provides**: MCP operations
- **Files**: `lexum-server/src/api/mcp/`

#### UMICP Handler
- **Purpose**: Universal Model Interchange Communication Protocol
- **Dependencies**: Query Executor, Serializer
- **Provides**: Binary protocol handling
- **Files**: `lexum-server/src/api/umicp/`

### Level 6: Gateway

#### API Gateway
- **Purpose**: Request routing and load balancing
- **Dependencies**: REST API, MCP Handler, UMICP Handler, Cluster Manager
- **Provides**: Routing, auth, rate limiting
- **Files**: `lexum-server/src/gateway/`

## Dependency Matrix

| Component | Config | Logging | Types | Error | Storage | Serializer | Index Mgr | Doc Store | Schema | LQL | Query Exec | Agg | Cluster | Shard | Replica | REST | MCP | UMICP | Gateway |
|-----------|--------|---------|-------|-------|---------|------------|-----------|-----------|--------|-----|------------|-----|---------|-------|---------|------|-----|-------|---------|
| Config | - | | | | | | | | | | | | | | | | | | |
| Logging | | - | | | | | | | | | | | | | | | | | |
| Types | | | - | | | | | | | | | | | | | | | | |
| Error | | ✓ | ✓ | - | | | | | | | | | | | | | | | |
| Storage | ✓ | ✓ | ✓ | | - | | | | | | | | | | | | | | |
| Serializer | | | ✓ | ✓ | | - | | | | | | | | | | | | | |
| Index Mgr | | | ✓ | ✓ | ✓ | | - | | | | | | | | | | | | |
| Doc Store | | | ✓ | ✓ | ✓ | ✓ | | - | | | | | | | | | | | |
| Schema | | | ✓ | ✓ | | | | | - | | | | | | | | | | |
| LQL Parser | | | ✓ | ✓ | | | | | | - | | | | | | | | | |
| Query Exec | | | ✓ | ✓ | | | ✓ | ✓ | | ✓ | - | | | | | | | | |
| Agg Engine | | | ✓ | ✓ | | | | | | | ✓ | - | | | | | | | |
| Cluster Mgr | ✓ | ✓ | ✓ | ✓ | ✓ | | | | | | | | - | | | | | | |
| Shard Mgr | | | ✓ | ✓ | | | ✓ | | | | | | ✓ | - | | | | | |
| Replica Mgr | | | ✓ | ✓ | | | | | | | | | ✓ | ✓ | - | | | | |
| REST API | | | ✓ | ✓ | | | ✓ | ✓ | | | ✓ | | | | | - | | | |
| MCP | | | ✓ | ✓ | | ✓ | | | | | ✓ | | | | | | - | | |
| UMICP | | | ✓ | ✓ | | ✓ | | | | | ✓ | | | | | | | - | |
| Gateway | | | ✓ | ✓ | | | | | | | | | ✓ | | | ✓ | ✓ | ✓ | - |

**Legend**: ✓ = Direct dependency

## Development Order

Based on the DAG, components should be developed in this order:

### Sprint 1-2 (Weeks 1-4)
1. Config
2. Logging
3. Types
4. Error Handling

### Sprint 3-4 (Weeks 5-8)
5. Storage
6. Serializer

### Sprint 5-7 (Weeks 9-14)
7. Index Manager
8. Document Store
9. Schema Manager

### Sprint 8-10 (Weeks 15-20)
10. LQL Parser
11. Query Executor
12. Aggregation Engine

### Sprint 11-13 (Weeks 21-26)
13. Cluster Manager
14. Shard Manager
15. Replica Manager

### Sprint 14-16 (Weeks 27-32)
16. REST API
17. MCP Handler
18. UMICP Handler

### Sprint 17-18 (Weeks 33-36)
19. API Gateway

## Testing Dependencies

### Unit Tests
- Each component tested independently
- Mock dependencies
- Fast execution

### Integration Tests
- Test component interactions
- Follow dependency order
- Real dependencies (no mocks)

### Test Execution Order
1. Level 0 components (parallel)
2. Level 1 components (parallel, after Level 0)
3. Level 2 components (parallel, after Level 1)
4. Continue through levels

## Critical Paths

### Path 1: Basic Search
```
Config → Types → Error → Storage → Index Mgr → Query Exec → REST API
```
**Time to MVP**: ~12 weeks

### Path 2: Distributed Search
```
... + Cluster Mgr → Shard Mgr → Replica Mgr → Gateway
```
**Time to Distributed**: +8 weeks (20 weeks total)

### Path 3: Advanced Queries
```
... + LQL Parser → Query Exec (enhanced) → Agg Engine
```
**Time to LQL**: +6 weeks (26 weeks total)

## Circular Dependency Prevention

### Rules
1. **Never** create circular dependencies
2. Use dependency injection for flexibility
3. Use traits/interfaces to break strong coupling
4. Consider creating intermediate abstraction layers

### Detected Risks
- **Query Executor ↔ Aggregation Engine**: Use trait abstraction
- **Cluster Manager ↔ Shard Manager**: Event-based communication
- **Index Manager ↔ Schema Manager**: Schema as value object

## External Dependencies

### Crates
- **tantivy**: Search engine (Level 2 - Index Manager)
- **tokio**: Async runtime (Level 0 - everywhere)
- **axum**: Web framework (Level 5 - REST API)
- **serde**: Serialization (Level 1 - Serializer)
- **rocksdb**: Storage (Level 1 - Storage)
- **raft-rs**: Consensus (Level 4 - Cluster Manager)

## Modification Guidelines

### When Adding a New Component

1. **Identify Dependencies**
   - What does this component need?
   - What level are dependencies at?

2. **Determine Level**
   - Component level = max(dependency levels) + 1

3. **Update DAG**
   - Add component to diagram
   - Add to dependency matrix
   - Update development order

4. **Validate**
   - Ensure no circular dependencies
   - Check if level assignment is correct
   - Update critical paths if needed

### When Modifying Dependencies

1. **Check Impact**
   - What components depend on this?
   - Will the change break anything?

2. **Update Tests**
   - Update integration tests
   - Verify test execution order

3. **Update Documentation**
   - Update this DAG document
   - Update ARCHITECTURE.md
   - Update DEVELOPMENT.md

## Monitoring Dependencies

### Tools
- `cargo tree` - View dependency tree
- `cargo depgraph` - Generate dependency graph
- Custom scripts in `/scripts`

### Metrics
- Dependency depth (target: <6 levels)
- Component coupling (target: <5 dependencies per component)
- Circular dependencies (target: 0)

## See Also

- [Architecture](./ARCHITECTURE.md) - System architecture
- [Roadmap](./ROADMAP.md) - Implementation timeline
- [Development](./DEVELOPMENT.md) - Development guide
- [Contributing](../CONTRIBUTING.md) - Contribution guidelines

---

**Note**: This DAG is a living document and should be updated whenever component dependencies change.

**Maintained by**: Core Team  
**Review Frequency**: Every sprint (2 weeks)

