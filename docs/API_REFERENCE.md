# API Reference

Complete API reference for Lexum search engine.

## Base URL

```
http://localhost:9200
```

## Authentication

All requests require authentication via one of these methods:

### API Key Header
```bash
curl -H "X-API-Key: your-api-key" http://localhost:9200/
```

### Bearer Token
```bash
curl -H "Authorization: Bearer your-token" http://localhost:9200/
```

### Basic Auth
```bash
curl -u username:password http://localhost:9200/
```

## Response Format

All responses follow this structure:

```json
{
  "success": true,
  "data": { ... },
  "meta": {
    "took": 42,
    "timestamp": "2024-10-25T10:00:00Z"
  }
}
```

Error responses:

```json
{
  "success": false,
  "error": {
    "code": "INVALID_QUERY",
    "message": "Query syntax error at line 1",
    "details": { ... }
  },
  "meta": {
    "timestamp": "2024-10-25T10:00:00Z"
  }
}
```

## Cluster API

### GET /

Get cluster information.

```bash
curl http://localhost:9200/
```

**Response:**
```json
{
  "name": "lexum-node-1",
  "cluster_name": "lexum-prod",
  "version": "0.1.0",
  "tagline": "You Know, for Search"
}
```

### GET /_cluster/health

Get cluster health status.

```bash
curl http://localhost:9200/_cluster/health
```

**Response:**
```json
{
  "cluster_name": "lexum-prod",
  "status": "green",
  "number_of_nodes": 3,
  "number_of_data_nodes": 3,
  "active_primary_shards": 15,
  "active_shards": 30,
  "relocating_shards": 0,
  "initializing_shards": 0,
  "unassigned_shards": 0
}
```

**Status Values:**
- `green`: All shards assigned
- `yellow`: Primary shards assigned, some replicas missing
- `red`: Some primary shards missing

### GET /_cluster/stats

Get detailed cluster statistics.

```bash
curl http://localhost:9200/_cluster/stats
```

### GET /_nodes

Get information about nodes.

```bash
curl http://localhost:9200/_nodes
```

### GET /_nodes/stats

Get node statistics.

```bash
curl http://localhost:9200/_nodes/stats
```

## Index API

### PUT /{index}

Create an index.

```bash
curl -X PUT http://localhost:9200/my_index \
  -H 'Content-Type: application/json' \
  -d '{
    "settings": {
      "number_of_shards": 3,
      "number_of_replicas": 1,
      "refresh_interval": "1s"
    },
    "mappings": {
      "properties": {
        "title": {
          "type": "text",
          "analyzer": "english"
        },
        "content": {
          "type": "text"
        },
        "tags": {
          "type": "keyword"
        },
        "created_at": {
          "type": "date"
        },
        "views": {
          "type": "integer"
        }
      }
    }
  }'
```

**Settings:**
- `number_of_shards`: Number of primary shards (default: 5)
- `number_of_replicas`: Number of replica shards (default: 1)
- `refresh_interval`: How often to refresh (default: "1s")

**Field Types:**
- `text`: Full-text searchable
- `keyword`: Exact matching, aggregatable
- `integer`: 64-bit integer
- `long`: 64-bit integer
- `float`: 64-bit floating point
- `double`: 64-bit floating point
- `boolean`: true/false
- `date`: ISO 8601 timestamp
- `object`: Nested object
- `geo_point`: Geographical coordinates

### GET /{index}

Get index information.

```bash
curl http://localhost:9200/my_index
```

### DELETE /{index}

Delete an index.

```bash
curl -X DELETE http://localhost:9200/my_index
```

### GET /_cat/indices

List all indices.

```bash
curl http://localhost:9200/_cat/indices?v
```

**Response:**
```
health status index    pri rep docs.count docs.deleted store.size pri.store.size
green  open   my_index   3   1     150000            0     45.2mb         22.6mb
```

### POST /{index}/_refresh

Refresh an index.

```bash
curl -X POST http://localhost:9200/my_index/_refresh
```

