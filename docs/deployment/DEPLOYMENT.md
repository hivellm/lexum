# Deployment Guide

Complete guide for deploying Lexum in production using Docker, Kubernetes, and bare metal.

## Quick Start

### Docker

```bash
# Build image
docker build -t lexum:latest .

# Run single node
docker run -d \
  --name lexum-server \
  -p 9200:9200 \
  -v lexum-data:/data \
  -v lexum-snapshots:/snapshots \
  -e LEXUM_NETWORK_HOST=0.0.0.0 \
  -e LEXUM_NETWORK_HTTP_PORT=9200 \
  lexum:latest

# Check health
curl http://localhost:9200/_cluster/health

# View logs
docker logs lexum-server -f
```

See [Dockerfile](../Dockerfile) for detailed usage instructions.

### Docker Compose

**Single node:**

```bash
docker-compose -f docker-compose.yml up -d
```

**Multi-node cluster:**

```bash
docker-compose -f docker-compose.cluster.yml up -d
```

See [docker-compose.yml](../docker-compose.yml) and [docker-compose.cluster.yml](../docker-compose.cluster.yml) for configuration details.

### Kubernetes (Helm)

```bash
# Install from local chart
helm install lexum ./helm/lexum \
  --namespace lexum \
  --create-namespace

# Install with custom values
helm install lexum ./helm/lexum \
  --namespace lexum \
  --create-namespace \
  -f my-values.yaml

# Upgrade
helm upgrade lexum ./helm/lexum \
  --namespace lexum \
  -f my-values.yaml

# Check status
kubectl get pods -n lexum
kubectl get svc -n lexum
```

See [helm/lexum/README.md](../helm/lexum/README.md) for detailed Helm chart documentation.

## Architecture Patterns

### Single Node (Development)

```
┌─────────────┐
│   Lexum     │
│   Node      │
│  (All roles)│
└─────────────┘
```

**Use Case:** Development, testing, small datasets

**Configuration:**

```yaml
# config.yml
cluster:
  name: lexum-dev
  initial_master_nodes: [node-1]

node:
  name: node-1
  roles: [master, data, ingest]

path:
  data: /data
```

### Multi-Node Cluster (Production)

```
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Master 1 │  │ Master 2 │  │ Master 3 │
└──────────┘  └──────────┘  └──────────┘
      │              │              │
      └──────────────┼──────────────┘
                     │
    ┌────────────────┼────────────────┐
    │                │                │
┌────────┐      ┌────────┐      ┌────────┐
│ Data 1 │      │ Data 2 │      │ Data 3 │
└────────┘      └────────┘      └────────┘
```

**Use Case:** Production, high availability

**Configuration:**

Master nodes:

```yaml
# master-node.yml
cluster:
  name: lexum-prod
  initial_master_nodes: [master-1, master-2, master-3]

node:
  name: master-1
  roles: [master]
```

Data nodes:

```yaml
# data-node.yml
cluster:
  name: lexum-prod

node:
  name: data-1
  roles: [data, ingest]
```

### Dedicated Roles

```
                  ┌──────────┐
                  │   LB     │
                  └─────┬────┘
                        │
    ┌───────────────────┼───────────────────┐
    │                   │                   │
┌─────────┐      ┌──────────┐      ┌───────────┐
│Coordin 1│      │Coordin 2│      │Coordin 3  │
│ (Query) │      │ (Query)  │      │  (Query)  │
└────┬────┘      └─────┬────┘      └─────┬─────┘
     │                 │                  │
     └─────────────────┼──────────────────┘
                       │
       ┌───────────────┼───────────────┐
       │               │               │
   ┌───────┐       ┌───────┐      ┌───────┐
   │Data 1 │       │Data 2 │      │Data 3 │
   └───────┘       └───────┘      └───────┘
       │               │               │
   ┌───────┐       ┌───────┐      ┌───────┐
   │Data 4 │       │Data 5 │      │Data 6 │
   └───────┘       └───────┘      └───────┘
```

