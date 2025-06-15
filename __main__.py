"""An AWS Python Pulumi program"""
import base64

import os
import pulumi
import pulumi_aws as aws
import pulumi_docker as docker
from pulumi_docker import RegistryArgs

# Get neccessary settings from the pulumi config
config = pulumi.Config()
availability_zone = aws.config.region

hash_tag = os.getenv('GITHUB_SHA', 'unknown')
tags = ['latest', 'dev']

if hash_tag != 'unknown':
    tags.append(hash_tag)
# TODO Add 80 port mapping

# Define the user's home directory dynamically
home_dir = os.path.expanduser("~")

# Ensure .aws directory exists
aws_credentials_path = os.path.join(home_dir, ".aws")
pulumi.info(f"aws credentials path={aws_credentials_path}")

if not os.path.exists(aws_credentials_path):
    pulumi.error(f"The .aws directory is missing in {aws_credentials_path}. Make sure to create it.")
    import sys
    sys.exit(1)

pulumi.info(f"Running the configuration in this region {availability_zone}")
pulumi.info(f'Possible tags = {tags}')
# Create an IAM policy that allows ECR repository creation
# Creating storage space to upload a docker image of our app to

url= "886248216134.dkr.ecr.eu-west-1.amazonaws.com/p2p/tracker-server"
# Fetch the ECR repository details
app_ecr_repo = aws.ecr.get_repository(name='p2p/tracker-server')

# Attaching an application life cycle policy to the storage
app_lifecycle_policy = aws.ecr.LifecyclePolicy(
    "app-lifecycle-policy",
    repository=app_ecr_repo.name,
    policy="""{
        "rules": [
            {
                "rulePriority": 10,
                "description": "Remove untagged images",
                "selection": {
                    "tagStatus": "untagged",
                    "countType": "imageCountMoreThan",
                    "countNumber": 1
                },
                "action": {
                    "type": "expire"
                }
            }
        ]
    }""",
)



def get_registry_info(rid):
    creds = aws.ecr.get_credentials(registry_id=rid)
    decoded = base64.b64decode(creds.authorization_token).decode()
    parts = decoded.split(':')
    if len(parts) != 2:
        raise Exception("Invalid credentials")
    return RegistryArgs(
        server=creds.proxy_endpoint,
        username=parts[0],
        password=parts[1],
    )


app_registry = get_registry_info(app_ecr_repo.registry_id)

for tag in tags:
    url_version = f"{url}:{tag}"
    app_image = docker.Image(
        f"tracker-server-{tag}",
        image_name=url_version,
        build=docker.DockerBuildArgs(
            context='.',
            platform='linux/amd64',
        ),
        skip_push=False,
        registry=app_registry
    )
    pulumi.export("app image:", app_image.repo_digest)