### POST /{index}/_flush

Flush an index.

```bash
curl -X POST http://localhost:9200/my_index/_flush
```

### POST /{index}/_forcemerge

Force merge segments.

```bash
curl -X POST http://localhost:9200/my_index/_forcemerge?max_num_segments=1
```

## Document API

### POST /{index}/_doc

Index a document (auto-generate ID).

```bash
curl -X POST http://localhost:9200/my_index/_doc \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Introduction to Lexum",
    "content": "Lexum is a high-performance search engine...",
    "tags": ["search", "rust"],
    "created_at": "2024-10-25T10:00:00Z",
    "views": 0
  }'
```

**Response:**
```json
{
  "_index": "my_index",
  "_id": "abc123",
  "_version": 1,
  "result": "created"
}
```

### PUT /{index}/_doc/{id}

Index a document with specific ID.

```bash
curl -X PUT http://localhost:9200/my_index/_doc/doc_1 \
  -H 'Content-Type: application/json' \
  -d '{
    "title": "Custom ID Document",
    "content": "This document has a custom ID"
  }'
```

### GET /{index}/_doc/{id}

Get a document by ID.

```bash
curl http://localhost:9200/my_index/_doc/doc_1
```

**Response:**
```json
{
  "_index": "my_index",
  "_id": "doc_1",
  "_version": 1,
  "_source": {
    "title": "Custom ID Document",
    "content": "This document has a custom ID"
  }
}
```

### POST /{index}/_update/{id}

Update a document.

```bash
curl -X POST http://localhost:9200/my_index/_update/doc_1 \
  -H 'Content-Type: application/json' \
  -d '{
    "doc": {
      "views": 42
    }
  }'
```

### DELETE /{index}/_doc/{id}

Delete a document.

```bash
curl -X DELETE http://localhost:9200/my_index/_doc/doc_1
```

### POST /_bulk

Bulk operations.

```bash
curl -X POST http://localhost:9200/_bulk \
  -H 'Content-Type: application/x-ndjson' \
  -d '
{ "index": { "_index": "my_index", "_id": "1" } }
{ "title": "Doc 1", "content": "Content 1" }
{ "index": { "_index": "my_index", "_id": "2" } }
{ "title": "Doc 2", "content": "Content 2" }
{ "delete": { "_index": "my_index", "_id": "old_doc" } }
{ "update": { "_index": "my_index", "_id": "3" } }
{ "doc": { "views": 100 } }
'
```

**Operations:**
- `index`: Create or replace document
- `create`: Create document (fail if exists)
- `update`: Update existing document
- `delete`: Delete document

## Search API

### GET /{index}/_search

Search using query DSL.

```bash
curl -X POST http://localhost:9200/my_index/_search \
  -H 'Content-Type: application/json' \
  -d '{
    "query": {
      "match": {
        "content": "search engine"
      }
    },
    "size": 10,
    "from": 0,
    "sort": [
      { "_score": "desc" },
      { "created_at": "desc" }
    ]
  }'
```

**Response:**
```json
{
  "took": 15,
  "hits": {
    "total": {
      "value": 150,
      "relation": "eq"
    },
    "max_score": 5.234,
    "hits": [
      {
        "_index": "my_index",
        "_id": "doc_1",
        "_score": 5.234,
        "_source": {
          "title": "...",
          "content": "..."
        }
      }
    ]
  }
}
```

### Query Types

#### Match Query

```json
{
  "query": {
    "match": {
      "content": "search query"
    }
  }
}
```

#### Match Phrase

```json
{
  "query": {
    "match_phrase": {
      "content": "exact phrase"
    }
  }
}
```

#### Multi-Match

```json
{
  "query": {
    "multi_match": {
      "query": "search terms",
      "fields": ["title^3", "content"]
    }
  }
}
```

#### Term Query

```json
{
  "query": {
    "term": {
      "status": "active"
    }
  }
}
```

#### Range Query

```json
{
  "query": {
    "range": {
      "created_at": {
        "gte": "2024-01-01",
        "lte": "2024-12-31"
      }
    }
  }
}
```

