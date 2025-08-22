"""An AWS Python Pulumi program"""
from pathlib import Path
from typing import Dict, Any

import pulumi
import pulumi_aws as aws

repo_tracker_server = aws.ecr.get_repository(name='p2p/tracker-server')
repo_backend = aws.ecr.get_repository(name='p2p/backend')
repo_frontend = aws.ecr.get_repository(name='p2p/frontend')
repo_pyagents = aws.ecr.get_repository(name='p2p/pyagents')

availability_zone = aws.config.region
tag_deploy='dev'

tracker_server_image = aws.ecr.get_image(
    repository_name=repo_tracker_server.name,
    image_tag=tag_deploy  # Ensure this tag exists in ECR
)

backend_image = aws.ecr.get_image(
    repository_name=repo_backend.name,
    image_tag=tag_deploy  # Ensure this tag exists in ECR
)

frontend_image = aws.ecr.get_image(
    repository_name=repo_frontend.name,
    image_tag=tag_deploy  # Ensure this tag exists in ECR
)

pyagents_image = aws.ecr.get_image(
    repository_name=repo_pyagents.name,
    image_tag=tag_deploy  # Ensure this tag exists in ECR
)




tracker_server_image_name = f"{repo_tracker_server.repository_url}@{tracker_server_image.image_digest}"
backend_image_name = f"{repo_backend.repository_url}@{backend_image.image_digest}"
frontend_image_name = f"{repo_frontend.repository_url}@{frontend_image.image_digest}"
pyagents_image_name = f"{repo_pyagents.repository_url}@{pyagents_image.image_digest}"

app_cluster = aws.ecs.Cluster("app-cluster")


# Creating a VPC and a public subnet
app_vpc = aws.ec2.Vpc("app-vpc", cidr_block="172.31.0.0/16", enable_dns_hostnames=True)

# Private dns
dns_ns = aws.servicediscovery.PrivateDnsNamespace(
    "p2p-ns",
    name="p2p.local",
    vpc=app_vpc.id,
    description="Private DNS for ECS services",
)


app_vpc_subnet = aws.ec2.Subnet(
    "app-vpc-subnet",
    cidr_block="172.31.0.0/20",
    availability_zone="eu-west-1a",
    vpc_id=app_vpc.id,
)

app_vpc_subnet_b = aws.ec2.Subnet(
    "app-vpc-subnet-b",
    cidr_block="172.31.16.0/20",
    availability_zone="eu-west-1b",
    vpc_id=app_vpc.id,
)

# Creating a gateway to the web for the VPC
app_gateway = aws.ec2.InternetGateway("app-gateway", vpc_id=app_vpc.id)

app_routetable = aws.ec2.RouteTable(
    "app-routetable",
    routes=[
        aws.ec2.RouteTableRouteArgs(
            cidr_block="0.0.0.0/0",
            gateway_id=app_gateway.id,
        )
    ],
    vpc_id=app_vpc.id,
)

# Associating our gateway with our VPC, to allow our app to communicate with the greater internet
app_routetable_association = aws.ec2.MainRouteTableAssociation(
    "app_routetable_association", route_table_id=app_routetable.id, vpc_id=app_vpc.id
)

# Creating a Security Group that restricts incoming traffic to HTTP
sg_all_open = aws.ec2.SecurityGroup(
    "security-group",
    vpc_id=app_vpc.id,
    description="Enables all access",
    ingress=[
        aws.ec2.SecurityGroupIngressArgs(
            protocol="tcp",
            from_port=0,
            to_port=65535,
            cidr_blocks=["0.0.0.0/0"],
        )
    ],
    egress=[
        aws.ec2.SecurityGroupEgressArgs(
            protocol="-1",
            from_port=0,
            to_port=0,
            cidr_blocks=["0.0.0.0/0"],
        )
    ],
)



