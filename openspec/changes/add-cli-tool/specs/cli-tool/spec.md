## ADDED Requirements

### Requirement: Server Management
The CLI SHALL support starting and stopping Lexum server.

#### Scenario: Start server
- **WHEN** user runs `lexum serve`
- **THEN** server starts on configured port
- **AND** logs are output to terminal

#### Scenario: Validate configuration
- **WHEN** user runs `lexum config validate`
- **THEN** configuration is checked for errors
- **AND** validation results are displayed

### Requirement: Index Management
The CLI SHALL support all index operations.

#### Scenario: Create index
- **WHEN** user runs `lexum index create my_index`
- **THEN** index is created
- **AND** success message is displayed

#### Scenario: List indices
- **WHEN** user runs `lexum index list`
- **THEN** all indices are displayed in table format

### Requirement: Document Operations
The CLI SHALL support document CRUD operations.

#### Scenario: Index document from file
- **WHEN** user runs `lexum doc index my_index < doc.json`
- **THEN** document is indexed
- **AND** document ID is displayed

### Requirement: Query Execution
The CLI SHALL support executing queries from command line.

#### Scenario: Execute search query
- **WHEN** user runs `lexum query my_index "search terms"`
- **THEN** search is executed
- **AND** results are displayed

#### Scenario: Execute LQL query
- **WHEN** user runs `lexum lql "FROM users | LIMIT 10"`
- **THEN** LQL query is executed
- **AND** results are formatted and displayed

### Requirement: Output Formatting
The CLI SHALL support multiple output formats.

#### Scenario: JSON output
- **WHEN** user adds `--format json` flag
- **THEN** output is valid JSON

#### Scenario: Table output
- **WHEN** user adds `--format table` flag
- **THEN** output is formatted as ASCII table

