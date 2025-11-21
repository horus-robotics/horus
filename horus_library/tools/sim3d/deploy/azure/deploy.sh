#!/bin/bash
# Azure AKS Deployment Script for Sim3D

set -e

# Configuration
RESOURCE_GROUP="${AZURE_RESOURCE_GROUP:-sim3d-rg}"
CLUSTER_NAME="${SIM3D_CLUSTER_NAME:-sim3d-cluster}"
LOCATION="${AZURE_LOCATION:-eastus}"
NODE_SIZE="${AZURE_NODE_SIZE:-Standard_D8s_v3}"
MIN_NODES="${AZURE_MIN_NODES:-2}"
MAX_NODES="${AZURE_MAX_NODES:-20}"
ACR_NAME="${SIM3D_ACR_NAME:-sim3dacr}"

echo "=== Sim3D Azure AKS Deployment ==="
echo "Resource Group: $RESOURCE_GROUP"
echo "Cluster: $CLUSTER_NAME"
echo "Location: $LOCATION"
echo "Node Size: $NODE_SIZE"
echo "Nodes: $MIN_NODES - $MAX_NODES"
echo

# Check Azure CLI
if ! command -v az &> /dev/null; then
    echo "Error: Azure CLI not found. Please install it first."
    exit 1
fi

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    echo "Error: kubectl not found. Please install it first."
    exit 1
fi

# Step 1: Login to Azure
echo "Step 1: Logging in to Azure..."
az login

# Step 2: Create resource group
echo "Step 2: Creating resource group..."
az group create --name $RESOURCE_GROUP --location $LOCATION || echo "Resource group already exists"

# Step 3: Create ACR
echo "Step 3: Creating Azure Container Registry..."
az acr create \
    --resource-group $RESOURCE_GROUP \
    --name $ACR_NAME \
    --sku Standard \
    --location $LOCATION \
    || echo "ACR already exists"

# Login to ACR
az acr login --name $ACR_NAME

ACR_URI="$ACR_NAME.azurecr.io"
echo "ACR: $ACR_URI"

# Step 4: Build and push Docker image
echo "Step 4: Building and pushing Docker image..."
cd ../../../
docker build -t sim3d:latest -f horus_library/tools/sim3d/Dockerfile .
docker tag sim3d:latest $ACR_URI/sim3d:latest
docker tag sim3d:latest $ACR_URI/sim3d:$(git rev-parse --short HEAD)
docker push $ACR_URI/sim3d:latest
docker push $ACR_URI/sim3d:$(git rev-parse --short HEAD)

cd horus_library/tools/sim3d

# Step 5: Create AKS cluster
echo "Step 5: Creating AKS cluster..."
az aks create \
    --resource-group $RESOURCE_GROUP \
    --name $CLUSTER_NAME \
    --location $LOCATION \
    --node-count $MIN_NODES \
    --node-vm-size $NODE_SIZE \
    --enable-autoscaler \
    --min-count $MIN_NODES \
    --max-count $MAX_NODES \
    --enable-addons monitoring \
    --attach-acr $ACR_NAME \
    --generate-ssh-keys \
    || echo "Cluster already exists"

# Get credentials
az aks get-credentials --resource-group $RESOURCE_GROUP --name $CLUSTER_NAME

# Step 6: Deploy Sim3D
echo "Step 6: Deploying Sim3D..."
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/storage.yaml
kubectl apply -f k8s/configmap.yaml

# Update deployment with ACR image
sed "s|image: sim3d:latest|image: $ACR_URI/sim3d:latest|g" k8s/deployment.yaml | kubectl apply -f -

kubectl apply -f k8s/autoscaling.yaml

# Step 7: Wait for deployment
echo "Step 7: Waiting for deployment to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/sim3d -n sim3d

# Step 8: Get service endpoint
echo "Step 8: Getting service endpoint..."
kubectl get svc -n sim3d sim3d-service

echo
echo "=== Deployment Complete ==="
echo "Resource Group: $RESOURCE_GROUP"
echo "Cluster: $CLUSTER_NAME"
echo "Location: $LOCATION"
echo "Image: $ACR_URI/sim3d:latest"
echo
echo "To check status:"
echo "  kubectl get pods -n sim3d"
echo
echo "To get logs:"
echo "  kubectl logs -n sim3d -l app=sim3d --tail=100"
echo
echo "To access dashboard:"
echo "  az aks browse --resource-group $RESOURCE_GROUP --name $CLUSTER_NAME"