# Creating an IAM role used by Fargate to execute all our services
app_exec_role = aws.iam.Role(
    "app-exec-role",
    assume_role_policy="""{
        "Version": "2012-10-17",
        "Statement": [
        {
            "Action": "sts:AssumeRole",
            "Principal": {
                "Service": "ecs-tasks.amazonaws.com"
            },
            "Effect": "Allow",
            "Sid": ""
        }]
    }""",
)

# Attaching execution permissions to the exec role
exec_policy_attachment = aws.iam.RolePolicyAttachment(
    "app-exec-policy",
    role=app_exec_role.name,
    policy_arn=aws.iam.ManagedPolicy.AMAZON_ECS_TASK_EXECUTION_ROLE_POLICY,
)

# Creating an IAM role used by Fargate to manage tasks
app_task_role = aws.iam.Role(
    "app-task-role",
    assume_role_policy="""{
        "Version": "2012-10-17",
        "Statement": [
        {
            "Action": "sts:AssumeRole",
            "Principal": {
                "Service": "ecs-tasks.amazonaws.com"
            },
            "Effect": "Allow",
            "Sid": ""
        }]
    }""",
)

# Attaching execution permissions to the task role
task_policy_attachment = aws.iam.RolePolicyAttachment(
    "app-access-policy",
    role=app_task_role.name,
    policy_arn=aws.iam.ManagedPolicy.AMAZON_ECS_FULL_ACCESS,
)

############################
#TODO to parametrize
port_tracker_server = 3000
port_p2p_tracker_server = 15000


# Creating a Cloudwatch instance to store the logs that the ECS services produce
django_log_group = aws.cloudwatch.LogGroup(
    "p2p-infra-log-group", retention_in_days=1, name="p2p-infra-log-group"
)


def env_list(d: Dict[str, Any]) -> list[dict]:
    return [{"name": k, "value": v} for k, v in d.items()]

def make_sd_service(name: str):
    return aws.servicediscovery.Service(
        f"{name}-sd",
        name=name,  # el hostname será "<name>.p2p.local"
        dns_config=aws.servicediscovery.ServiceDnsConfigArgs(
            namespace_id=dns_ns.id,
            dns_records=[
                aws.servicediscovery.ServiceDnsConfigDnsRecordArgs(
                    ttl=5,
                    type="A",  # Fargate -> A records
                )
            ],
            routing_policy="MULTIVALUE",
        ),
        health_check_custom_config=aws.servicediscovery.ServiceHealthCheckCustomConfigArgs(
            failure_threshold=1,
        ),
    )


############################ Tracker task - service ############################
def make_service(name: str,
                 id_subnets: list[str],
                 id_security_groups: list[str],
                 availability_zone: str,
                 image_name: str,
                 cpu: str,
                 memory: int,
                 port_mappings: list,
                 env_vars: list,
                 logs_group: str,
                 depends_on: list[Any],
                 load_balancers: list[aws.ecs.ServiceLoadBalancerArgs],
                 sd_service: aws.servicediscovery.Service | None = None):
    # Creating a task definition for the second Django instance. This instance will
    # act as the server, and will run indefinately

    service_registries = None
    if sd_service is not None:
        service_registries = aws.ecs.ServiceServiceRegistriesArgs(
            registry_arn=sd_service.arn
            # opcional pero recomendable si tienes varios puertos:
            # port=port_mappings[0]["containerPort"],
        )

    task = aws.ecs.TaskDefinition(
        f"{name}-task-definition",
        family=f"{name}-task-definition-family",
        cpu=str(cpu),
        memory=str(memory),
        network_mode="awsvpc",
        requires_compatibilities=["FARGATE"],
        execution_role_arn=app_exec_role.arn,
        task_role_arn=app_task_role.arn,
        container_definitions=pulumi.Output.json_dumps(
            [
                {
                    "name": f"{name}-container",
                    "image": image_name,
                    "memory": memory,
                    "essential": True,
                    "portMappings": port_mappings,
                    "environment": env_vars,
                    "logConfiguration": {
                        "logDriver": "awslogs",
                        "options": {
                            "awslogs-group": logs_group,
                            "awslogs-region": availability_zone,
                            "awslogs-stream-prefix": "p2p-infra-site",
                        },
                    },
                    #   "command": [],
                }
            ]
        ),
        volumes=[],
    )

    # Launching our tracker server service on Fargate, using our configurations and load balancers
    svc = aws.ecs.Service(
        f"{name}-service",
        force_new_deployment=True,
        cluster=app_cluster.arn,
        desired_count=1,
        launch_type="FARGATE",
        task_definition=task.arn,
        wait_for_steady_state=False,
        network_configuration=aws.ecs.ServiceNetworkConfigurationArgs(
            assign_public_ip=True,
            subnets=id_subnets, # TODO CHECK THIS TO BE INCLUDED IN OUR VPC
            security_groups=id_security_groups,
        ),
        load_balancers=load_balancers,
        service_registries=service_registries,
        opts=pulumi.ResourceOptions(depends_on=depends_on),
    )
    return svc, task

