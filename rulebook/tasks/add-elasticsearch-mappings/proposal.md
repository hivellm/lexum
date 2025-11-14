## Why

Lexum needs Elasticsearch-compatible mappings support to enable easy migration from Elasticsearch and provide a familiar API for users coming from Elasticsearch. This includes:

- Support for Elasticsearch mapping format (GET/PUT /{index}/_mapping)
- Conversion between Lexum schemas and Elasticsearch mappings
- Dynamic mapping support
- Field mapping parameters (analyzer, index, store, etc.)
- Nested and object field types
- Multi-field support

## What Changes

- Add GET /{index}/_mapping endpoint (Elasticsearch-compatible)
- Add PUT /{index}/_mapping endpoint (update mappings)
- Implement Elasticsearch mapping format parser
- Add mapping to schema converter
- Add schema to mapping converter
- Support dynamic mapping (auto-detect field types)
- Add nested and object field types
- Implement multi-field support (text with keyword sub-field)
- Add field mapping parameters (analyzer, normalizer, index_options, etc.)
- Support mapping templates

## Impact

- Affected specs: `rest-api`, `index-management`, `schema-management`
- Affected code: Creates/extends:
  - `lexum-core/src/schema/mapping.rs` - Mapping format handling
  - `lexum-core/src/schema/converter.rs` - Schema <-> Mapping conversion
  - `lexum-server/src/handlers/mapping.rs` - Mapping endpoints
- Dependencies: serde_json for mapping format
- Compatibility: Elasticsearch 7.x+ mapping format
- Migration: Enables easy migration from Elasticsearch

## Benefits

- **Elasticsearch Compatibility**: Users can migrate from Elasticsearch without changing mapping definitions
- **Familiar API**: Elasticsearch users feel at home
- **Dynamic Mapping**: Auto-detect field types from documents
- **Rich Field Types**: Support nested objects, arrays, multi-fields
- **Migration Path**: Clear path from Elasticsearch to Lexum

