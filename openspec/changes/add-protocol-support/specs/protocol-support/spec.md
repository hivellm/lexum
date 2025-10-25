## ADDED Requirements

### Requirement: StreamableHTTP
The system SHALL support streaming HTTP responses via Server-Sent Events.

#### Scenario: Stream search results
- **WHEN** client requests GET /{index}/_search/stream with Accept: text/event-stream
- **THEN** results are streamed as SSE events
- **AND** client receives results incrementally

#### Scenario: Backpressure handling
- **WHEN** client is slow to consume stream
- **THEN** server applies backpressure
- **AND** memory usage remains bounded

### Requirement: MCP Protocol
The system SHALL support Model Context Protocol for AI/LLM integration.

#### Scenario: MCP search operation
- **WHEN** client sends MCP search request
- **THEN** semantic search is performed
- **AND** results are returned in MCP format

#### Scenario: MCP retrieve operation
- **WHEN** client requests document retrieval via MCP
- **THEN** documents are retrieved and returned

### Requirement: UMICP Protocol
The system SHALL support UMICP binary protocol over WebSocket.

#### Scenario: UMICP connection
- **WHEN** client connects to ws://server/_umicp
- **THEN** binary protocol connection is established
- **AND** messages use bincode serialization

#### Scenario: UMICP bulk operation
- **WHEN** client sends bulk request via UMICP
- **THEN** operations are processed efficiently
- **AND** binary format reduces overhead

### Requirement: WebSocket Real-time Updates
The system SHALL support WebSocket for real-time notifications.

#### Scenario: Subscribe to index changes
- **WHEN** client subscribes to index via WebSocket
- **THEN** client receives notifications on document changes

#### Scenario: Query subscription
- **WHEN** client subscribes to query results
- **THEN** client receives updates as matching documents change

### Requirement: Protocol Performance
The system SHALL maintain low overhead for all protocols.

#### Scenario: MCP overhead
- **WHEN** comparing MCP to REST API
- **THEN** protocol overhead is less than 5ms

#### Scenario: UMICP binary efficiency
- **WHEN** using UMICP vs HTTP JSON
- **THEN** payload size is at least 30% smaller