mongo_sd = make_sd_service("mongo")
meili_sd = make_sd_service("meilisearch")
redis_sd = make_sd_service("redis")
frontend_sd = make_sd_service("frontend")


# tracker server operation
cpu="256"
memory=512
port_mappings=[{"containerPort": port_tracker_server, "protocol": "tcp"},
                    {"containerPort": port_p2p_tracker_server,
                     "protocol": "tcp"}]
env_vars=[]
image_name = tracker_server_image_name
name = "tracker-server"
logs_group = "p2p-infra-log-group"
subnets: list  = [app_vpc_subnet.id, app_vpc_subnet_b.id]
security_groups: list  = [sg_all_open.id]
depends_on = []
load_balancers: list[aws.ecs.ServiceLoadBalancerArgs] = []

make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory, port_mappings, env_vars, logs_group, depends_on, load_balancers)


name = 'mongo'
image_name = 'mongo:7'
cpu = '256'
memory = 512

port_mappings=[{"containerPort": 27017, "protocol": "tcp"}]
mongo_env=dict()
mongo_env.setdefault("MONGO_INITDB_ROOT_USERNAME", "root")
mongo_env.setdefault("MONGO_INITDB_ROOT_PASSWORD", "example")

depends_on = []
load_balancers = []

make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory,
             port_mappings, env_vars, logs_group, depends_on, load_balancers, mongo_sd)


name = 'meilisearch'
image_name = 'getmeili/meilisearch:v1.16'
cpu = '256'
memory = 1024
port_mappings=[{"containerPort": 7700, "protocol": "tcp"}]
meilisearch_env=dict()
depends_on = []
load_balancers = []
make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory
             , port_mappings, env_vars, logs_group, depends_on, load_balancers, meili_sd)

name = "redis"
image_name = "redis:7"
cpu = '256'
memory = 512
port_mappings=[{"containerPort": 6379, "protocol": "tcp"}]
redis_env=dict()
load_balancers = []
make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory
             , port_mappings, env_vars, logs_group, depends_on, load_balancers, redis_sd)

mongo_url = pulumi.Output.concat("mongodb://root:example@", "mongo.", dns_ns.name, ":27017")
meili_url = pulumi.Output.concat("http://", "meilisearch.", dns_ns.name, ":7700")
redis_url = pulumi.Output.concat("redis://", "redis.", dns_ns.name, ":6379/0")
frontend_url = pulumi.Output.concat("http://", "frontend.", dns_ns.name, ":8080")


name = "backend"
image_name = backend_image_name
cpu = '256'
memory = 1024
port_mappings=[{"containerPort": 3000, "protocol": "tcp"}]

# ALB
alb = aws.lb.LoadBalancer(
    "backend-alb",
    load_balancer_type="application",
    security_groups=security_groups,                 # tu SG único abierto
    subnets=subnets  # 2 AZs mínimo
)

backend_tg = aws.lb.TargetGroup(
    "backend-tg",
    port=3000, protocol="HTTP", target_type="ip", vpc_id=app_vpc.id,
    health_check=aws.lb.TargetGroupHealthCheckArgs(protocol="HTTP", path="/", port="traffic-port"),
)