**Node Roles:**

- **Master**: Cluster management, shard allocation
- **Data**: Store data, execute queries
- **Ingest**: Document preprocessing
- **Coordinator**: Route requests, merge results

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.85-bookworm as builder

WORKDIR /build
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/lexum /usr/local/bin/
COPY config.example.yml /etc/lexum/config.yml

RUN useradd -r -s /bin/false lexum && \
    mkdir -p /data /var/log/lexum && \
    chown -R lexum:lexum /data /var/log/lexum

USER lexum
EXPOSE 9200 9300

VOLUME ["/data"]

ENTRYPOINT ["lexum"]
CMD ["serve", "--config", "/etc/lexum/config.yml"]
```

### Build and Push

```bash
# Build
docker build -t lexum/lexum:0.1.0 .
docker tag lexum/lexum:0.1.0 lexum/lexum:latest

# Push to registry
docker push lexum/lexum:0.1.0
docker push lexum/lexum:latest
```

### Docker Compose - Production

```yaml
version: "3.8"

services:
  # Master nodes
  master-1:
    image: lexum/lexum:latest
    container_name: lexum-master-1
    hostname: master-1
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=master-1
      - NODE_ROLES=master
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
      - INITIAL_MASTER_NODES=master-1,master-2,master-3
    volumes:
      - master-1-data:/data
    networks:
      - lexum

  master-2:
    image: lexum/lexum:latest
    container_name: lexum-master-2
    hostname: master-2
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=master-2
      - NODE_ROLES=master
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
      - INITIAL_MASTER_NODES=master-1,master-2,master-3
    volumes:
      - master-2-data:/data
    networks:
      - lexum

  master-3:
    image: lexum/lexum:latest
    container_name: lexum-master-3
    hostname: master-3
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=master-3
      - NODE_ROLES=master
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
      - INITIAL_MASTER_NODES=master-1,master-2,master-3
    volumes:
      - master-3-data:/data
    networks:
      - lexum

  # Data nodes
  data-1:
    image: lexum/lexum:latest
    container_name: lexum-data-1
    hostname: data-1
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=data-1
      - NODE_ROLES=data,ingest
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
    volumes:
      - data-1-data:/data
    networks:
      - lexum
    deploy:
      resources:
        limits:
          memory: 4G
        reservations:
          memory: 2G

  data-2:
    image: lexum/lexum:latest
    container_name: lexum-data-2
    hostname: data-2
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=data-2
      - NODE_ROLES=data,ingest
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
    volumes:
      - data-2-data:/data
    networks:
      - lexum
    deploy:
      resources:
        limits:
          memory: 4G
        reservations:
          memory: 2G

  data-3:
    image: lexum/lexum:latest
    container_name: lexum-data-3
    hostname: data-3
    environment:
      - CLUSTER_NAME=lexum-prod
      - NODE_NAME=data-3
      - NODE_ROLES=data,ingest
      - DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
    volumes:
      - data-3-data:/data
    networks:
      - lexum
    deploy:
      resources:
        limits:
          memory: 4G
        reservations:
          memory: 2G

  # Load balancer
  nginx:
    image: nginx:alpine
    ports:
      - "9200:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
    depends_on:
      - data-1
      - data-2
      - data-3
    networks:
      - lexum

volumes:
  master-1-data:
  master-2-data:
  master-3-data:
  data-1-data:
  data-2-data:
  data-3-data:

networks:
  lexum:
    driver: bridge
```

### Nginx Load Balancer Config

```nginx
# nginx.conf
events {
    worker_connections 1024;
}

