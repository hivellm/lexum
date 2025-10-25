## ADDED Requirements

### Requirement: Docker Image
The system SHALL provide optimized Docker image.

#### Scenario: Build Docker image
- **WHEN** building Docker image
- **THEN** image size is less than 100MB
- **AND** build completes in less than 5 minutes

#### Scenario: Run container
- **WHEN** running Lexum container
- **THEN** server starts successfully
- **AND** health check passes

### Requirement: Kubernetes StatefulSet
The system SHALL deploy as StatefulSet in Kubernetes.

#### Scenario: Deploy StatefulSet
- **WHEN** StatefulSet is deployed
- **THEN** pods are created with stable network IDs
- **AND** each pod has persistent storage

### Requirement: Helm Chart
The system SHALL provide Helm chart for easy deployment.

#### Scenario: Helm install
- **WHEN** user runs helm install
- **THEN** all resources are created
- **AND** cluster is operational

### Requirement: Health Probes
The system SHALL support Kubernetes health probes.

#### Scenario: Liveness probe
- **WHEN** pod is running
- **THEN** liveness probe succeeds
- **AND** pod is not restarted

#### Scenario: Readiness probe  
- **WHEN** pod is starting
- **THEN** readiness probe fails initially
- **AND** succeeds once pod is ready
- **AND** traffic is only routed to ready pods

### Requirement: Horizontal Scaling
The system SHALL support horizontal pod autoscaling.

#### Scenario: Scale up
- **WHEN** CPU usage exceeds 80%
- **THEN** HPA adds more pods
- **AND** cluster rebalances shards

### Requirement: Persistent Storage
The system SHALL use PersistentVolumes for data.

#### Scenario: Data persistence
- **WHEN** pod is restarted
- **THEN** data is retained in PersistentVolume
- **AND** index is immediately available

