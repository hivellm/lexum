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

## Mappings API

Lexum supports Elasticsearch-compatible index mappings, enabling easy migration from Elasticsearch and providing a familiar API. Mappings define how documents and their fields are stored and indexed.

### Supported Field Types

Lexum supports the following Elasticsearch field types:

- **text**: Full-text searchable text fields (with analyzer support)
- **keyword**: Exact-match keyword fields (not analyzed)
- **long**: 64-bit signed integer
- **double**: 64-bit floating point
- **date**: Date/timestamp (with format support)
- **boolean**: Boolean value
- **object**: Nested objects (flattened to dot notation)
- **nested**: Nested documents (flattened to dot notation)
- **geo_point**: Geographic coordinates (stored as text, future: custom type)
- **ip**: IP addresses (stored as keyword)
- **completion**: Completion type for suggestions (stored as text, future: custom type)

### Field Parameters

Supported field parameters:

- **analyzer**: Analyzer for text fields (stored in mapping)
- **normalizer**: Normalizer for keyword fields (stored in mapping)
- **index**: Whether field is indexed (default: true)
- **store**: Whether field is stored (default: true)
- **index_options**: Index options for text fields (docs, freqs, positions, offsets)
- **norms**: Whether norms are enabled (stored in mapping)
- **boost**: Field boost (applied in query boosting)
- **copy_to**: Copy field value to other fields (✅ implemented - supports string or array format, applied during document indexing)
- **format**: Date format for date fields
- **ignore_above**: Maximum length for keyword fields

### Multi-Field Support

Lexum supports multi-fields, allowing a single source field to be indexed in multiple ways:

```json
{
  "title": {
    "type": "text",
    "analyzer": "standard",
    "fields": {
      "keyword": {
        "type": "keyword",
        "ignore_above": 256
      }
    }
  }
}
```

### GET /{index}/_mapping

Get index mapping.

```bash
curl http://localhost:9200/my_index/_mapping
```

**Response:**
```json
{
  "my_index": {
    "mappings": {
      "properties": {
        "title": {
          "type": "text",
          "analyzer": "standard",
          "fields": {
            "keyword": {
              "type": "keyword",
              "ignore_above": 256
            }
          }
        },
        "price": {
          "type": "double"
        },
        "created_at": {
          "type": "date",
          "format": "strict_date_optional_time||epoch_millis"
        }
      }
    }
  }
}
```

### PUT /{index}/_mapping

Update index mapping (currently returns not implemented - schema updates not supported in Tantivy).

**Note**: Lexum currently does not support updating mappings after index creation due to Tantivy limitations. Mappings must be specified during index creation.

```bash
curl -X PUT http://localhost:9200/my_index/_mapping \
  -H 'Content-Type: application/json' \
  -d '{
    "properties": {
      "new_field": {
        "type": "text"
      }
    }
  }'
```

### GET /{index}/_mapping/{field}

Get mapping for a specific field.

```bash
curl http://localhost:9200/my_index/_mapping/title
```

**Response:**
```json
{
  "my_index": {
    "field": "title",
    "mapping": {
      "type": "text",
      "analyzer": "standard",
      "fields": {
        "keyword": {
          "type": "keyword",
          "ignore_above": 256
        }
      }
    }
  }
}
```

### GET /_mapping

Get mappings for all indices.

```bash
curl http://localhost:9200/_mapping
```

**Response:**
```json
{
  "mappings": {
    "index1": {
      "properties": {
        "title": {
          "type": "text"
        }
      }
    },
    "index2": {
      "properties": {
        "name": {
          "type": "keyword"
        }
      }
    }
  }
}
```

### Creating Index with Mapping

You can specify mappings when creating an index:

```bash
curl -X PUT http://localhost:9200/my_index \
  -H 'Content-Type: application/json' \
  -d '{
    "mappings": {
      "properties": {
        "title": {
          "type": "text",
          "analyzer": "standard",
          "fields": {
            "keyword": {
              "type": "keyword"
            }
          }
        },
        "price": {
          "type": "double"
        },
        "metadata": {
          "type": "object",
          "properties": {
            "author": {
              "type": "keyword"
            }
          }
        }
      }
    }
  }'
```

### Using copy_to Parameter

The `copy_to` parameter allows you to copy field values to other fields during indexing. This is useful for creating a single searchable field from multiple source fields:

```bash
curl -X PUT http://localhost:9200/my_index \
  -H 'Content-Type: application/json' \
  -d '{
    "mappings": {
      "properties": {
        "title": {
          "type": "text",
          "copy_to": "full_text"
        },
        "content": {
          "type": "text",
          "copy_to": "full_text"
        },
        "full_text": {
          "type": "text"
        }
      }
    }
  }'
```

When you index a document:

```json
{
  "title": "My Article",
  "content": "This is the article content"
}
```

The `full_text` field will automatically contain both `title` and `content` values: `["My Article", "This is the article content"]`.

You can also use an array format to copy to multiple fields:

```json
{
  "title": {
    "type": "text",
    "copy_to": ["full_text", "search_fields"]
  }
}
```

### Elasticsearch Compatibility

Lexum supports Elasticsearch 7.x and 8.x mapping formats. The parser automatically handles:
- Standard mapping format: `{ "mappings": { "properties": {...} } }`
- Direct mapping format: `{ "properties": {...} }`
- ES 8.x additional fields (ignored but not rejected): `_source`, `_routing`, `_meta`

### Field Type Mapping

When converting from Lexum schema to Elasticsearch mapping:

| Tantivy Field Type | Elasticsearch Field Type |
|-------------------|--------------------------|
| Str (indexed) | text |
| Str (not indexed) | keyword |
| I64 | long |
| F64 | double |
| Date | date |
| U64/Bool | boolean |
| Bytes | keyword |
| Facet | keyword |
| JsonObject | object |
| IpAddr | ip |

### Dynamic Mapping

Dynamic mapping validation is implemented. You can set the `dynamic` parameter to control how unknown fields are handled:

- **`true`** (default): Unknown fields are allowed but not indexed. Field types can be auto-detected during index creation using `detect_from_document`.
- **`false`**: Unknown fields are ignored and not indexed
- **`strict`**: Documents with unknown fields are rejected with an error (including nested objects)

**Example:**
```json
{
  "mappings": {
    "dynamic": "strict",
    "properties": {
      "title": {
        "type": "text"
      }
    }
  }
}
```

**Note:** Auto-detection of field types is implemented via `detect_from_document` method, which can be used during index creation. However, automatic schema updates after index creation are not supported due to Tantivy's schema limitation (schemas cannot be modified after index creation).

### Limitations

1. **Mapping Updates**: Schema updates are not supported after index creation (Tantivy limitation)
2. **copy_to Parameter**: ✅ Implemented - copy_to is applied during document indexing, copying values from source fields to destination fields
3. **Dynamic Mapping Auto-Detection**: ✅ Implemented - Auto-detection is available via `detect_from_document` method during index creation. Includes date detection, numeric detection, and dynamic templates support. Recursive validation for nested objects in strict mode is also implemented.
4. **Nested Types**: Flattened to dot notation (e.g., `user.name` instead of nested structure)
5. **Custom Analyzers**: Analyzer names are stored but not yet applied to Tantivy schema

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
        "value": "search",
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
```

## Template API

Index templates allow you to automatically apply settings and mappings to new indices based on naming patterns.

### PUT /_template/{name}

Create or update an index template.

**Request Body:**
```json
{
  "index_patterns": ["logs-*", "metrics-*"],
  "priority": 1,
  "version": 1,
  "settings": {
    "number_of_shards": 2,
    "number_of_replicas": 1,
    "refresh_interval": 5,
    "custom": {
      "analysis": {
        "analyzer": "standard"
      }
    }
  },
  "mappings": {
    "properties": {
      "title": {
        "type": "text",
        "analyzer": "standard"
      },
      "timestamp": {
        "type": "date"
      },
      "level": {
        "type": "keyword"
      }
    }
  },
  "order": 0
}
```

**Response:**
```json
{
  "name": "logs-template",
  "acknowledged": true
}
```

**Example:**
```bash
curl -X PUT http://localhost:9200/_template/logs-template \
  -H 'Content-Type: application/json' \
  -d '{
    "index_patterns": ["logs-*"],
    "priority": 1,
    "version": 1,
    "settings": {
      "number_of_shards": 2,
      "number_of_replicas": 1,
      "refresh_interval": 5
    },
    "mappings": {
      "properties": {
        "message": {
          "type": "text",
          "analyzer": "standard"
        },
        "timestamp": {
          "type": "date"
        },
        "level": {
          "type": "keyword"
        }
      }
    },
    "order": 0
  }'
```

### GET /_template/{name}

Get a specific template.

**Response:**
```json
{
  "name": "logs-template",
  "index_patterns": ["logs-*"],
  "priority": 1,
  "version": 1,
  "settings": {
    "number_of_shards": 2,
    "number_of_replicas": 1,
    "refresh_interval": 5
  },
  "mappings": {
    "properties": {
      "message": {
        "type": "text",
        "analyzer": "standard"
      },
      "timestamp": {
        "type": "date"
      },
      "level": {
        "type": "keyword"
      }
    }
  },
  "order": 0
}
```

**Example:**
```bash
curl http://localhost:9200/_template/logs-template
```

### GET /_template

List all templates.

**Response:**
```json
{
  "templates": [
    {
      "name": "logs-template",
      "index_patterns": ["logs-*"],
      "priority": 1,
      "version": 1,
      "settings": {
        "number_of_shards": 2,
        "number_of_replicas": 1,
        "refresh_interval": 5
      },
      "mappings": {
        "properties": {
          "message": {
            "type": "text",
            "analyzer": "standard"
          },
          "timestamp": {
            "type": "date"
          },
          "level": {
            "type": "keyword"
          }
        }
      },
      "order": 0
    }
  ]
}
```

**Example:**
```bash
curl http://localhost:9200/_template
```

### DELETE /_template/{name}

Delete a template.

**Response:**
```json
{
  "name": "logs-template",
  "acknowledged": true
}
```

**Example:**
```bash
curl -X DELETE http://localhost:9200/_template/logs-template
```

### Template Parameters

- **index_patterns**: Array of index patterns this template applies to (required)
- **priority**: Template priority, higher numbers have higher priority (default: 0)
- **version**: Template version number (default: 1)
- **settings**: Index settings to apply
  - **number_of_shards**: Number of primary shards (default: 1)
  - **number_of_replicas**: Number of replica shards (default: 0)
  - **refresh_interval**: Refresh interval in seconds (default: 1)
  - **custom**: Additional custom settings
- **mappings**: Field mappings to apply
  - **properties**: Field definitions
- **order**: Template order for sorting when priority is equal (default: 0)

## LQL API50.5,
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

Restore with index renaming:
```bash
curl -X POST http://localhost:9200/_snapshot/my_backup/snapshot_1/_restore \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": ["old_index"],
    "rename_pattern": "old_(.*)",
    "rename_replacement": "new_$1"
  }'
