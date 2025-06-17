#!/bin/bash

aws="/usr/local/bin/aws"

# Step 1: Get the cluster name (you can change the index or filter if needed)
CLUSTER=$($aws ecs list-clusters \
  --query "clusterArns[?contains(@, 'app-cluster')]|[0]" \
  --output text | awk -F'/' '{print $2}')

echo "🔧 Cluster: $CLUSTER"

# Step 2: Get the first task ARN in the cluster
TASK_ARN=$($aws ecs list-tasks \
  --cluster "$CLUSTER" \
  --query "taskArns[0]" \
  --output text)

echo "📦 Task ARN: $TASK_ARN"

# Step 3: Extract the Task ID from the full ARN
TASK_ID=$(basename "$TASK_ARN")

echo "🔍 Task ID: $TASK_ID"

# Step 1: Extract the ENI ID from the ECS task
ENI_ID=$($aws ecs describe-tasks \
  --cluster "$CLUSTER" \
  --tasks "$TASK_ID" \
  --query "tasks[0].attachments[0].details[?name=='networkInterfaceId'].value" \
  --output text)

echo "🔌 ENI ID: $ENI_ID"

# Step 2: Get the public IP from the ENI
PUBLIC_IP=$($aws ec2 describe-network-interfaces \
  --network-interface-ids "$ENI_ID" \
  --query "NetworkInterfaces[0].Association.PublicIp" \
  --output text)

echo "🌍 Public IP: $PUBLIC_IP"
echo "Tracker address=http://52.51.213.92:3000/tracker"

