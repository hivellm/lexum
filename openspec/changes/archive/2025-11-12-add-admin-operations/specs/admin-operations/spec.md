## ADDED Requirements

### Requirement: Snapshot Creation
The system SHALL support creating snapshots of indices.

#### Scenario: Create snapshot
- **WHEN** admin creates snapshot of index
- **THEN** snapshot is created in repository
- **AND** snapshot includes all index data

#### Scenario: Incremental snapshot
- **WHEN** creating second snapshot
- **THEN** only changed data is stored
- **AND** snapshot completes faster

### Requirement: Snapshot Restore
The system SHALL support restoring indices from snapshots.

#### Scenario: Restore snapshot
- **WHEN** admin restores snapshot
- **THEN** index is recreated from snapshot
- **AND** all documents are restored

### Requirement: Index Templates
The system SHALL support index templates for automatic configuration.

#### Scenario: Apply template
- **WHEN** new index matches template pattern
- **THEN** template settings are automatically applied

### Requirement: Index Aliases
The system SHALL support index aliases.

#### Scenario: Create alias
- **WHEN** admin creates alias pointing to index
- **THEN** operations on alias target the index

#### Scenario: Atomic alias switch
- **WHEN** admin switches alias from old to new index
- **THEN** operation is atomic
- **AND** no requests are lost

### Requirement: Reindexing
The system SHALL support reindexing from one index to another.

#### Scenario: Reindex operation
- **WHEN** admin reindexes from source to destination
- **THEN** all documents are copied
- **AND** progress is trackable

### Requirement: Cluster Settings
The system SHALL support dynamic cluster settings.

#### Scenario: Update cluster setting
- **WHEN** admin updates cluster setting
- **THEN** setting is applied across all nodes
- **AND** takes effect immediately

### Requirement: Task Management
The system SHALL track and manage long-running tasks.

#### Scenario: List tasks
- **WHEN** admin requests task list
- **THEN** all running tasks are shown with progress

#### Scenario: Cancel task
- **WHEN** admin cancels running task
- **THEN** task is stopped gracefully

