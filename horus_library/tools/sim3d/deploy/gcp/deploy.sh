#!/bin/bash
# GCP GKE Deployment Script for Sim3D

set -e

# Configuration
PROJECT_ID="${GCP_PROJECT_ID:-sim3d-project}"
CLUSTER_NAME="${SIM3D_CLUSTER_NAME:-sim3d-cluster}"
REGION="${GCP_REGION:-us-central1}"
ZONE="${GCP_ZONE:-us-central1-a}"
MACHINE_TYPE="${GCP_MACHINE_TYPE:-n1-standard-8}"
MIN_NODES="${GCP_MIN_NODES:-2}"
MAX_NODES="${GCP_MAX_NODES:-20}"
GCR_REPO="${SIM3D_GCR_REPO:-sim3d}"

echo "=== Sim3D GCP GKE Deployment ==="
echo "Project: $PROJECT_ID"
echo "Cluster: $CLUSTER_NAME"
echo "Region: $REGION"
echo "Zone: $ZONE"
echo "Machine Type: $MACHINE_TYPE"
echo "Nodes: $MIN_NODES - $MAX_NODES"
echo

# Check gcloud CLI
if ! command -v gcloud &> /dev/null; then
    echo "Error: gcloud CLI not found. Please install it first."
    exit 1
fi

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    echo "Error: kubectl not found. Please install it first."
    exit 1
fi

# Step 1: Set project
echo "Step 1: Setting GCP project..."
gcloud config set project $PROJECT_ID

# Step 2: Enable required APIs
echo "Step 2: Enabling required APIs..."
gcloud services enable container.googleapis.com
gcloud services enable containerregistry.googleapis.com

# Step 3: Configure Docker for GCR
echo "Step 3: Configuring Docker for GCR..."
gcloud auth configure-docker

# Step 4: Build and push Docker image
echo "Step 4: Building and pushing Docker image..."
cd ../../../
GCR_URI="gcr.io/$PROJECT_ID/$GCR_REPO"
docker build -t $GCR_REPO:latest -f horus_library/tools/sim3d/Dockerfile .
docker tag $GCR_REPO:latest $GCR_URI:latest
docker tag $GCR_REPO:latest $GCR_URI:$(git rev-parse --short HEAD)
docker push $GCR_URI:latest
docker push $GCR_URI:$(git rev-parse --short HEAD)

cd horus_library/tools/sim3d

echo "GCR Image: $GCR_URI:latest"

# Step 5: Create GKE cluster
echo "Step 5: Creating GKE cluster..."
gcloud container clusters create $CLUSTER_NAME \
    --region $REGION \
    --machine-type $MACHINE_TYPE \
    --num-nodes $MIN_NODES \
    --enable-autoscaling \
    --min-nodes $MIN_NODES \
    --max-nodes $MAX_NODES \
    --enable-autorepair \
    --enable-autoupgrade \
    --disk-size 100 \
    --disk-type pd-standard \
    --addons HorizontalPodAutoscaling,HttpLoadBalancing \
    --workload-pool=$PROJECT_ID.svc.id.goog \
    || echo "Cluster already exists"

# Get credentials
gcloud container clusters get-credentials $CLUSTER_NAME --region $REGION

# Step 6: Deploy Sim3D
echo "Step 6: Deploying Sim3D..."
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/storage.yaml
kubectl apply -f k8s/configmap.yaml

# Update deployment with GCR image
sed "s|image: sim3d:latest|image: $GCR_URI:latest|g" k8s/deployment.yaml | kubectl apply -f -

kubectl apply -f k8s/autoscaling.yaml

# Step 7: Wait for deployment
echo "Step 7: Waiting for deployment to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/sim3d -n sim3d

# Step 8: Get service endpoint
echo "Step 8: Getting service endpoint..."
kubectl get svc -n sim3d sim3d-service

echo
echo "=== Deployment Complete ==="
echo "Project: $PROJECT_ID"
echo "Cluster: $CLUSTER_NAME"
echo "Region: $REGION"
echo "Image: $GCR_URI:latest"
echo
echo "To check status:"
echo "  kubectl get pods -n sim3d"
echo
echo "To get logs:"
echo "  kubectl logs -n sim3d -l app=sim3d --tail=100"
echo
echo "To access dashboard:"
echo "  gcloud container clusters get-credentials $CLUSTER_NAME --region $REGION"
echo "  kubectl proxy"
