## Why

Users need advanced search capabilities beyond basic matching including fuzzy search, phrase queries, wildcards, highlighting, suggestions, and more. These features are essential for production search applications.

## What Changes

- Implement fuzzy search with configurable edit distance
- Add phrase query support
- Implement wildcard queries (prefix, suffix, contains)
- Add regex query support
- Implement field boosting
- Add result highlighting
- Implement search suggestions (autocomplete)
- Add more-like-this queries
- Implement explain API for query debugging

## Impact

- Affected specs: `advanced-search`
- Affected code: Extends `lexum-core/src/query/`:
  - `fuzzy.rs` - Fuzzy queries
  - `phrase.rs` - Phrase queries
  - `wildcard.rs` - Wildcard queries
  - `highlight.rs` - Result highlighting
  - `suggest.rs` - Suggestions
- Dependencies: Already available in Tantivy
- Performance: Should not degrade existing search performance