```

**Response Codes:**
- `200 OK`: Restore operation started successfully
- `400 Bad Request`: Invalid request parameters
- `404 Not Found`: Snapshot or repository not found
- `500 Internal Server Error`: Restore operation failed

**Notes:**
- The snapshot must be in a successful state to be restored
- Restored indices will be created in the `./data/` directory
- If `rename_pattern` and `rename_replacement` are provided, indices will be renamed during restore
- The restore operation validates snapshot integrity before proceeding
- Compressed snapshots are automatically decompressed during restore

### POST /_tasks/{task_id}/_cancel

Cancel a task.

```bash
curl -X POST http://localhost:9200/_tasks/task_123/_cancel
```
- If `rename_pattern` and `rename_replacement` are provided, indices will be renamed during restore
- The restore operation validates snapshot integrity before proceeding
- Compressed snapshots are automatically decompressed during restoremicpRequest {
    method: "search",
    params: SearchParams { ... }
};
let bytes = bincode:### POST /_tasks/{task_id}/_cancel

Cancel a task.

```bash
curl -X POST http://localhost:9200/_tasks/task_123/_cancel
```

### POST /_reindex

Reindex documents from one index to another with comprehensive configuration options.

```bash
curl -X POST http://localhost:9200/_reindex \
  -H 'Content-Type: application/json' \
  -d '{
    "source": {
      "index": "source_index",
      "query": {"match_all": {}},
      "source": ["title", "content"],
      "source_excludes": ["_id"],
      "size": 100,
      "sort": [{"created_at": {"order": "desc"}}],
      "scroll": "5m"
    },
    "dest": {
      "index": "dest_index",
      "version_type": "external",
      "op_type": "create",
      "pipeline": "my_pipeline",
      "routing": "user_id",
      "refresh": true
    },
    "script": {
      "source": "ctx._source.new_field = \"transformed\"",
      "lang": "painless",
      "params": {"multiplier": 2}
    },
    "max_docs": 1000,
    "wait_for_completion": true,
    "refresh": true,
    "timeout": "10m",
    "conflicts": "proceed",
    "retries": 3,
    "requests_per_second": 100.0,
    "slices": 4
  }'
```

#### Source Configuration

- `index`: Source index name (required)
- `query`: Query to filter documents (optional)
- `source`: Fields to include (optional)
- `source_excludes`: Fields to exclude (optional)
- `size`: Number of documents per batch (optional, default: 100)
- `sort`: Sort configuration for consistent ordering (optional)
- `remote`: Remote source configuration for cross-cluster reindexing (optional)
- `scroll`: Scroll timeout for source queries (optional)

#### Remote Source Configuration

- `host`: Remote cluster host (required)
- `username`: Remote cluster username (optional)
- `password`: Remote cluster password (optional)
- `headers`: Custom headers (optional)
- `socket_timeout`: Socket timeout (optional)
- `connect_timeout`: Connection timeout (optional)

#### Destination Configuration

- `index`: Destination index name (required)
- `version_type`: Version type for conflict resolution (optional)
- `op_type`: Operation type - "index" or "create" (optional, default: "index")
- `pipeline`: Pipeline to process documents (optional)
- `routing`: Routing value for documents (optional)
- `refresh`: Refresh policy for destination index (optional)
- `id`: Custom document ID for create operations (optional)

#### Reindex Settings

- `max_docs`: Maximum number of documents to process (optional)
- `wait_for_completion`: Wait for completion before returning (optional, default: false)
- `refresh`: Refresh policy for destination index (optional)
- `timeout`: Timeout for the reindex operation (optional)
- `conflicts`: How to handle version conflicts - "abort" or "proceed" (optional)
- `retries`: Number of retries on failure (optional)
- `requests_per_second`: Throttle requests per second (optional)
- `slices`: Number of slices for parallel processing (optional)ialization (bincode).

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