#### Bool Query

```json
{
  "query": {
    "bool": {
      "must": [
        { "match": { "content": "search" } }
      ],
      "filter": [
        { "term": { "status": "published" } },
        { "range": { "views": { "gte": 100 } } }
      ],
      "should": [
        { "term": { "featured": true } }
      ],
      "must_not": [
        { "term": { "archived": true } }
      ]
    }
  }
}
```

#### Fuzzy Query

```json
{
  "query": {
    "fuzzy": {
      "title": {
        "value": "serch",
        "fuzziness": 2
      }
    }
  }
}
```

#### Wildcard Query

```json
{
  "query": {
    "wildcard": {
      "title": "rust*"
    }
  }
}
```

### Aggregations

```bash
curl -X POST http://localhost:9200/my_index/_search \
  -H 'Content-Type: application/json' \
  -d '{
    "size": 0,
    "aggs": {
      "tags_count": {
        "terms": {
          "field": "tags",
          "size": 10
        }
      },
      "view_stats": {
        "stats": {
          "field": "views"
        }
      },
      "created_histogram": {
        "date_histogram": {
          "field": "created_at",
          "interval": "1d"
        }
      }
    }
  }'
```

**Response:**
```json
{
  "aggregations": {
    "tags_count": {
      "buckets": [
        { "key": "rust", "doc_count": 150 },
        { "key": "search", "doc_count": 120 }
      ]
    },
    "view_stats": {
      "count": 1000,
      "min": 0,
      "max": 5000,
      "avg": 250.5,
      "sum": 250500
    },
    "created_histogram": {
      "buckets": [
        {
          "key_as_string": "2024-10-25",
          "doc_count": 42
        }
      ]
    }
  }
}
```

### Streaming Search

```bash
curl -X POST http://localhost:9200/my_index/_search/stream \
  -H 'Accept: text/event-stream' \
  -d '{
    "query": {
      "match": { "content": "search" }
    }
  }'
```

**Response (SSE):**
```
data: {"_id":"1","_source":{"title":"Doc 1"}}

data: {"_id":"2","_source":{"title":"Doc 2"}}

data: {"done":true,"total":2}
```

## LQL API

### POST /_lql

Execute LQL query.

```bash
curl -X POST http://localhost:9200/_lql \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM my_index | WHERE views > 100 | SORT created_at DESC | LIMIT 10"
  }'
```

### POST /_lql/explain

Explain LQL query execution plan.

```bash
curl -X POST http://localhost:9200/_lql/explain \
  -H 'Content-Type: application/json' \
  -d '{
    "query": "FROM my_index | WHERE status = \"active\""
  }'
```

**Response:**
```json
{
  "query": "FROM my_index | WHERE status = \"active\"",
  "plan": {
    "type": "IndexScan",
    "index": "my_index",
    "filter": {
      "type": "TermFilter",
      "field": "status",
      "value": "active"
    },
    "estimated_rows": 1500,
    "cost": 120
  }
}
```

## MCP API

### POST /_mcp

Execute MCP request.

```bash
curl -X POST http://localhost:9200/_mcp \
  -H 'Content-Type: application/json' \
  -d '{
    "method": "search",
    "params": {
      "index": "knowledge_base",
      "## Admin API

### PUT /_snapshot/{repository}

Create or update a snapshot repository.

**Request Body:**
```json
{
  "type": "fs",
  "settings": {
    "location": "/path/to/snapshots",
    "compress": "true",
    "chunk_size": "1gb",
    "max_restore_bytes_per_sec": "40mb",
    "max_snapshot_bytes_per_sec": "40mb",
    "readonly": "false"
  }
}
```

**Response:**
```json
{
  "name": "my_backup",
  "type": "fs",
  "settings": {
    "location": "/path/to/snapshots",
    "compress": "true",
    "chunk_size": "1gb",
    "max_restore_bytes_per_sec": "40mb",
    "max_snapshot_bytes_per_sec": "40mb",
    "readonly": "false"
  },
  "snapshot_count": 0,
  "total_size": 0
}
```

**Example:**
```bash
curl -X PUT http://localhost:9200/_snapshot/my_backup \
  -H 'Content-Type: application/json' \
  -d '{
    "type": "fs",
    "settings": {
      "location": "/tmp/snapshots",
      "compress": "true"
    }
  }'
