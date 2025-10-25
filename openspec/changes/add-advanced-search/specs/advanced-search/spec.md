## ADDED Requirements

### Requirement: Fuzzy Search
The system SHALL support fuzzy matching with configurable edit distance.

#### Scenario: Fuzzy match
- **WHEN** user searches for "serch" with fuzziness=2
- **THEN** documents containing "search" are returned

#### Scenario: Auto fuzziness
- **WHEN** user uses fuzziness=AUTO
- **THEN** edit distance is calculated based on term length

### Requirement: Phrase Queries
The system SHALL support exact phrase matching.

#### Scenario: Exact phrase
- **WHEN** user searches for phrase "machine learning"
- **THEN** only documents with exact phrase are returned

#### Scenario: Phrase with slop
- **WHEN** user searches phrase with slop=2
- **THEN** terms can be 2 positions apart

### Requirement: Wildcard Queries
The system SHALL support wildcard matching.

#### Scenario: Prefix wildcard
- **WHEN** user searches for "rust*"
- **THEN** terms starting with "rust" match

#### Scenario: Suffix wildcard
- **WHEN** user searches for "*ing"
- **THEN** terms ending with "ing" match

### Requirement: Regex Queries
The system SHALL support regular expression queries.

#### Scenario: Regex match
- **WHEN** user queries with regex "[A-Z][a-z]+"
- **THEN** matching terms are found

### Requirement: Field Boosting
The system SHALL support field-level relevance boosting.

#### Scenario: Boosted field
- **WHEN** user boosts title field by 3x
- **THEN** matches in title score 3x higher

### Requirement: Result Highlighting
The system SHALL highlight matching terms in results.

#### Scenario: Highlight matches
- **WHEN** user requests highlighting
- **THEN** matching terms are wrapped in HTML tags

### Requirement: Search Suggestions
The system SHALL provide search suggestions.

#### Scenario: Autocomplete
- **WHEN** user types partial query
- **THEN** completion suggestions are returned

### Requirement: More-Like-This
The system SHALL find similar documents.

#### Scenario: Find similar documents
- **WHEN** user requests documents like document ID
- **THEN** semantically similar documents are returned

### Requirement: Query Explanation
The system SHALL explain query scoring.

#### Scenario: Explain score
- **WHEN** user requests explanation for document score
- **THEN** detailed score calculation is returned

