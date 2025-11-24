# Cloud Deployment Guide

This guide covers deploying Sim3D for production use cases including Docker containers, Kubernetes orchestration, and cloud provider deployments.

## Docker Deployment

### Building the Docker Image

Sim3D includes a multi-stage Dockerfile optimized for production:

```bash
# Build the Docker image
cd horus_library/tools/sim3d
docker build -t sim3d:latest .

# Build with specific features
docker build --build-arg FEATURES="headless" -t sim3d:headless .
```

### Dockerfile Overview

```dockerfile
# Multi-stage Docker build for Sim3D
# Stage 1: Build stage
FROM rust:1.75-slim-bullseye as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libudev-dev \
    libxcb-render0-dev \
    libxcb-shape0-dev \
    libxcb-xfixes0-dev \
    libxkbcommon-dev \
    libfontconfig1-dev \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy and build dependencies (cached layer)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src && echo "fn main() {}" > src/main.rs
RUN cargo build --release --features headless

# Copy actual source and rebuild
COPY . .
RUN cargo build --release --features headless

# Stage 2: Runtime stage
FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libudev1 \
    libfontconfig1 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 sim3d && \
    mkdir -p /app /data /recordings && \
    chown -R sim3d:sim3d /app /data /recordings

COPY --from=builder /build/target/release/sim3d /app/sim3d

WORKDIR /app
USER sim3d

ENV RUST_LOG=info
ENV SIM3D_DATA_DIR=/data
ENV SIM3D_RECORDINGS_DIR=/recordings

EXPOSE 8080
VOLUME ["/data", "/recordings"]

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD [ "test", "-f", "/app/sim3d" ]

ENTRYPOINT ["/app/sim3d"]
CMD ["--help"]
```

### Running with Docker

```bash
# Basic run
docker run --rm sim3d:latest --mode headless --task navigation

# With volume mounts for data persistence
docker run --rm \
    -v $(pwd)/data:/data \
    -v $(pwd)/recordings:/recordings \
    sim3d:latest --mode headless --task navigation

# With GPU support (NVIDIA)
docker run --rm \
    --gpus all \
    -v $(pwd)/data:/data \
    sim3d:latest --mode headless --task navigation

# Interactive mode
docker run -it --rm sim3d:latest /bin/bash
```

### Docker Compose

```yaml
version: '3.8'

services:
  sim3d:
    image: sim3d:latest
    build:
      context: .
      dockerfile: Dockerfile
    environment:
      - RUST_LOG=info
      - SIM3D_HEADLESS=true
    volumes:
      - sim3d_data:/data
      - sim3d_recordings:/recordings
    ports:
      - "8080:8080"
    deploy:
      resources:
        limits:
          cpus: '4'
          memory: 8G
        reservations:
          cpus: '2'
          memory: 4G
    command: ["--mode", "headless", "--task", "navigation"]

  tensorboard:
    image: tensorflow/tensorflow:latest
    ports:
      - "6006:6006"
    volumes:
      - sim3d_data:/logs:ro
    command: ["tensorboard", "--logdir=/logs", "--bind_all"]

volumes:
  sim3d_data:
  sim3d_recordings:
```

## Kubernetes Deployment

### Namespace and ConfigMap

```yaml
# k8s/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: sim3d
  labels:
    app: sim3d

---
# k8s/configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: sim3d-config
  namespace: sim3d
data:
  RUST_LOG: "info"
  SIM3D_HEADLESS: "true"
  MAX_PARALLEL_ENVS: "32"
  PHYSICS_TIMESTEP: "0.016667"
```

### Persistent Volume Claims

```yaml
# k8s/storage.yaml
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: sim3d-data-pvc
  namespace: sim3d
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 50Gi
  storageClassName: standard

---
apiVersion: v1
kind: PersistentVolumeClaim
metadata:
  name: sim3d-recordings-pvc
  namespace: sim3d
spec:
  accessModes:
    - ReadWriteOnce
  resources:
    requests:
      storage: 100Gi
  storageClassName: standard
```

### Deployment

```yaml
# k8s/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: sim3d
  namespace: sim3d
  labels:
    app: sim3d
    component: simulator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: sim3d
      component: simulator
  template:
    metadata:
      labels:
        app: sim3d
        component: simulator
    spec:
      containers:
      - name: sim3d
        image: sim3d:latest
        imagePullPolicy: IfNotPresent
        ports:
        - containerPort: 8080
          name: http
          protocol: TCP
        env:
        - name: RUST_LOG
          valueFrom:
            configMapKeyRef:
              name: sim3d-config
              key: RUST_LOG
        - name: SIM3D_HEADLESS
          valueFrom:
            configMapKeyRef:
              name: sim3d-config
              key: SIM3D_HEADLESS
        - name: SIM3D_DATA_DIR
          value: "/data"
        - name: SIM3D_RECORDINGS_DIR
          value: "/recordings"
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        resources:
          requests:
            cpu: "2000m"
            memory: "4Gi"
          limits:
            cpu: "4000m"
            memory: "8Gi"
        volumeMounts:
        - name: data
          mountPath: /data
        - name: recordings
          mountPath: /recordings
        - name: config
          mountPath: /config
          readOnly: true
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
      volumes:
      - name: data
        persistentVolumeClaim:
          claimName: sim3d-data-pvc
      - name: recordings
        persistentVolumeClaim:
          claimName: sim3d-recordings-pvc
      - name: config
        configMap:
          name: sim3d-config
      restartPolicy: Always

---
# k8s/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: sim3d-service
  namespace: sim3d
spec:
  type: LoadBalancer
  selector:
    app: sim3d
    component: simulator
  ports:
  - port: 80
    targetPort: 8080
    protocol: TCP
    name: http
  sessionAffinity: ClientIP
```

