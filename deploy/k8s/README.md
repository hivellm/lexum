# Kubernetes Manifests for Lexum

This directory contains Kubernetes manifests for deploying Lexum Search Engine.

## Prerequisites

- Kubernetes cluster (1.19+)
- kubectl configured to access your cluster
- Storage class configured in your cluster

## Quick Start

### 1. Create namespace and resources

```bash
# Apply all manifests
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f service-headless.yaml
kubectl apply -f service.yaml
kubectl apply -f statefulset.yaml
```

### 2. Using Kustomize

```bash
# Deploy with default settings
kubectl apply -k .

# Customize replicas
kubectl kustomize . | kubectl apply -f -
```

### 3. Check deployment status

```bash
# Check pods
kubectl get pods -n lexum

# Check services
kubectl get svc -n lexum

# Check StatefulSet
kubectl get statefulset -n lexum

# View logs
kubectl logs -n lexum -l app=lexum-search-engine -f
```

## Configuration

### ConfigMap

Edit `configmap.yaml` to customize:
- Cluster name
- Network settings
- Logging configuration
- Snapshot settings

### Secrets

Create secrets for sensitive data:

```bash
# API key
kubectl create secret generic lexum-secret \
  --from-literal=api-key=your-api-key \
  --namespace=lexum

# AWS credentials (for S3 snapshots)
kubectl create secret generic lexum-secret \
  --from-literal=aws-access-key-id=your-key \
  --from-literal=aws-secret-access-key=your-secret \
  --namespace=lexum
```

### Storage

Update `storageClassName` in `statefulset.yaml` to match your cluster's storage class:

```yaml
storageClassName: fast-ssd  # Change to your storage class
```

### Ingress

Edit `ingress.yaml` to configure:
- Domain name
- TLS certificates
- Ingress controller annotations

## Scaling

### Manual scaling

```bash
kubectl scale statefulset lexum --replicas=5 -n lexum
```

### Automatic scaling (HPA)

The HPA is configured to scale between 3-10 replicas based on CPU (70%) and memory (80%) usage.

```bash
# Check HPA status
kubectl get hpa -n lexum

# View HPA details
kubectl describe hpa lexum-hpa -n lexum
```

## Monitoring

### Prometheus

If using Prometheus Operator, apply the ServiceMonitor:

```bash
kubectl apply -f servicemonitor.yaml
```

### Health Checks

Health endpoints:
- Liveness: `/_cluster/health`
- Readiness: `/_cluster/health`
- Metrics: `/metrics`

## Troubleshooting

### Pods not starting

```bash
# Check pod events
kubectl describe pod -n lexum <pod-name>

# Check logs
kubectl logs -n lexum <pod-name>
```

### Storage issues

```bash
# Check PVCs
kubectl get pvc -n lexum

# Check PVs
kubectl get pv
```

### Network issues

```bash
# Check services
kubectl get svc -n lexum

# Test connectivity
kubectl run -it --rm debug --image=busybox --restart=Never -n lexum -- wget -qO- http://lexum:9200/_cluster/health
```

## Cleanup

```bash
# Delete all resources
kubectl delete -f .

# Or using kustomize
kubectl delete -k .

# Delete namespace (removes everything)
kubectl delete namespace lexum
```

