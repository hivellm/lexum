## ADDED Requirements

### Requirement: Terms Aggregation
The system SHALL support terms aggregation to find top values.

#### Scenario: Top terms
- **WHEN** user aggregates by status field with size 10
- **THEN** top 10 status values are returned with doc counts

### Requirement: Stats Aggregation
The system SHALL compute statistical aggregations.

#### Scenario: Compute stats
- **WHEN** user requests stats on price field
- **THEN** min, max, avg, sum, count are returned

### Requirement: Histogram Aggregation
The system SHALL create numeric histograms.

#### Scenario: Numeric histogram
- **WHEN** user creates histogram on price with interval 100
- **THEN** prices are grouped into 100-unit buckets

### Requirement: Date Histogram
The system SHALL create time-based histograms.

#### Scenario: Hourly histogram
- **WHEN** user creates date histogram with 1h interval
- **THEN** documents are grouped by hour

### Requirement: Percentile Aggregation
The system SHALL compute percentiles for numeric fields.

#### Scenario: Calculate percentiles
- **WHEN** user requests 50th, 95th, 99th percentiles
- **THEN** accurate percentile values are computed

### Requirement: Cardinality Aggregation
The system SHALL count unique values efficiently.

#### Scenario: Unique count
- **WHEN** user requests cardinality of user_id field
- **THEN** approximate unique count is returned

### Requirement: Nested Aggregations
The system SHALL support nesting aggregations.

#### Scenario: Nested aggregation
- **WHEN** user nests stats inside terms aggregation
- **THEN** stats are computed for each term bucket

### Requirement: Distributed Aggregations
The system SHALL merge aggregations from multiple shards.

#### Scenario: Distributed terms
- **WHEN** terms aggregation runs on 6-shard index
- **THEN** results from all shards are merged correctly