### Horizontal Pod Autoscaler

```yaml
# k8s/hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: sim3d-hpa
  namespace: sim3d
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: sim3d
  minReplicas: 2
  maxReplicas: 20
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  behavior:
    scaleDown:
      stabilizationWindowSeconds: 300
      policies:
      - type: Percent
        value: 10
        periodSeconds: 60
    scaleUp:
      stabilizationWindowSeconds: 0
      policies:
      - type: Percent
        value: 100
        periodSeconds: 15
```

### Deploy to Kubernetes

```bash
# Create namespace and apply configurations
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/configmap.yaml
kubectl apply -f k8s/storage.yaml
kubectl apply -f k8s/deployment.yaml
kubectl apply -f k8s/hpa.yaml

# Check status
kubectl -n sim3d get pods
kubectl -n sim3d get services
kubectl -n sim3d get hpa

# View logs
kubectl -n sim3d logs -f deployment/sim3d

# Scale manually
kubectl -n sim3d scale deployment/sim3d --replicas=5
```

## AWS Deployment

### ECS Task Definition

```json
{
  "family": "sim3d",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "4096",
  "memory": "8192",
  "executionRoleArn": "arn:aws:iam::ACCOUNT:role/ecsTaskExecutionRole",
  "taskRoleArn": "arn:aws:iam::ACCOUNT:role/sim3dTaskRole",
  "containerDefinitions": [
    {
      "name": "sim3d",
      "image": "ACCOUNT.dkr.ecr.REGION.amazonaws.com/sim3d:latest",
      "essential": true,
      "portMappings": [
        {
          "containerPort": 8080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {"name": "RUST_LOG", "value": "info"},
        {"name": "SIM3D_HEADLESS", "value": "true"}
      ],
      "mountPoints": [
        {
          "sourceVolume": "data",
          "containerPath": "/data"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/sim3d",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "sim3d"
        }
      }
    }
  ],
  "volumes": [
    {
      "name": "data",
      "efsVolumeConfiguration": {
        "fileSystemId": "fs-XXXXXXXX",
        "rootDirectory": "/sim3d"
      }
    }
  ]
}
```

### CloudFormation Template

```yaml
# aws/cloudformation.yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: Sim3D ECS Deployment

Parameters:
  VpcId:
    Type: AWS::EC2::VPC::Id
  SubnetIds:
    Type: List<AWS::EC2::Subnet::Id>
  DesiredCount:
    Type: Number
    Default: 2

Resources:
  ECSCluster:
    Type: AWS::ECS::Cluster
    Properties:
      ClusterName: sim3d-cluster
      CapacityProviders:
        - FARGATE
        - FARGATE_SPOT

  TaskDefinition:
    Type: AWS::ECS::TaskDefinition
    Properties:
      Family: sim3d
      NetworkMode: awsvpc
      RequiresCompatibilities:
        - FARGATE
      Cpu: '4096'
      Memory: '8192'
      ExecutionRoleArn: !GetAtt ExecutionRole.Arn
      TaskRoleArn: !GetAtt TaskRole.Arn
      ContainerDefinitions:
        - Name: sim3d
          Image: !Sub '${AWS::AccountId}.dkr.ecr.${AWS::Region}.amazonaws.com/sim3d:latest'
          Essential: true
          PortMappings:
            - ContainerPort: 8080
          Environment:
            - Name: RUST_LOG
              Value: info
            - Name: SIM3D_HEADLESS
              Value: 'true'
          LogConfiguration:
            LogDriver: awslogs
            Options:
              awslogs-group: /ecs/sim3d
              awslogs-region: !Ref AWS::Region
              awslogs-stream-prefix: sim3d

  Service:
    Type: AWS::ECS::Service
    Properties:
      ServiceName: sim3d-service
      Cluster: !Ref ECSCluster
      TaskDefinition: !Ref TaskDefinition
      DesiredCount: !Ref DesiredCount
      LaunchType: FARGATE
      NetworkConfiguration:
        AwsvpcConfiguration:
          AssignPublicIp: ENABLED
          Subnets: !Ref SubnetIds
          SecurityGroups:
            - !Ref SecurityGroup

  SecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Sim3D Security Group
      VpcId: !Ref VpcId
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 8080
          ToPort: 8080
          CidrIp: 0.0.0.0/0

  ExecutionRole:
    Type: AWS::IAM::Role
    Properties:
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Service: ecs-tasks.amazonaws.com
            Action: sts:AssumeRole
      ManagedPolicyArns:
        - arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy

  TaskRole:
    Type: AWS::IAM::Role
    Properties:
      AssumeRolePolicyDocument:
        Version: '2012-10-17'
        Statement:
          - Effect: Allow
            Principal:
              Service: ecs-tasks.amazonaws.com
            Action: sts:AssumeRole
```