http {
    upstream lexum_cluster {
        least_conn;
        server data-1:9200 max_fails=3 fail_timeout=30s;
        server data-2:9200 max_fails=3 fail_timeout=30s;
        server data-3:9200 max_fails=3 fail_timeout=30s;
    }

    server {
        listen 80;

        location / {
            proxy_pass http://lexum_cluster;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;

            # Timeouts for long-running queries
            proxy_connect_timeout 60s;
            proxy_send_timeout 300s;
            proxy_read_timeout 300s;
        }

        location /_health {
            access_log off;
            proxy_pass http://lexum_cluster/_health;
        }
    }
}
```

## Kubernetes Deployment

### Using Kustomize

```bash
# Deploy with default settings
kubectl apply -k k8s/

# Customize and deploy
kubectl kustomize k8s/ | kubectl apply -f -
```

### Using Helm

See [Helm Chart](#kubernetes-helm) section above.

### Manual Deployment

All Kubernetes manifests are available in the `k8s/` directory:

- `namespace.yaml` - Namespace definition
- `configmap.yaml` - Configuration
- `secret.yaml` - Secrets template
- `statefulset.yaml` - StatefulSet for pods
- `service.yaml` - ClusterIP service
- `service-headless.yaml` - Headless service for StatefulSet
- `service-loadbalancer.yaml` - LoadBalancer service (optional)
- `ingress.yaml` - Ingress configuration
- `hpa.yaml` - Horizontal Pod Autoscaler
- `servicemonitor.yaml` - Prometheus ServiceMonitor
- `pvc.yaml` - PersistentVolumeClaim templates

See [k8s/README.md](../k8s/README.md) for detailed instructions.

### Namespace

```yaml
# namespace.yml
apiVersion: v1
kind: Namespace
metadata:
  name: lexum
```

### ConfigMap

```yaml
# configmap.yml
apiVersion: v1
kind: ConfigMap
metadata:
  name: lexum-config
  namespace: lexum
data:
  config.yml: |
    cluster:
      name: ${CLUSTER_NAME}

    node:
      name: ${NODE_NAME}
      roles: ${NODE_ROLES}

    network:
      host: 0.0.0.0
      http_port: 9200
      transport_port: 9300

    path:
      data: /data
      logs: /var/log/lexum
```

### StatefulSet - Master Nodes

```yaml
# master-statefulset.yml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: lexum-master
  namespace: lexum
spec:
  serviceName: lexum-master
  replicas: 3
  selector:
    matchLabels:
      app: lexum
      role: master
  template:
    metadata:
      labels:
        app: lexum
        role: master
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchExpressions:
                  - key: role
                    operator: In
                    values:
                      - master
              topologyKey: kubernetes.io/hostname

      containers:
        - name: lexum
          image: lexum/lexum:latest
          env:
            - name: CLUSTER_NAME
              value: "lexum-k8s"
            - name: NODE_NAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: NODE_ROLES
              value: "master"
            - name: DISCOVERY_SEED_HOSTS
              value: "lexum-master-0.lexum-master,lexum-master-1.lexum-master,lexum-master-2.lexum-master"
            - name: INITIAL_MASTER_NODES
              value: "lexum-master-0,lexum-master-1,lexum-master-2"

          ports:
            - containerPort: 9200
              name: http
            - containerPort: 9300
              name: transport

          volumeMounts:
            - name: data
              mountPath: /data
            - name: config
              mountPath: /etc/lexum

          resources:
            requests:
              memory: "1Gi"
              cpu: "500m"
            limits:
              memory: "2Gi"
              cpu: "1000m"

          livenessProbe:
            httpGet:
              path: /_health
              port: 9200
            initialDelaySeconds: 30
            periodSeconds: 10

          readinessProbe:
            httpGet:
              path: /_health
              port: 9200
            initialDelaySeconds: 15
            periodSeconds: 5

      volumes:
        - name: config
          configMap:
            name: lexum-config

  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 10Gi
```

### StatefulSet - Data Nodes

```yaml
# data-statefulset.yml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: lexum-data
  namespace: lexum
