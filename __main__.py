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

url_relay= "886248216134.dkr.ecr.eu-west-1.amazonaws.com/p2p/tracker-server"
url_backend = "886248216134.dkr.ecr.eu-west-1.amazonaws.com/p2p/backend"
url_frontend = "886248216134.dkr.ecr.eu-west-1.amazonaws.com/p2p/frontend"
url_pyagents = "886248216134.dkr.ecr.eu-west-1.amazonaws.com/p2p/pyagents"
urls_with_contexts = [
    (url_relay, ".", "tracker-server", "Dockerfile"),
    (url_backend, ".", "backend", "Dockerfile.backend"),
    (url_frontend,".", "frontend", "Dockerfile.frontend"),
    (url_pyagents, "py-agents/", "pyagents", "Dockerfile.pyagents")]


# Fetch the ECR repository details
app_ecr_repo_relay = aws.ecr.get_repository(name='p2p/tracker-server')
app_ecr_repo_backend = aws.ecr.get_repository(name='p2p/backend')
app_ecr_repo_frontend = aws.ecr.get_repository(name='p2p/frontend')
app_ecr_repo_pyagents = aws.ecr.get_repository(name='p2p/pyagents')

app_ecr_repos = [("relay-server", app_ecr_repo_relay), ("backend", app_ecr_repo_backend),
                 ("frontend", app_ecr_repo_frontend),
                 ("pyagents", app_ecr_repo_pyagents)
                 ]

def set_up_policy(name, app_ecr_repo):
    return aws.ecr.LifecyclePolicy(
        f"app-lifecycle-policy-{name}",
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


def new_docker_image(url, tag, context, name, dockerfile):
    url_version = f"{url}:{tag}"
    app_image = docker.Image(
        f"{name}-{tag}",
        image_name=url_version,
        build=docker.DockerBuildArgs(
            context=context,
            platform='linux/amd64',
            dockerfile=dockerfile
        ),
        skip_push=False,
        registry=app_registry
    )
    return app_image

for (name, app_ecr_repo) in app_ecr_repos:
    print(f"Creating policy for {name}")
    set_up_policy(name, app_ecr_repo)


for (name, app_ecr_repo) in app_ecr_repos:
    print(f"Creating {app_ecr_repo.registry_id}")
    app_registry = get_registry_info(app_ecr_repo.registry_id)


app_images = []
for (url, context, name, dockerfile) in urls_with_contexts:
    for tag in tags:
        print(f"{url}:{tag} with context={context}")
        app_image = new_docker_image(url, tag, context, name, dockerfile)
        app_images.append(app_image.repo_digest)

for app_image in app_images:
    pulumi.export("app image:", app_image)