```

### GET /_snapshot/{repository}

Get repository information.

**Response:**
```json
{
  "name": "my_backup",
  "type": "fs",
  "settings": {
    "location": "/path/to/snapshots",
    "compress": "true"
  },
  "snapshot_count": 5,
  "total_size": 1048576
}
```

**Example:**
```bash
curl http://localhost:9200/_snapshot/my_backup
```

### GET /_snapshot

List all snapshot repositories.

**Response:**
```json
[
  {
    "name": "my_backup",
    "type": "fs",
    "settings": {
      "location": "/path/to/snapshots",
      "compress": "true"
    },
    "snapshot_count": 5,
    "total_size": 1048576
  }
]
```

**Example:**
```bash
curl http://localhost:9200/_snapshot
```

### GET /_snapshot/{repository}/_all

List all snapshots in a repository.

**Response:**
```json
{
  "snapshots": [
    {
      "name": "snapshot_1",
      "repository": "my_backup",
      "state": "SUCCESS",
      "indices": ["my_index"],
      "start_time": "2024-01-15T10:30:00Z",
      "end_time": "2024-01-15T10:35:00Z",
      "duration_in_millis": 300000,
      "failures": 0,
      "shards": {
        "total": 1,
        "successful": 1,
        "failed": 0
      },
      "metadata": {
        "user_metadata": {
          "description": "Daily backup"
        },
        "version": "1.0",
        "creation_time": "2024-01-15T10:30:00Z"
      }
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:9200/_snapshot/my_backup/_all
```

### GET /_snapshot/{repository}/{snapshot}

Get information about a specific snapshot.

**Response:**
```json
{
  "name": "snapshot_1",
  "repository": "my_backup",
  "state": "SUCCESS",
  "indices": ["my_index"],
  "start_time": "2024-01-15T10:30:00Z",
  "end_time": "2024-01-15T10:35:00Z",
  "duration_in_millis": 300000,
  "failures": 0,
  "shards": {
    "total": 1,
    "successful": 1,
    "failed": 0
  },
  "metadata": {
    "user_metadata": {
      "description": "Daily backup"
    },
    "version": "1.0",
    "creation_time": "2024-01-15T10:30:00Z"
  }
}
```

**Example:**
```bash
curl http://localhost:9200/_snapshot/my_backup/snapshot_1
```

### DELETE /_snapshot/{repository}/{snapshot}

Delete a snapshot.

**Response:**
```json
{
  "acknowledged": true
}
```

**Example:**
```bash
curl -X DELETE http://localhost:9200/_snapshot/my_backup/snapshot_1
```

### GET /_snapshot/{repository}/_stats

Get snapshot statistics for a repository.

**Response:**
```json
{
  "stats": {
    "total_snapshots": 5,
    "total_size": 1048576,
    "successful_snapshots": 4,
    "failed_snapshots": 1,
    "in_progress_snapshots": 0
  }
}
```

**Example:**
```bash
curl http://localhost:9200/_snapshot/my_backup/_stats
```

### GET /_snapshot/_stats

Get global snapshot statistics across all repositories.

**Response:**
```json
{
  "stats": {
    "total_snapshots": 10,
    "total_size": 2097152,
    "successful_snapshots": 8,
    "failed_snapshots": 2,
    "in_progress_snapshots": 0
  }
}
```

**Example:**
```bash
curl http://localhost:9200/_snapshot/_stats
```

### POST /_snapshot/{repository}/{snapshot}

Create a snapshot. `search`: Semantic search
- `retrieve`: Retrieve documents
- `aggregate`: Run aggregations
- `index`: Index documents

## UMICP API

UMICP uses binary protocol over TCP/WebSocket.

### Connecti### POST /_snapshot/{repository}/{snapshot}/_restore

Restore indices from a snapshot. This operation restores the indices that were included in the snapshot back to the system.

**Parameters:**
- `repository` (string, required): Repository name
- `snapshot` (string, required): Snapshot name

**Request Body:**
```json
{
  "indices": ["index1", "index2"],
  "rename_pattern": "index_(.*)",
  "rename_replacement": "restored_$1",
  "wait_for_completion": false,
  "ignore_unavailable": false,
  "include_global_state": true,
  "include_aliases": true
}
```

**Request Body Parameters:**
- `indices` (array, optional): List of indices to restore. If empty, restores all indices from the snapshot
- `rename_pattern` (string, optional): Regular expression pattern for renaming indices during restore
- `rename_replacement` (string, optional): Replacement pattern for renaming indices
- `wait_for_completion` (boolean, optional): Wait for restore completion before returning (default: false)
- `ignore_unavailable` (boolean, optional): Ignore unavailable indices (default: false)
- `include_global_state` (boolean, optional): Include global state in restore (default: true)
- `include_aliases` (boolean, optional): Include aliases in restore (default: true)

**Response:**
```json
{
  "acknowledged": true,
  "message": "Snapshot restore completed successfully"
}
```

**Examples:**

Restore all indices from a snapshot:
```bash
curl -X POST http://localhost:9200/_snapshot/my_backup/snapshot_1/_restore \
  -H 'Content-Type: application/json' \
  -d '{}'
```

Restore specific indices:
```bash
curl -X POST http://localhost:9200/_snapshot/my_backup/snapshot_1/_restore \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": ["index1", "index2"]
  }'
```

Restore with index renaming:
```bash
curl -X POST http://localhost:9200/_snapshot/my_backup/snapshot_1/_restore \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": ["index1"],
    "rename_pattern": "index_(.*)",
    "rename_replacement": "restored_$1"
  }'
```

**Error Responses:**
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Snapshot or repository not found
- `500 Internal Server Error`: Restore operation failed

**Notes:**
- The snapshot must be in a successful state to be restored
- Restored indices will be created in the `./data/` directory
- If `rename_pattern` and `rename_replacement` are provided, indices will be renamed during restore
- The restore operation validates snapshot integrity before proceeding
- Compressed snapshots are automatically decompressed during restoremicpRequest {
    method: "search",
    params: SearchParams { ... }
};
let bytes = bincode::serialize(&request)?;
stream.send(Message::Binary(bytes)).await?;
```

### Methods

Same as MCP but with binary serialization (bincode).

## Admin API

### POST /_snapshot/{repository}/{snapshot}

Create a snapshot.

```bash
curl -X PUT http://localhost:9200/_snapshot/my_backup/snapshot_1 \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": "my_index",
    "include_global_state": false
  }'
```

### POST /_snapshot/{repository}/{snapshot}/_restore

Restore from snapshot.

```bash
curl -X POST http://localhost:9200/_snapshot/my_backup/snapshot_1/_restore
```

### GET /_tasks

List running tasks.

```bash
curl http://localhost:9200/_tasks
```

### POST /_tasks/{task_id}/_cancel

Cancel a task.

```bash
curl -X POST http://localhost:9200/_tasks/task_123/_cancel
```

## User Management API

### POST /_security/user/{username}

Create a user.

```bash
curl -X POST http://localhost:9200/_security/user/john \
  -H 'Content-Type: application/json' \
  -d '{
    "password": "secure_password",
    "roles": ["admin", "user"],
    "full_name": "John Doe",
    "email": "john@example.com"
  }'
```

### GET /_security/user/{username}

Get user information.

```bash
curl http://localhost:9200/_security/user/john
```

### DELETE /_security/user/{username}

Delete a user.

```bash
curl -X DELETE http://localhost:9200/_security/user/john
```

### POST /_security/role/{role}

Create a role.

```bash
curl -X POST http://localhost:9200/_security/role/read_only \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": [
      {
        "names": ["*"],
        "privileges": ["read"]
      }
    ]
  }'
```

## Monitoring API

### GET /_metrics

Prometheus-format metrics.

```bash
curl http://localhost:9200/_metrics
```

**Response:**
```
# HELP lexum_search_requests_total Total search requests
# TYPE lexum_search_requests_total counter
lexum_search_requests_total{status="success"} 15234

# HELP lexum_search_duration_seconds Search duration
# TYPE lexum_search_duration_seconds histogram
lexum_search_duration_seconds_bucket{le="0.01"} 5000
lexum_search_duration_seconds_bucket{le="0.1"} 14500
lexum_search_duration_seconds_bucket{le="1.0"} 15200
```

### GET /_health

Health check endpoint.

```bash
curl http://localhost:9200/_health
```

**Response:**
```json
{
  "status": "healthy",
  "checks": {
    "cluster": "ok",
    "disk_space": "ok",
    "memory": "ok"
  }
}
```

## Rate Limiting

Rate limits are applied per API key:

**Headers:**
```
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 950
X-RateLimit-Reset: 1698235200
```

**Limits:**
- Default: 1000 requests/minute
- Bulk operations: 100 requests/minute
- Admin operations: 50 requests/minute

## Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `INVALID_QUERY` | 400 | Query syntax error |
| `INDEX_NOT_FOUND` | 404 | Index does not exist |
| `DOCUMENT_NOT_FOUND` | 404 | Document does not exist |
| `UNAUTHORIZED` | 401 | Authentication required |
| `FORBIDDEN` | 403 | Insufficient permissions |
| `CONFLICT` | 409 | Version conflict |
| `TOO_MANY_REQUESTS` | 429 | Rate limit exceeded |
| `INTERNAL_ERROR` | 500 | Internal server error |
| `SERVICE_UNAVAILABLE` | 503 | Service temporarily unavailable |

## SDK Examples

### Rust

```rust
use lexum_client::{Client, Query};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("http://localhost:9200")
        .with_api_key("your-api-key");
    
    let results = client
        .search("my_index")
        .query(Query::match_query("content", "search terms"))
        .size(10)
        .execute()
        .await?;
    
    println!("Found {} results", results.total);
    Ok(())
}
```

### Python

```python
from lexum import Client

client = Client("http://localhost:9200", api_key="your-api-key")

results = client.search(
    index="my_index",
    query={"match": {"content": "search terms"}},
    size=10
)

print(f"Found {results['hits']['total']['value']} results")
```

### JavaScript/TypeScript

```typescript
import { LexumClient } from '@lexum/client';

const client = new LexumClient({
  url: 'http://localhost:9200',
  apiKey: 'your-api-key'
});

const results = await client.search({
  index: 'my_index',
  query: {
    match: { content: 'search terms' }
  },
  size: 10
});

console.log(`Found ${results.hits.total.value} results`);
```

## Versioning

API version is specified in the URL:

```
http://localhost:9200/v1/_search
```

Current version: `v1`

## WebSocket API

Real-time updates via WebSocket:

```javascript
const ws = new WebSocket('ws://localhost:9200/_ws');

ws.on('message', (data) => {
  const event = JSON.parse(data);
  console.log('Event:', event);
});

// Subscribe to index changes
ws.send(JSON.stringify({
  type: 'subscribe',
  index: 'my_index'
}));
```

## Best Practices

1. **Use bulk API** for multiple documents
2. **Implement retry logic** with exponential backoff
3. **Cache frequently accessed data**
4. **Use specific field selection** to reduce payload size
5. **Monitor rate limits** and adjust request rates
6. **Use streaming** for large result sets
7. **Implement pagination** for large datasets
8. **Use filters over queries** when exact matching is needed

## See Also

- [Query Language](./QUERY_LANGUAGE.md)
- [Architecture](./ARCHITECTURE.md)
- [Deployment](./DEPLOYMENT.md)