spec:
  serviceName: lexum-data
  replicas: 3
  selector:
    matchLabels:
      app: lexum
      role: data
  template:
    metadata:
      labels:
        app: lexum
        role: data
    spec:
      affinity:
        podAntiAffinity:
          preferredDuringSchedulingIgnoredDuringExecution:
            - weight: 100
              podAffinityTerm:
                labelSelector:
                  matchExpressions:
                    - key: role
                      operator: In
                      values:
                        - data
                topologyKey: kubernetes.io/hostname

      containers:
        - name: lexum
          image: lexum/lexum:latest
          env:
            - name: CLUSTER_NAME
              value: "lexum-k8s"
            - name: NODE_NAME
              valueFrom:
                fieldRef:
                  fieldPath: metadata.name
            - name: NODE_ROLES
              value: "data,ingest"
            - name: DISCOVERY_SEED_HOSTS
              value: "lexum-master-0.lexum-master,lexum-master-1.lexum-master,lexum-master-2.lexum-master"

          ports:
            - containerPort: 9200
              name: http
            - containerPort: 9300
              name: transport

          volumeMounts:
            - name: data
              mountPath: /data
            - name: config
              mountPath: /etc/lexum

          resources:
            requests:
              memory: "4Gi"
              cpu: "2000m"
            limits:
              memory: "8Gi"
              cpu: "4000m"

          livenessProbe:
            httpGet:
              path: /_health
              port: 9200
            initialDelaySeconds: 60
            periodSeconds: 10

          readinessProbe:
            httpGet:
              path: /_health
              port: 9200
            initialDelaySeconds: 30
            periodSeconds: 5

      volumes:
        - name: config
          configMap:
            name: lexum-config

  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: ["ReadWriteOnce"]
        storageClassName: fast-ssd
        resources:
          requests:
            storage: 100Gi
```

### Services

```yaml
# services.yml
---
apiVersion: v1
kind: Service
metadata:
  name: lexum-master
  namespace: lexum
spec:
  clusterIP: None
  selector:
    app: lexum
    role: master
  ports:
    - name: http
      port: 9200
    - name: transport
      port: 9300

---
apiVersion: v1
kind: Service
metadata:
  name: lexum-data
  namespace: lexum
spec:
  clusterIP: None
  selector:
    app: lexum
    role: data
  ports:
    - name: http
      port: 9200
    - name: transport
      port: 9300

---
apiVersion: v1
kind: Service
metadata:
  name: lexum
  namespace: lexum
spec:
  type: LoadBalancer
  selector:
    app: lexum
    role: data
  ports:
    - name: http
      port: 9200
      targetPort: 9200
```

### Ingress

```yaml
# ingress.yml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: lexum-ingress
  namespace: lexum
  annotations:
    cert-manager.io/cluster-issuer: letsencrypt-prod
    nginx.ingress.kubernetes.io/proxy-body-size: "100m"
    nginx.ingress.kubernetes.io/proxy-read-timeout: "300"
spec:
  ingressClassName: nginx
  tls:
    - hosts:
        - search.example.com
      secretName: lexum-tls
  rules:
    - host: search.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: lexum
                port:
                  number: 9200
```

### Helm Chart

```yaml
# values.yml
cluster:
  name: lexum-prod

master:
  replicas: 3
  resources:
    requests:
      memory: 1Gi
      cpu: 500m
    limits:
      memory: 2Gi
      cpu: 1000m
  persistence:
    enabled: true
    size: 10Gi
    storageClass: fast-ssd

data:
  replicas: 3
  resources:
    requests:
      memory: 4Gi
      cpu: 2000m
    limits:
      memory: 8Gi
      cpu: 4000m
  persistence:
    enabled: true
    size: 100Gi
    storageClass: fast-ssd

ingress:
  enabled: true
  hostname: search.example.com
  tls:
    enabled: true

monitoring:
  enabled: true
  serviceMonitor:
    enabled: true
