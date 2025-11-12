# Elasticsearch Mappings Support Implementation Tasks

## Status: 🔵 NOT STARTED

## 1. Core Mapping Format
- [ ] 1.1 Define Elasticsearch mapping structure (properties, field types, parameters)
- [ ] 1.2 Implement mapping parser (JSON -> internal format)
- [ ] 1.3 Implement mapping serializer (internal format -> JSON)
- [ ] 1.4 Add mapping validation
- [ ] 1.5 Support Elasticsearch 7.x+ mapping format
- [ ] 1.6 Support Elasticsearch 8.x mapping format

## 2. Field Types Support
- [ ] 2.1 Support basic types (text, keyword, long, double, date, boolean)
- [ ] 2.2 Add object type support
- [ ] 2.3 Add nested type support
- [ ] 2.4 Add array type support (implicit via multi-value)
- [ ] 2.5 Add geo_point type support
- [ ] 2.6 Add ip type support
- [ ] 2.7 Add completion type support (for suggestions)

## 3. Field Parameters
- [ ] 3.1 Support analyzer parameter (text fields)
- [ ] 3.2 Support normalizer parameter (keyword fields)
- [ ] 3.3 Support index parameter (true/false)
- [ ] 3.4 Support store parameter (true/false)
- [ ] 3.5 Support index_options (docs, freqs, positions, offsets)
- [ ] 3.6 Support norms (true/false)
- [ ] 3.7 Support boost parameter
- [ ] 3.8 Support copy_to parameter
- [ ] 3.9 Support format parameter (date fields)
- [ ] 3.10 Support ignore_above parameter (keyword fields)

## 4. Multi-Field Support
- [ ] 4.1 Support fields parameter (sub-fields)
- [ ] 4.2 Implement text field with keyword sub-field
- [ ] 4.3 Support multiple analyzers on same field
- [ ] 4.4 Support custom sub-field configurations

## 5. Dynamic Mapping
- [ ] 5.1 Implement dynamic mapping (true/false/strict)
- [ ] 5.2 Auto-detect field types from documents
- [ ] 5.3 Support date detection
- [ ] 5.4 Support numeric detection
- [ ] 5.5 Support dynamic templates

## 6. Schema Conversion
- [ ] 6.1 Implement mapping -> schema converter
- [ ] 6.2 Implement schema -> mapping converter
- [ ] 6.3 Handle type mapping (text -> Text, keyword -> Keyword, etc.)
- [ ] 6.4 Convert field parameters to Tantivy options
- [ ] 6.5 Handle nested/object types
- [ ] 6.6 Preserve field metadata during conversion

## 7. REST API Endpoints
- [ ] 7.1 Add GET /{index}/_mapping endpoint
- [ ] 7.2 Add PUT /{index}/_mapping endpoint
- [ ] 7.3 Add GET /{index}/_mapping/{field} endpoint
- [ ] 7.4 Add GET /_mapping endpoint (all indices)
- [ ] 7.5 Support mapping updates (add fields, modify fields)
- [ ] 7.6 Validate mapping updates (no breaking changes)
- [ ] 7.7 Add ToSchema for mapping types

## 8. Index Creation Integration
- [ ] 8.1 Support mappings in PUT /{index} (create index with mappings)
- [ ] 8.2 Support mappings in index templates
- [ ] 8.3 Apply mappings from templates automatically
- [ ] 8.4 Merge mappings from multiple sources

## 9. Testing
- [ ] 9.1 Unit tests for mapping parser
- [ ] 9.2 Unit tests for mapping serializer
- [ ] 9.3 Unit tests for schema conversion
- [ ] 9.4 Integration tests for mapping endpoints
- [ ] 9.5 Test Elasticsearch compatibility (sample mappings)
- [ ] 9.6 Test dynamic mapping scenarios
- [ ] 9.7 Test nested/object fields
- [ ] 9.8 Test multi-field support

## 10. Documentation
- [ ] 10.1 Document mapping format support
- [ ] 10.2 Document Elasticsearch compatibility
- [ ] 10.3 Document migration guide (Elasticsearch -> Lexum)
- [ ] 10.4 Document dynamic mapping behavior
- [ ] 10.5 Add mapping examples
- [ ] 10.6 Document field type mappings

## Summary
- **Status**: Not Started
- **Total Tasks**: 60+
- **Priority**: High (enables Elasticsearch migration)
- **Dependencies**: Requires schema system (already exists)
- **Estimated Effort**: Medium (2-3 weeks)

