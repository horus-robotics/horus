#

 Sim3D Cloud Deployment Guide

This directory contains deployment configurations and scripts for deploying Sim3D to various cloud platforms.

## Quick Start

### Local Docker Deployment

```bash
# Build and run locally
docker-compose up

# Run in headless mode for training
docker-compose up -d

# Scale workers
docker-compose --profile distributed up -d
```

### Kubernetes Deployment

```bash
# Apply all Kubernetes configs
kubectl apply -f k8s/

# Check deployment status
kubectl get all -n sim3d

# View logs
kubectl logs -n sim3d -l app=sim3d --tail=100 -f
```

## Cloud Platform Deployment

### AWS (EKS)

```bash
cd deploy/aws
export AWS_REGION=us-west-2
export SIM3D_CLUSTER_NAME=sim3d-prod
export AWS_MIN_NODES=3
export AWS_MAX_NODES=20

./deploy.sh
```

**Requirements:**
- AWS CLI configured
- kubectl installed
- eksctl installed
- Docker installed

### GCP (GKE)

```bash
cd deploy/gcp
export GCP_PROJECT_ID=my-project
export GCP_REGION=us-central1
export GCP_MIN_NODES=3
export GCP_MAX_NODES=20

./deploy.sh
```

**Requirements:**
- gcloud CLI configured
- kubectl installed
- Docker installed

### Azure (AKS)

```bash
cd deploy/azure
export AZURE_RESOURCE_GROUP=sim3d-rg
export AZURE_LOCATION=eastus
export AZURE_MIN_NODES=3
export AZURE_MAX_NODES=20

./deploy.sh
```

**Requirements:**
- Azure CLI configured
- kubectl installed
- Docker installed

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level | `info` |
| `SIM3D_DATA_DIR` | Data directory | `/data` |
| `SIM3D_RECORDINGS_DIR` | Recordings directory | `/recordings` |
| `SIM3D_HEADLESS` | Run in headless mode | `true` |
| `SIM3D_NUM_WORKERS` | Number of parallel environments | `16` |

### Resource Limits

**Per Pod:**
- CPU Request: 2 cores
- CPU Limit: 4 cores
- Memory Request: 4 GiB
- Memory Limit: 8 GiB

### Auto-scaling

The HPA (Horizontal Pod Autoscaler) will scale based on:
- CPU utilization > 70%
- Memory utilization > 80%

**Scaling behavior:**
- Min replicas: 2
- Max replicas: 20
- Scale up: +100% or +4 pods (max) every 30s
- Scale down: -50% or -2 pods (min) every 60s

## Storage

### Persistent Volumes

- **Data PVC**: 100 GiB (ReadWriteMany)
  - Used for environment data, models, assets
- **Recordings PVC**: 500 GiB (ReadWriteMany)
  - Used for trajectory recordings, videos, datasets

### Storage Classes

Adjust storage class in `k8s/storage.yaml`:
- AWS: `gp2` or `gp3`
- GCP: `standard` or `pd-ssd`
- Azure: `default` or `managed-premium`

## Monitoring

### Kubernetes Dashboard

```bash
kubectl proxy
# Access at: http://localhost:8001/api/v1/namespaces/kubernetes-dashboard/services/https:kubernetes-dashboard:/proxy/
```

### Logs

```bash
# All pods
kubectl logs -n sim3d -l app=sim3d --tail=100 -f

# Specific pod
kubectl logs -n sim3d pod/sim3d-xxxxx -f

# Previous crashed pod
kubectl logs -n sim3d pod/sim3d-xxxxx --previous
```

### Metrics

```bash
# Pod metrics
kubectl top pods -n sim3d

# Node metrics
kubectl top nodes
```

## Operations

### Scaling

```bash
# Manual scaling
kubectl scale deployment sim3d -n sim3d --replicas=10

# Check autoscaler status
kubectl get hpa -n sim3d

# Describe autoscaler
kubectl describe hpa sim3d-hpa -n sim3d
```

### Updates

```bash
# Update image
kubectl set image deployment/sim3d sim3d=sim3d:v2.0 -n sim3d

# Rollout status
kubectl rollout status deployment/sim3d -n sim3d

# Rollback
kubectl rollout undo deployment/sim3d -n sim3d
```

### Debugging

```bash
# Exec into pod
kubectl exec -it -n sim3d pod/sim3d-xxxxx -- /bin/bash

# Port forward
kubectl port-forward -n sim3d svc/sim3d-service 8080:80

# Describe pod
kubectl describe pod -n sim3d sim3d-xxxxx

# Events
kubectl get events -n sim3d --sort-by='.lastTimestamp'
```

## Distributed Training

For multi-node distributed RL training:

```bash
# Use distributed profile
docker-compose --profile distributed up -d

# Or in Kubernetes, scale up
kubectl scale deployment sim3d -n sim3d --replicas=10
```

Configure distributed training in `k8s/configmap.yaml`:
```yaml
rl:
  num_envs: 16
  parallel: true
  async_mode: true
  distributed: true
  world_size: 10  # Number of replicas
```

## Cost Optimization

### Spot Instances

**AWS:**
```bash
eksctl create nodegroup \
  --cluster sim3d-cluster \
  --name sim3d-spot \
  --node-type m5.2xlarge \
  --nodes-min 2 \
  --nodes-max 20 \
  --spot
```

**GCP:**
```bash
gcloud container node-pools create sim3d-spot \
  --cluster sim3d-cluster \
  --preemptible \
  --machine-type n1-standard-8 \
  --num-nodes 2
```

**Azure:**
```bash
az aks nodepool add \
  --resource-group sim3d-rg \
  --cluster-name sim3d-cluster \
  --name sim3dspot \
  --priority Spot \
  --eviction-policy Delete \
  --spot-max-price -1 \
  --node-count 2
```

### Auto-shutdown

Add to `k8s/configmap.yaml`:
```yaml
shutdown:
  idle_timeout: 3600  # Shutdown after 1 hour of inactivity
  schedule: "0 2 * * *"  # Daily at 2 AM
```

## Troubleshooting

### Common Issues

**Pods stuck in Pending:**
```bash
kubectl describe pod -n sim3d sim3d-xxxxx
# Check for resource constraints or PVC binding issues
```

**OOMKilled pods:**
```bash
# Increase memory limits in k8s/deployment.yaml
resources:
  limits:
    memory: "16Gi"
```

**Image pull errors:**
```bash
# Check image exists
docker images | grep sim3d

# Check registry credentials
kubectl get secret -n sim3d
```

## Security

### RBAC

Create service account with limited permissions:
```bash
kubectl create serviceaccount sim3d-sa -n sim3d
kubectl create rolebinding sim3d-rb --role=sim3d-role --serviceaccount=sim3d:sim3d-sa -n sim3d
```

### Network Policies

Apply network policies to restrict traffic:
```bash
kubectl apply -f k8s/network-policy.yaml
```

### Secrets

Store sensitive data in Kubernetes secrets:
```bash
kubectl create secret generic sim3d-secrets \
  --from-literal=api-key=xxxxx \
  -n sim3d
```

## Backup

### Automated Backups

```bash
# Backup recordings to S3/GCS/Azure Blob
kubectl create cronjob backup-recordings \
  --image=backup-tool:latest \
  --schedule="0 2 * * *" \
  -- /backup.sh
```

## Support

For issues or questions:
- GitHub Issues: https://github.com/your-org/sim3d/issues
- Documentation: https://docs.sim3d.io
- Community: https://discord.gg/sim3d
