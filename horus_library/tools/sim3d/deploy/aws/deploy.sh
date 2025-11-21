#!/bin/bash
# AWS EKS Deployment Script for Sim3D

set -e

# Configuration
CLUSTER_NAME="${SIM3D_CLUSTER_NAME:-sim3d-cluster}"
REGION="${AWS_REGION:-us-west-2}"
NODE_TYPE="${AWS_NODE_TYPE:-m5.2xlarge}"
MIN_NODES="${AWS_MIN_NODES:-2}"
MAX_NODES="${AWS_MAX_NODES:-20}"
ECR_REPO="${SIM3D_ECR_REPO:-sim3d}"

echo "=== Sim3D AWS EKS Deployment ==="
echo "Cluster: $CLUSTER_NAME"
echo "Region: $REGION"
echo "Node Type: $NODE_TYPE"
echo "Nodes: $MIN_NODES - $MAX_NODES"
echo

# Check AWS CLI
if ! command -v aws &> /dev/null; then
    echo "Error: AWS CLI not found. Please install it first."
    exit 1
fi

# Check kubectl
if ! command -v kubectl &> /dev/null; then
    echo "Error: kubectl not found. Please install it first."
    exit 1
fi

# Check eksctl
if ! command -v eksctl &> /dev/null; then
    echo "Error: eksctl not found. Please install it first."
    exit 1
fi

# Step 1: Create ECR repository
echo "Step 1: Creating ECR repository..."
aws ecr create-repository \
    --repository-name $ECR_REPO \
    --region $REGION \
    --image-scanning-configuration scanOnPush=true \
    || echo "Repository already exists"

# Get ECR login
ECR_URI=$(aws ecr describe-repositories --repository-names $ECR_REPO --region $REGION --query 'repositories[0].repositoryUri' --output text)
aws ecr get-login-password --region $REGION | docker login --username AWS --password-stdin $ECR_URI

echo "ECR Repository: $ECR_URI"

# Step 2: Build and push Docker image
echo "Step 2: Building and pushing Docker image..."
cd ../../../
docker build -t $ECR_REPO:latest -f horus_library/tools/sim3d/Dockerfile .
docker tag $ECR_REPO:latest $ECR_URI:latest
docker tag $ECR_REPO:latest $ECR_URI:$(git rev-parse --short HEAD)
docker push $ECR_URI:latest
docker push $ECR_URI:$(git rev-parse --short HEAD)

cd horus_library/tools/sim3d

# Step 3: Create EKS cluster
echo "Step 3: Creating EKS cluster..."
eksctl create cluster \
    --name $CLUSTER_NAME \
    --region $REGION \
    --nodegroup-name sim3d-workers \
    --node-type $NODE_TYPE \
    --nodes $MIN_NODES \
    --nodes-min $MIN_NODES \
    --nodes-max $MAX_NODES \
    --managed \
    --with-oidc \
    --ssh-access \
    --ssh-public-key ~/.ssh/id_rsa.pub \
    --full-ecr-access \
    || echo "Cluster already exists"

# Update kubeconfig
aws eks update-kubeconfig --name $CLUSTER_NAME --region $REGION

# Step 4: Install cluster autoscaler
echo "Step 4: Installing cluster autoscaler..."
kubectl apply -f https://raw.githubusercontent.com/kubernetes/autoscaler/master/cluster-autoscaler/cloudprovider/aws/examples/cluster-autoscaler-autodiscover.yaml

kubectl -n kube-system annotate deployment.apps/cluster-autoscaler \
    cluster-autoscaler.kubernetes.io/safe-to-evict="false" \
    --overwrite

kubectl -n kube-system set image deployment.apps/cluster-autoscaler \
    cluster-autoscaler=k8s.gcr.io/autoscaling/cluster-autoscaler:v1.23.0

# Step 5: Deploy Sim3D
echo "Step 5: Deploying Sim3D..."
kubectl apply -f k8s/namespace.yaml
kubectl apply -f k8s/storage.yaml
kubectl apply -f k8s/configmap.yaml

# Update deployment with ECR image
sed "s|image: sim3d:latest|image: $ECR_URI:latest|g" k8s/deployment.yaml | kubectl apply -f -

kubectl apply -f k8s/autoscaling.yaml

# Step 6: Wait for deployment
echo "Step 6: Waiting for deployment to be ready..."
kubectl wait --for=condition=available --timeout=300s deployment/sim3d -n sim3d

# Step 7: Get service endpoint
echo "Step 7: Getting service endpoint..."
kubectl get svc -n sim3d sim3d-service

echo
echo "=== Deployment Complete ==="
echo "Cluster: $CLUSTER_NAME"
echo "Region: $REGION"
echo "Image: $ECR_URI:latest"
echo
echo "To check status:"
echo "  kubectl get pods -n sim3d"
echo
echo "To get logs:"
echo "  kubectl logs -n sim3d -l app=sim3d --tail=100"
echo
echo "To scale:"
echo "  kubectl scale deployment sim3d -n sim3d --replicas=10"
