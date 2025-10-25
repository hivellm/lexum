# Telemetry and Observability

Complete guide for monitoring, metrics, tracing, and logging in Lexum.

## Overview

Lexum provides comprehensive observability through:

- **Metrics**: Prometheus-compatible metrics
- **Tracing**: Distributed tracing with OpenTelemetry
- **Logging**: Structured JSON logging
- **Health Checks**: Liveness and readiness probes
- **Profiling**: CPU and memory profiling

## Architecture

```
┌──────────────┐
│   Lexum      │
│   Nodes      │
└──────┬───────┘
       │
       ├─────────────┐
       │             │
       ▼             ▼
┌─────────────┐ ┌─────────────┐
│  Metrics    │ │   Traces    │
│(Prometheus) │ │  (Jaeger)   │
└──────┬──────┘ └──────┬──────┘
       │               │
       ▼               ▼
┌─────────────────────────┐
│    Grafana Dashboards    │
└─────────────────────────┘
       │
       ▼
┌─────────────────────────┐
│  Logs (Loki/ELK)        │
└─────────────────────────┘
```

## Metrics

### Configuration

```yaml
# config.yml
telemetry:
  metrics:
    enabled: true
    endpoint: /metrics
    interval: 15s
    exporter: prometheus
```

### Prometheus Endpoint

```bash
curl http://localhost:9200/_metrics
```

### Key Metrics

#### Request Metrics

```promql
# Request rate
rate(lexum_http_requests_total[5m])

# Request duration (p95)
histogram_quantile(0.95, 
  rate(lexum_http_request_duration_seconds_bucket[5m]))

# Request errors
rate(lexum_http_requests_total{status=~"5.."}[5m])
```

**Metrics:**
- `lexum_http_requests_total`: Total HTTP requests
- `lexum_http_request_duration_seconds`: Request duration histogram
- `lexum_http_requests_in_flight`: Current in-flight requests
- `lexum_http_request_size_bytes`: Request size
- `lexum_http_response_size_bytes`: Response size

#### Search Metrics

```promql
# Search rate
rate(lexum_search_requests_total[5m])

# Search latency (p99)
histogram_quantile(0.99,
  rate(lexum_search_duration_seconds_bucket[5m]))

# Slow queries
rate(lexum_slow_queries_total[5m])
```

**Metrics:**
- `lexum_search_requests_total`: Total search requests
- `lexum_search_duration_seconds`: Search duration histogram
- `lexum_search_hits_total`: Number of hits returned
- `lexum_search_cache_hits_total`: Cache hit count
- `lexum_search_cache_misses_total`: Cache miss count
- `lexum_slow_queries_total`: Slow query count

#### Indexing Metrics

```promql
# Indexing rate
rate(lexum_index_documents_total[5m])

# Indexing throughput (docs/sec)
rate(lexum_index_documents_total[1m])

# Index size growth
deriv(lexum_index_size_bytes[10m])
```

**Metrics:**
- `lexum_index_documents_total`: Total documents indexed
- `lexum_index_operations_total`: Index operations (create, update, delete)
- `lexum_index_duration_seconds`: Indexing duration
- `lexum_index_size_bytes`: Index size in bytes
- `lexum_index_segments_total`: Number of segments

#### Cluster Metrics

```promql
# Node count
lexum_cluster_nodes_total

# Shard health
lexum_cluster_shards_active / lexum_cluster_shards_total

# Unassigned shards
lexum_cluster_shards_unassigned
```

**Metrics:**
- `lexum_cluster_nodes_total`: Total nodes in cluster
- `lexum_cluster_shards_total`: Total shards
- `lexum_cluster_shards_active`: Active shards
- `lexum_cluster_shards_unassigned`: Unassigned shards
- `lexum_cluster_shards_relocating`: Relocating shards
- `lexum_cluster_health_status`: Cluster health (0=red, 1=yellow, 2=green)

#### System Metrics