```

### Deploy with Helm

```bash
# Install
helm install lexum ./helm/lexum \
  -n lexum \
  --create-namespace \
  -f values.yml

# Upgrade
helm upgrade lexum ./helm/lexum \
  -n lexum \
  -f values.yml

# Uninstall
helm uninstall lexum -n lexum
```

## Bare Metal Deployment

### System Requirements

**Per Node:**

- CPU: 4+ cores
- RAM: 8GB+ (16GB+ recommended)
- Disk: 100GB+ SSD
- OS: Ubuntu 22.04+, RHEL 8+, Debian 11+

### Installation

```bash
# Download binary
wget https://github.com/your-org/lexum/releases/download/v0.1.0/lexum-v0.1.0-linux-x86_64.tar.gz

# Extract
tar -xzf lexum-v0.1.0-linux-x86_64.tar.gz
sudo mv lexum /usr/local/bin/
sudo chmod +x /usr/local/bin/lexum

# Create user
sudo useradd -r -s /bin/false lexum

# Create directories
sudo mkdir -p /var/lib/lexum /var/log/lexum /etc/lexum
sudo chown -R lexum:lexum /var/lib/lexum /var/log/lexum
```

### SystemD Service

```ini
# /etc/systemd/system/lexum.service
[Unit]
Description=Lexum Search Engine
After=network.target

[Service]
Type=simple
User=lexum
Group=lexum
ExecStart=/usr/local/bin/lexum serve --config /etc/lexum/config.yml
Restart=on-failure
RestartSec=5s
LimitNOFILE=65536
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

### Configuration

```yaml
# /etc/lexum/config.yml
cluster:
  name: lexum-prod
  initial_master_nodes: [node-1, node-2, node-3]

node:
  name: node-1
  roles: [master, data, ingest]

network:
  host: 0.0.0.0
  http_port: 9200
  transport_port: 9300
  publish_host: 192.168.1.10

discovery:
  seed_hosts:
    - 192.168.1.10:9300
    - 192.168.1.11:9300
    - 192.168.1.12:9300

path:
  data: /var/lib/lexum/data
  logs: /var/log/lexum

security:
  enabled: true
  tls:
    enabled: true
    certificate: /etc/lexum/certs/node.crt
    key: /etc/lexum/certs/node.key
```

### Start Service

```bash
# Enable and start
sudo systemctl enable lexum
sudo systemctl start lexum

# Check status
sudo systemctl status lexum

# View logs
sudo journalctl -u lexum -f
```

## Configuration

### Environment Variables

```bash
# Cluster
CLUSTER_NAME=lexum-prod
NODE_NAME=node-1
NODE_ROLES=master,data

# Network
NETWORK_HOST=0.0.0.0
HTTP_PORT=9200
TRANSPORT_PORT=9300

# Discovery
DISCOVERY_SEED_HOSTS=master-1,master-2,master-3
INITIAL_MASTER_NODES=master-1,master-2,master-3

# Paths
PATH_DATA=/data
PATH_LOGS=/var/log/lexum

# Performance
THREAD_POOL_SIZE=4
QUERY_CACHE_SIZE=1g
FIELD_CACHE_SIZE=512m

# Security
SECURITY_ENABLED=true
TLS_ENABLED=true
```

### Performance Tuning

```yaml
# config.yml
performance:
  # Thread pools
  thread_pool:
    search:
      size: 8
      queue_size: 1000
    index:
      size: 4
      queue_size: 500
    bulk:
      size: 4
      queue_size: 200

  # Caching
  cache:
    query_cache:
      enabled: true
      size: 1gb
    field_cache:
      enabled: true
      size: 512mb
    filter_cache:
      enabled: true
      size: 256mb

  # Memory
  heap_size: 4g

  # Disk
  merge_policy:
    max_merged_segment: 5gb
    segments_per_tier: 10
```

## Scaling

### Vertical Scaling

