## ADDED Requirements

### Requirement: TLS Encryption
The system SHALL support TLS 1.3 for all HTTP connections.

#### Scenario: TLS connection
- **WHEN** client connects with HTTPS
- **THEN** connection is encrypted with TLS 1.3
- **AND** certificate is validated

#### Scenario: mTLS inter-node
- **WHEN** nodes communicate
- **THEN** mutual TLS authentication is used
- **AND** only trusted certificates are accepted

### Requirement: API Key Authentication
The system SHALL authenticate requests via API keys.

#### Scenario: Valid API key
- **WHEN** request includes valid X-API-Key header
- **THEN** request is authenticated
- **AND** user identity is established

#### Scenario: Missing API key
- **WHEN** request lacks API key and auth is required
- **THEN** server returns 401 Unauthorized

#### Scenario: Invalid API key
- **WHEN** request has invalid or expired API key
- **THEN** server returns 401 Unauthorized

### Requirement: Role-Based Access Control
The system SHALL enforce role-based permissions on all operations.

#### Scenario: User with read permission
- **WHEN** user with read-only role attempts search
- **THEN** operation is allowed

#### Scenario: User without write permission
- **WHEN** user without write role attempts to index document
- **THEN** server returns 403 Forbidden

#### Scenario: Admin operations
- **WHEN** non-admin user attempts admin operation
- **THEN** server returns 403 Forbidden

### Requirement: Document-Level Security
The system SHALL filter search results based on document permissions.

#### Scenario: Restricted document filtering
- **WHEN** user searches index with restricted documents
- **THEN** only documents user has access to are returned
- **AND** total count reflects accessible documents only

### Requirement: Field-Level Security
The system SHALL mask fields based on user permissions.

#### Scenario: Sensitive field masking
- **WHEN** user without permission views document with sensitive fields
- **THEN** sensitive fields are masked or omitted

### Requirement: Audit Logging
The system SHALL log all authentication and authorization events.

#### Scenario: Authentication logged
- **WHEN** user authenticates
- **THEN** authentication attempt is logged with outcome

#### Scenario: Authorization logged
- **WHEN** permission check occurs
- **THEN** decision is logged with user, resource, and outcome

#### Scenario: Data access logged
- **WHEN** user accesses documents
- **THEN** access is logged with user, index, and document IDs

### Requirement: Rate Limiting
The system SHALL enforce rate limits per user.

#### Scenario: Within rate limit
- **WHEN** user makes requests within limit
- **THEN** all requests are processed

#### Scenario: Exceeded rate limit
- **WHEN** user exceeds rate limit
- **THEN** server returns 429 Too Many Requests
- **AND** includes Retry-After header

