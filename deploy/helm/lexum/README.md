# Lexum Helm Chart

A Helm chart for deploying Lexum Search Engine on Kubernetes.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.0+
- Storage class configured in your cluster

## Installation

### Add the repository (if hosted)

```bash
helm repo add lexum https://charts.hivellm.com
helm repo update
```

### Install from local chart

```bash
helm install lexum ./helm/lexum
```

### Install with custom values

```bash
helm install lexum ./helm/lexum -f my-values.yaml
```

### Install with specific namespace

```bash
helm install lexum ./helm/lexum --namespace lexum --create-namespace
```

## Configuration

The following table lists the configurable parameters and their default values:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `3` |
| `image.repository` | Image repository | `lexum` |
| `image.tag` | Image tag | `0.1.0-alpha` |
| `image.pullPolicy` | Image pull policy | `IfNotPresent` |
| `service.type` | Service type | `ClusterIP` |
| `service.port` | Service port | `9200` |
| `ingress.enabled` | Enable ingress | `false` |
| `persistence.enabled` | Enable persistent volumes | `true` |
| `persistence.data.size` | Data volume size | `10Gi` |
| `persistence.snapshots.size` | Snapshots volume size | `20Gi` |
| `resources.requests.memory` | Memory request | `512Mi` |
| `resources.requests.cpu` | CPU request | `500m` |
| `resources.limits.memory` | Memory limit | `2Gi` |
| `resources.limits.cpu` | CPU limit | `2000m` |
| `autoscaling.enabled` | Enable HPA | `true` |
| `autoscaling.minReplicas` | Minimum replicas | `3` |
| `autoscaling.maxReplicas` | Maximum replicas | `10` |

## Examples

### Production deployment

```yaml
replicaCount: 5
resources:
  requests:
    memory: "2Gi"
    cpu: "1000m"
  limits:
    memory: "4Gi"
    cpu: "4000m"
persistence:
  data:
    size: 50Gi
  snapshots:
    size: 100Gi
autoscaling:
  minReplicas: 5
  maxReplicas: 20
```

### Development deployment

```yaml
replicaCount: 1
resources:
  requests:
    memory: "256Mi"
    cpu: "250m"
  limits:
    memory: "512Mi"
    cpu: "500m"
persistence:
  data:
    size: 5Gi
  snapshots:
    size: 10Gi
```

## Upgrading

```bash
helm upgrade lexum ./helm/lexum
```

## Uninstalling

```bash
helm uninstall lexum
```

## Troubleshooting

### Check pod status

```bash
kubectl get pods -l app.kubernetes.io/name=lexum
```

### View logs

```bash
kubectl logs -l app.kubernetes.io/name=lexum -f
```

### Check events

```bash
kubectl get events --sort-by='.lastTimestamp'
```

## Support

For issues and questions, please visit: https://github.com/hivellm/lexum