listener3000 = aws.lb.Listener(
    "backend-http-3000",
    load_balancer_arn=alb.arn,
    port=3000, protocol="HTTP",
    default_actions=[aws.lb.ListenerDefaultActionArgs(type="forward", target_group_arn=backend_tg.arn)],
)

load_balancers = [aws.ecs.ServiceLoadBalancerArgs(
    target_group_arn=backend_tg.arn,
    container_name=f"{name}-container",
    container_port=3000,
)]

dns_name_backend  = pulumi.Output.concat("http://", alb.dns_name, ":3000")

backend_env=dict()
backend_env.setdefault("URL_FRONTEND", frontend_url)
backend_env.setdefault("URL_DB", mongo_url)
backend_env.setdefault("DATABASE_NAME", "db")
backend_env.setdefault("COLLECTION_NAME", "links")

backend_env.setdefault("URL_LLMDB", meili_url)
backend_env.setdefault("URL_QUEUE", redis_url)
backend_env.setdefault("RUST_LOG", "info")

make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory
             , port_mappings, env_list(backend_env), logs_group, depends_on, load_balancers)


name = "frontend"
image_name = frontend_image_name
cpu = '256'
memory = 1024
port_mappings=[{"containerPort": 8080, "protocol": "tcp"}]

frontend_alb = aws.lb.LoadBalancer(
    "frontend-alb",
    load_balancer_type="application",
    security_groups=security_groups,
    subnets=subnets,
)

frontend_tg = aws.lb.TargetGroup(
    "frontend-tg",
    port=8080, protocol="HTTP", target_type="ip", vpc_id=app_vpc.id,
)

listener8080 = aws.lb.Listener(
    "frontend-http-8080",
    load_balancer_arn=frontend_alb.arn,
    port=80, protocol="HTTP",
    default_actions=[aws.lb.ListenerDefaultActionArgs(
        type="forward", target_group_arn=frontend_tg.arn
    )],
)

# luego al crear el servicio frontend:
load_balancers = [aws.ecs.ServiceLoadBalancerArgs(
    target_group_arn=frontend_tg.arn,
    container_name="frontend-container",
    container_port=8080,
)]

frontend_env=dict()
frontend_env.setdefault("URL_BACKEND", dns_name_backend)
frontend_env.setdefault("URL_DB", mongo_url)
frontend_env.setdefault("DATABASE_NAME", "db")
frontend_env.setdefault("COLLECTION_NAME", "links")

frontend_env.setdefault("URL_LLMDB", meili_url)
frontend_env.setdefault("URL_QUEUE", redis_url)
frontend_env.setdefault("RUST_LOG", "info")

make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory
             , port_mappings, env_list(frontend_env), logs_group, depends_on, load_balancers, frontend_sd)

name = "pyagents"
image_name = pyagents_image_name
cpu = '256'
memory = 1024
port_mappings=[]
load_balancers = []

pyagents_env=dict()
pyagents_env.setdefault("URL_BACKEND", dns_name_backend)
pyagents_env.setdefault("URL_DB", mongo_url)
pyagents_env.setdefault("DATABASE_NAME", "db")
pyagents_env.setdefault("COLLECTION_NAME", "links")

pyagents_env.setdefault("URL_LLMDB", meili_url)
pyagents_env.setdefault("URL_QUEUE", redis_url)
pyagents_env.setdefault("RUST_LOG", "info")

make_service(name, subnets, security_groups, availability_zone, image_name, cpu, memory
             , port_mappings, env_list(pyagents_env), logs_group, depends_on, load_balancers)



pulumi.export("backend_public_url", dns_name_backend)

frontend_public_url = pulumi.Output.concat("http://", frontend_alb.dns_name)
pulumi.export("frontend_public_url", frontend_public_url)

pulumi.export(
    "mongo_url",
    mongo_url,
)
pulumi.export(
    "meili_url",
    meili_url,
)
pulumi.export(
    "redis_url",
    redis_url
)