## GCP Deployment

### Google Cloud Run

```bash
# Build and push to GCR
gcloud builds submit --tag gcr.io/PROJECT_ID/sim3d

# Deploy to Cloud Run
gcloud run deploy sim3d \
    --image gcr.io/PROJECT_ID/sim3d \
    --platform managed \
    --region us-central1 \
    --memory 8Gi \
    --cpu 4 \
    --max-instances 20 \
    --set-env-vars RUST_LOG=info,SIM3D_HEADLESS=true
```

### GKE Deployment

```bash
# Create GKE cluster
gcloud container clusters create sim3d-cluster \
    --num-nodes=3 \
    --machine-type=n1-standard-4 \
    --region=us-central1

# Get credentials
gcloud container clusters get-credentials sim3d-cluster --region=us-central1

# Deploy
kubectl apply -f k8s/
```

## Azure Deployment

### Azure Container Instances

```bash
# Create resource group
az group create --name sim3d-rg --location eastus

# Create container instance
az container create \
    --resource-group sim3d-rg \
    --name sim3d \
    --image sim3d:latest \
    --cpu 4 \
    --memory 8 \
    --ports 8080 \
    --environment-variables RUST_LOG=info SIM3D_HEADLESS=true
```

### AKS Deployment

```bash
# Create AKS cluster
az aks create \
    --resource-group sim3d-rg \
    --name sim3d-aks \
    --node-count 3 \
    --node-vm-size Standard_D4s_v3 \
    --generate-ssh-keys

# Get credentials
az aks get-credentials --resource-group sim3d-rg --name sim3d-aks

# Deploy
kubectl apply -f k8s/
```

## Headless Rendering

For visual rendering without a display (e.g., for recording videos):

### Using Xvfb

```bash
# Install Xvfb
apt-get install -y xvfb

# Run with virtual framebuffer
Xvfb :99 -screen 0 1920x1080x24 &
export DISPLAY=:99

# Run Sim3D
./sim3d --mode visual --record output.mp4
```

### Using EGL (Preferred)

```bash
# Build with EGL support
cargo build --release --features "headless,egl"

# Run (no X required)
./sim3d --mode visual --backend egl --record output.mp4
```

### Docker with Virtual Display

```dockerfile
FROM sim3d:latest

RUN apt-get update && apt-get install -y xvfb && rm -rf /var/lib/apt/lists/*

COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT ["/entrypoint.sh"]
```

```bash
#!/bin/bash
# entrypoint.sh
Xvfb :99 -screen 0 1920x1080x24 &
export DISPLAY=:99
exec /app/sim3d "$@"
```

## Performance Optimization

### Environment Variables

```bash
# Optimize for headless training
export RUST_LOG=warn                    # Reduce logging overhead
export RAYON_NUM_THREADS=8              # Parallel computation threads
export SIM3D_PHYSICS_SUBSTEPS=4         # Physics accuracy
export SIM3D_DISABLE_RENDERING=true     # Skip all rendering
```

### Resource Limits

```yaml
# Kubernetes resource configuration
resources:
  requests:
    cpu: "2000m"     # 2 CPU cores
    memory: "4Gi"    # 4 GB RAM
  limits:
    cpu: "4000m"     # 4 CPU cores max
    memory: "8Gi"    # 8 GB RAM max
```

### Batch Processing

```python
# Python example for batch environment management
import sim3d_rl
import multiprocessing as mp

def run_simulation(env_id, config):
    env = sim3d_rl.make_env(**config)
    # ... training logic
    env.close()

# Run multiple simulations in parallel
configs = [{"task": "navigation", "seed": i} for i in range(32)]
with mp.Pool(8) as pool:
    pool.starmap(run_simulation, enumerate(configs))
```

## Monitoring and Logging

### Prometheus Metrics

```yaml
# k8s/servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: sim3d
  namespace: sim3d
spec:
  selector:
    matchLabels:
      app: sim3d
  endpoints:
  - port: http
    path: /metrics
    interval: 15s
```

### Grafana Dashboard

Import the provided dashboard from `k8s/grafana-dashboard.json` or create custom visualizations for:

- Physics step duration
- Environment steps per second
- Memory usage
- CPU utilization
- Network I/O

### Log Aggregation

```yaml
# Fluentd configuration for log collection
apiVersion: v1
kind: ConfigMap
metadata:
  name: fluentd-config
data:
  fluent.conf: |
    <source>
      @type tail
      path /var/log/containers/sim3d*.log
      pos_file /var/log/fluentd-sim3d.pos
      tag sim3d
      <parse>
        @type json
      </parse>
    </source>

    <match sim3d>
      @type elasticsearch
      host elasticsearch.logging
      port 9200
      index_name sim3d-logs
    </match>
```
