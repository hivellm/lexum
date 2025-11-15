## ADDED Requirements

### Requirement: YAML Configuration
The system SHALL support YAML configuration files.

#### Scenario: Load configuration from file
- **WHEN** server starts with config file path
- **THEN** configuration is loaded from YAML file
- **AND** all settings are applied

#### Scenario: Invalid YAML syntax
- **WHEN** config file has invalid YAML
- **THEN** system reports clear error message
- **AND** indicates line number of error

### Requirement: Environment Variables
The system SHALL support environment variable overrides for all configuration.

#### Scenario: Override with environment variable
- **WHEN** LEXUM_HTTP_PORT env var is set
- **THEN** HTTP port from env var overrides config file

### Requirement: Configuration Validation
The system SHALL validate all configuration values before startup.

#### Scenario: Invalid port number
- **WHEN** HTTP port is set to 0 or > 65535
- **THEN** validation fails with clear error

### Requirement: Structured Logging
The system SHALL output structured JSON logs.

#### Scenario: Log entry format
- **WHEN** any log is emitted
- **THEN** log is valid JSON with timestamp, level, target, message, fields

### Requirement: Log Levels
The system SHALL support configurable log levels.

#### Scenario: Filter by log level
- **WHEN** log level is set to WARN
- **THEN** only WARN and ERROR logs are emitted

### Requirement: Correlation IDs
The system SHALL propagate correlation IDs through all operations.

#### Scenario: Request correlation
- **WHEN** HTTP request is received
- **THEN** all logs for that request include same correlation ID

