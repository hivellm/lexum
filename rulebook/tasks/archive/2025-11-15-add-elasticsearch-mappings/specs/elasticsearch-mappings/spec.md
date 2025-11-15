# Elasticsearch Mappings Support Specification

## Overview

This specification defines support for Elasticsearch-compatible index mappings in Lexum. This enables users to migrate from Elasticsearch without changing their mapping definitions and provides a familiar API.

## Goals

1. **Compatibility**: Support Elasticsearch 7.x+ mapping format
2. **Migration**: Enable easy migration from Elasticsearch
3. **Familiarity**: Provide familiar API for Elasticsearch users
4. **Flexibility**: Support dynamic mapping and field auto-detection

## Requirements

### Requirement: GET Mapping Endpoint
The system SHALL expose GET /{index}/_mapping endpoint that returns index mappings in Elasticsearch format.

#### Scenario: Get index mappings
- **WHEN** client sends GET /my_index/_mapping
- **THEN** server returns 200 OK
- **AND** response includes mappings in Elasticsearch format
- **AND** response includes properties with field definitions

#### Scenario: Get non-existent index mappings
- **WHEN** client sends GET /nonexistent/_mapping
- **THEN** server returns 404 Not Found

### Requirement: PUT Mapping Endpoint
The system SHALL expose PUT /{index}/_mapping endpoint to update index mappings.

#### Scenario: Update mappings
- **WHEN** client sends PUT /my_index/_mapping with valid mapping JSON
- **THEN** server returns 200 OK
- **AND** mappings are updated
- **AND** existing documents remain compatible

#### Scenario: Invalid mapping format
- **WHEN** client sends invalid mapping JSON
- **THEN** server returns 400 Bad Request
- **AND** error message explains validation error

### Requirement: Field Types Support
The system SHALL support Elasticsearch field types.

#### Supported Types
- text (with analyzer support)
- keyword (with normalizer support)
- long (64-bit integer)
- double (64-bit float)
- date (with format support)
- boolean
- object (nested objects)
- nested (nested documents)
- geo_point (geographic coordinates)
- ip (IP addresses)
- completion (for suggestions)

### Requirement: Field Parameters
The system SHALL support Elasticsearch field parameters.

#### Parameters
- analyzer (text fields)
- normalizer (keyword fields)
- index (true/false)
- store (true/false)
- index_options (docs, freqs, positions, offsets)
- norms (true/false)
- boost (field boosting)
- copy_to (copy to other fields)
- format (date format)
- ignore_above (keyword max length)

### Requirement: Multi-Field Support
The system SHALL support multi-fields (fields parameter).

#### Scenario: Text field with keyword sub-field
- **WHEN** mapping defines text field with keyword sub-field
- **THEN** field is searchable as text
- **AND** field is searchable as exact keyword
- **AND** both fields are indexed separately

### Requirement: Dynamic Mapping
The system SHALL support dynamic mapping.

#### Dynamic Mapping Modes
- true: Auto-detect and add new fields
- false: Ignore new fields
- strict: Reject documents with new fields

#### Scenario: Auto-detect field types
- **WHEN** dynamic mapping is enabled
- **AND** document contains new field
- **THEN** field type is auto-detected
- **AND** field is added to mapping

### Requirement: Schema Conversion
The system SHALL convert between Lexum schemas and Elasticsearch mappings.

#### Scenario: Mapping to schema
- **WHEN** Elasticsearch mapping is provided
- **THEN** mapping is converted to Lexum schema
- **AND** field types are mapped correctly
- **AND** field parameters are preserved

#### Scenario: Schema to mapping
- **WHEN** Lexum schema exists
- **THEN** schema is converted to Elasticsearch mapping format
- **AND** mapping format is valid
- **AND** field types are mapped correctly

## Implementation Notes

### Mapping Format Example

```json
{
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
      "created_at": {
        "type": "date",
        "format": "strict_date_optional_time||epoch_millis"
      },
      "tags": {
        "type": "keyword"
      },
      "location": {
        "type": "geo_point"
      }
    }
  }
}
```

### Type Mapping

| Elasticsearch | Lexum | Tantivy |
|--------------|-------|---------|
| text | Text | Text (analyzed) |
| keyword | Keyword | Text (unanalyzed) |
| long | I64 | I64 |
| double | F64 | F64 |
| date | Date | Date |
| boolean | Boolean | U64 (0/1) |
| object | Object | Nested structure |
| nested | Nested | Separate index |
| geo_point | GeoPoint | Custom (future) |
| ip | Ip | Text (keyword) |
| completion | Completion | Custom (future) |

## Testing Requirements

- Unit tests for mapping parser (>95% coverage)
- Unit tests for schema conversion (>95% coverage)
- Integration tests for mapping endpoints
- Compatibility tests with Elasticsearch sample mappings
- Performance tests for mapping operations

## Migration Guide

Users migrating from Elasticsearch can:

1. Export mappings from Elasticsearch: `GET /index/_mapping`
2. Create index in Lexum with mappings: `PUT /index { "mappings": {...} }`
3. Reindex documents (same format)
4. Verify search behavior matches

## Future Enhancements

- Support for Elasticsearch 8.x specific features
- Advanced analyzers (custom analyzers)
- Field aliases
- Runtime fields
- Index templates with mappings

