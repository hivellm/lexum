## MODIFIED Requirements

### Requirement: Advanced Pattern Matching in Templates
Index template pattern matching SHALL support regex patterns in addition to simple wildcards.

#### Scenario: Regex pattern matching
- **WHEN** a template pattern contains regex syntax
- **THEN** the pattern is matched using regex rules
- **AND** index names matching the regex are selected
- **AND** complex pattern matching is supported

#### Scenario: Backward compatibility with wildcards
- **WHEN** a template pattern contains simple wildcards (*, ?)
- **THEN** the pattern continues to work as before
- **AND** backward compatibility is maintained