```promql
# CPU usage
rate(lexum_process_cpu_seconds_total[5m])

# Memory usage
lexum_process_memory_bytes

# Disk usage
lexum_disk_usage_bytes / lexum_disk_total_bytes
```

**Metrics:**
- `lexum_process_cpu_seconds_total`: CPU time
- `lexum_process_memory_bytes`: Memory usage
- `lexum_process_threads_total`: Thread count
- `lexum_disk_usage_bytes`: Disk usage
- `lexum_disk_total_bytes`: Total disk space
- `lexum_disk_io_ops_total`: Disk I/O operations

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'lexum'
    static_configs:
      - targets:
        - 'lexum-node-1:9200'
        - 'lexum-node-2:9200'
        - 'lexum-node-3:9200'
    metrics_path: /_metrics
    scrape_interval: 15s
```

## Distributed Tracing

### Configuration

```yaml
# config.yml
telemetry:
  tracing:
    enabled: true
    exporter: otlp
    endpoint: http://jaeger:4317
    sample_rate: 0.1  # Sample 10% of requests
```

### OpenTelemetry Collector

```yaml
# otel-collector-config.yml
receivers:
  otlp:
    protocols:
      grpc:
        endpoint: 0.0.0.0:4317
      http:
        endpoint: 0.0.0.0:4318

processors:
  batch:
    timeout: 1s
    send_batch_size: 1024

exporters:
  jaeger:
    endpoint: jaeger:14250
    tls:
      insecure: true

service:
  pipelines:
    traces:
      receivers: [otlp]
      processors: [batch]
      exporters: [jaeger]
```

### Trace Structure

```
Request Trace
├── HTTP Handler
│   ├── Authentication
│   ├── Authorization
│   └── Request Parsing
├── Query Planning
│   ├── LQL Parsing
│   ├── Query Optimization
│   └── Shard Selection
├── Shard Execution (parallel)
│   ├── Shard 0 Search
│   │   ├── Index Access
│   │   └── Result Collection
│   ├── Shard 1 Search
│   └── Shard 2 Search
├── Result Merging
│   ├── Score Aggregation
│   └── Sorting
└── Response Serialization
```

### Viewing Traces

Access Jaeger UI:
```
http://localhost:16686
```

## Logging

### Configuration

```yaml
# config.yml
logging:
  level: info  # trace, debug, info, warn, error
  format: json
  outputs:
    - stdout
    - file:
        path: /var/log/lexum/lexum.log
        max_size: 100mb
        max_backups: 10
        max_age: 30
```

### Log Levels

- `TRACE`: Very detailed debugging
- `DEBUG`: Debugging information
- `INFO`: Informational messages
- `WARN`: Warning messages
- `ERROR`: Error messages

### Log Format

```json
{
  "timestamp": "2024-10-25T10:00:00.123Z",
  "level": "INFO",
  "target": "lexum::search",
  "message": "Search query executed",
  "fields": {
    "query_id": "abc123",
    "index": "my_index",
    "took_ms": 42,
    "hits": 150
  },
  "span": {
    "trace_id": "a1b2c3d4e5f6",
    "span_id": "123456"
  }
}
```

### Log Aggregation

#### Loki Setup

```yaml
# docker-compose.yml
loki:
  image: grafana/loki:latest
  ports:
    - "3100:3100"
  command: -config.file=/etc/loki/config.yml

promtail:
  image: grafana/promtail:latest
  volumes:
    - /var/log/lexum:/var/log/lexum:ro
    - ./promtail-config.yml:/etc/promtail/config.yml
  command: -config.file=/etc/promtail/config.yml
```

```yaml
# promtail-config.yml
server:
  http_listen_port: 9080

positions:
  filename: /tmp/positions.yaml

clients:
  - url: http://loki:3100/loki/api/v1/push

