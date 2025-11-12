## MODIFIED Requirements

### Requirement: Efficient Search Sorting
Search executor SHALL use efficient Tantivy-based sorting when available instead of in-memory sorting.

#### Scenario: Search with sorting using Tantivy
- **WHEN** a search query includes sorting requirements
- **AND** Tantivy supports the requested sort field
- **THEN** sorting is performed by Tantivy during search execution
- **AND** results are returned in sorted order
- **AND** performance is improved compared to in-memory sorting

#### Scenario: Fallback to in-memory sorting
- **WHEN** a search query includes sorting requirements
- **AND** Tantivy does not support the requested sort field
- **THEN** sorting falls back to in-memory sorting
- **AND** results are still returned in sorted order