1. Increase RAM
2. Add CPU cores
3. Use faster disks (NVMe)
4. Increase cache sizes

### Horizontal Scaling

```bash
# Add data node
docker run -d \
  --name lexum-data-4 \
  -e CLUSTER_NAME=lexum-prod \
  -e NODE_NAME=data-4 \
  -e NODE_ROLES=data,ingest \
  -e DISCOVERY_SEED_HOSTS=master-1,master-2,master-3 \
  -v data-4:/data \
  lexum/lexum:latest

# Wait for node to join
curl http://localhost:9200/_cluster/health

# Rebalance shards
curl -X POST http://localhost:9200/_cluster/reroute
```

## Backup and Restore

### Snapshot Repository

```bash
# Create repository
curl -X PUT http://localhost:9200/_snapshot/backups \
  -H 'Content-Type: application/json' \
  -d '{
    "type": "fs",
    "settings": {
      "location": "/backups",
      "compress": true
    }
  }'
```

### Create Snapshot

```bash
# Snapshot all indices
curl -X PUT http://localhost:9200/_snapshot/backups/snapshot_1

# Snapshot specific indices
curl -X PUT http://localhost:9200/_snapshot/backups/snapshot_2 \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": "index1,index2",
    "include_global_state": false
  }'
```

### Restore

```bash
# Restore snapshot
curl -X POST http://localhost:9200/_snapshot/backups/snapshot_1/_restore

# Restore specific indices
curl -X POST http://localhost:9200/_snapshot/backups/snapshot_1/_restore \
  -H 'Content-Type: application/json' \
  -d '{
    "indices": "index1"
  }'
```

### Automated Backups

```bash
# Cron job
0 2 * * * curl -X PUT http://localhost:9200/_snapshot/backups/snapshot_$(date +\%Y\%m\%d)
```

## Monitoring

See [TELEMETRY.md](./TELEMETRY.md) for comprehensive monitoring setup.

## Security

### TLS Configuration

```yaml
# config.yml
security:
  tls:
    enabled: true
    http:
      certificate: /etc/lexum/certs/http.crt
      key: /etc/lexum/certs/http.key
      ca: /etc/lexum/certs/ca.crt
    transport:
      certificate: /etc/lexum/certs/transport.crt
      key: /etc/lexum/certs/transport.key
      ca: /etc/lexum/certs/ca.crt
```

### Generate Certificates

```bash
# Create CA
openssl genrsa -out ca.key 4096
openssl req -new -x509 -days 3650 -key ca.key -out ca.crt

# Create node certificate
openssl genrsa -out node.key 2048
openssl req -new -key node.key -out node.csr
openssl x509 -req -days 365 -in node.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out node.crt
```

## Troubleshooting

### Common Issues

**Port already in use:**

```bash
# Find process
sudo lsof -i :9200
# Kill process
sudo kill -9 <PID>
```

**Out of memory:**

```bash
# Increase heap size
export LEXUM_HEAP_SIZE=4g
```

**Disk full:**

```bash
# Check disk usage
df -h /data

# Clean old logs
find /var/log/lexum -mtime +7 -delete
```

**Split brain:**

```bash
# Ensure minimum_master_nodes = (master_nodes / 2) + 1
```

## Best Practices

1. **Use dedicated master nodes** in production
2. **Separate hot/warm/cold data** nodes for cost efficiency
3. **Monitor disk usage** and set up alerts at 85%
4. **Regular backups** at least daily
5. **Use SSD** for data nodes
6. **Enable TLS** in production
7. **Set appropriate heap size** (50% of RAM, max 32GB)
8. **Monitor cluster health** continuously
9. **Plan capacity** ahead of growth
10. **Test disaster recovery** procedures

## See Also

- [Architecture](./ARCHITECTURE.md)
- [Telemetry](./TELEMETRY.md)
- [Development](../development/DEVELOPMENT.md)