scrape_configs:
  - job_name: lexum
    static_configs:
      - targets:
          - localhost
        labels:
          job: lexum
          __path__: /var/log/lexum/*.log
    pipeline_stages:
      - json:
          expressions:
            level: level
            timestamp: timestamp
            message: message
      - labels:
          level:
      - timestamp:
          source: timestamp
          format: RFC3339Nano
```

### Query Logs

```bash
# Loki query
{job="lexum"} |= "error"

# Filter by level
{job="lexum", level="ERROR"}

# Time range
{job="lexum"} |= "error" [5m]
```

## Health Checks

### Liveness Probe

```bash
curl http://localhost:9200/_health
```

**Response:**
```json
{
  "status": "healthy",
  "timestamp": "2024-10-25T10:00:00Z"
}
```

**Status Codes:**
- `200`: Healthy
- `503`: Unhealthy

### Readiness Probe

```bash
curl http://localhost:9200/_ready
```

**Response:**
```json
{
  "ready": true,
  "checks": {
    "cluster": "ok",
    "disk_space": "ok",
    "memory": "ok"
  }
}
```

### Kubernetes Probes

```yaml
livenessProbe:
  httpGet:
    path: /_health
    port: 9200
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /_ready
    port: 9200
  initialDelaySeconds: 15
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3
```

## Performance Profiling

### CPU Profiling

```bash
# Enable profiling
curl -X POST http://localhost:9200/_profile/cpu/start

# Run workload...

# Stop and download profile
curl http://localhost:9200/_profile/cpu/stop -o cpu.prof

# Analyze with pprof
go tool pprof cpu.prof
```

### Memory Profiling

```bash
# Heap snapshot
curl http://localhost:9200/_profile/heap -o heap.prof

# Analyze
go tool pprof heap.prof
```

### Flamegraph

```bash
# Generate flamegraph
curl http://localhost:9200/_profile/flamegraph -o flamegraph.svg

# View in browser
open flamegraph.svg
```

## Grafana Dashboards

### Installation

```bash
# Add dashboard
curl -X POST http://admin:admin@localhost:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @dashboards/lexum-overview.json
```

### Overview Dashboard

**Panels:**
1. Cluster Health
2. Request Rate (QPS)
3. Request Latency (p50, p95, p99)
4. Error Rate
5. Index Rate
6. Search Rate
7. CPU Usage
8. Memory Usage
9. Disk Usage
10. Network I/O

### Search Performance Dashboard

**Panels:**
1. Search Latency Heatmap
2. Slow Queries
3. Cache Hit Rate
4. Query Types Distribution
5. Top Queries
6. Query Errors

### System Dashboard

**Panels:**
1. CPU per Node
2. Memory per Node
3. Disk I/O
4. Network I/O
5. Thread Count
6. GC Metrics

### Example Query

```promql
# Request rate by status
sum(rate(lexum_http_requests_total[5m])) by (status)

# p95 latency
histogram_quantile(0.95,
  sum(rate(lexum_http_request_duration_seconds_bucket[5m])) by (le))

# Error rate percentage
sum(rate(lexum_http_requests_total{status=~"5.."}[5m])) /
sum(rate(lexum_http_requests_total[5m])) * 100
```

## Alerting

### Prometheus Alerts

```yaml
# alerts.yml
groups:
  - name: lexum
    interval: 30s
    rules:
      # High error rate
      - alert: HighErrorRate
        expr: |
          sum(rate(lexum_http_requests_total{status=~"5.."}[5m])) /
          sum(rate(lexum_http_requests_total[5m])) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: High error rate detected
          description: Error rate is {{ $value | humanizePercentage }}

      # High latency
      - alert: HighLatency
        expr: |
          histogram_quantile(0.95,
            rate(lexum_search_duration_seconds_bucket[5m])) > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: High search latency
          description: p95 latency is {{ $value }}s

      # Cluster unhealthy
      - alert: ClusterUnhealthy
        expr: lexum_cluster_health_status < 2
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: Cluster is not green
          description: Cluster status is {{ $value }}

      # Disk space low
      - alert: LowDiskSpace
        expr: |
          (lexum_disk_usage_bytes / lexum_disk_total_bytes) > 0.85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: Low disk space
          description: Disk usage is {{ $value | humanizePercentage }}

      # Unassigned shards
      - alert: UnassignedShards
        expr: lexum_cluster_shards_unassigned > 0
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: Unassigned shards detected
          description: {{ $value }} shards are unassigned
```

### Alertmanager Configuration

```yaml
# alertmanager.yml
global:
  resolve_timeout: 5m

route:
  group_by: ['alertname', 'cluster']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 12h
  receiver: 'default'
  routes:
    - match:
        severity: critical
      receiver: 'pagerduty'
    - match:
        severity: warning
      receiver: 'slack'

receivers:
  - name: 'default'
    email_configs:
      - to: 'alerts@example.com'

  - name: 'slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/...'
        channel: '#alerts'
        title: 'Lexum Alert'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'

  - name: 'pagerduty'
    pagerduty_configs:
      - service_key: 'your-pagerduty-key'
```

## Slow Query Logging

### Configuration

```yaml
# config.yml
logging:
  slow_query:
    enabled: true
    threshold_ms: 1000
    log_level: warn
```

### Slow Query Log Format

```json
{
  "timestamp": "2024-10-25T10:00:00.123Z",
  "level": "WARN",
  "message": "Slow query detected",
  "query_id": "abc123",
  "index": "my_index",
  "query": "FROM my_index | WHERE ...",
  "took_ms": 1523,
  "hits": 15000,
  "shards": {
    "total": 3,
    "successful": 3,
    "failed": 0
  }
}
```

## Query Analytics

### Track Popular Queries

```bash
# Top queries by count
curl http://localhost:9200/_analytics/queries/top?limit=10

# Slow queries
curl http://localhost:9200/_analytics/queries/slow?threshold=1000

# Failed queries
curl http://localhost:9200/_analytics/queries/failed
```

## Deployment Monitoring

### Docker Compose

```yaml
# docker-compose-monitoring.yml
version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus

  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    volumes:
      - grafana-data:/var/lib/grafana
      - ./dashboards:/etc/grafana/provisioning/dashboards

  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"
      - "4317:4317"
      - "4318:4318"

  loki:
    image: grafana/loki:latest
    ports:
      - "3100:3100"
    volumes:
      - ./loki-config.yml:/etc/loki/config.yml
      - loki-data:/loki

  promtail:
    image: grafana/promtail:latest
    volumes:
      - /var/log/lexum:/var/log/lexum:ro
      - ./promtail-config.yml:/etc/promtail/config.yml

volumes:
  prometheus-data:
  grafana-data:
  loki-data:
```

### Start Monitoring Stack

```bash
docker-compose -f docker-compose-monitoring.yml up -d
```

**Access:**
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3000
- Jaeger: http://localhost:16686

## Best Practices

1. **Monitor Key Metrics**: Focus on request rate, latency, errors, saturation
2. **Set Alerts**: Alert on SLO violations, not symptoms
3. **Use Distributed Tracing**: For debugging complex queries
4. **Structure Logs**: Use JSON format for easy parsing
5. **Sample Traces**: Don't trace every request in production
6. **Aggregate Logs**: Use Loki or ELK for centralized logging
7. **Regular Review**: Review dashboards and alerts weekly
8. **Correlate Data**: Link metrics, traces, and logs
9. **Monitor Costs**: Track resource usage and costs
10. **Document Runbooks**: Create runbooks for common alerts

## Troubleshooting

### High Latency

1. Check slow query logs
2. Review query execution plans
3. Check cache hit rates
4. Monitor disk I/O
5. Review shard distribution

### High Error Rate

1. Check application logs
2. Review error types
3. Check cluster health
4. Monitor node resources
5. Review recent changes

### Memory Issues

1. Check heap usage
2. Review cache sizes
3. Monitor GC activity
4. Check for memory leaks
5. Review indexing rate

## See Also

- [Architecture](./ARCHITECTURE.md)
- [Deployment](./DEPLOYMENT.md)
- [API Reference](./API_REFERENCE.md)

