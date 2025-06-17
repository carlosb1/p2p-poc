"""An AWS Python Pulumi program"""

import pulumi
import pulumi_aws as aws

repo_tracker_server = aws.ecr.get_repository(name='p2p/tracker-server')
availability_zone = aws.config.region
tag_deploy='dev'

tracker_server_image = aws.ecr.get_image(
    repository_name=repo_tracker_server.name,
    image_tag=tag_deploy  # Ensure this tag exists in ECR
)

tracker_server_image_name = f"{repo_tracker_server.repository_url}@{tracker_server_image.image_digest}"
app_cluster = aws.ecs.Cluster("app-cluster")

# Creating a VPC and a public subnet
app_vpc = aws.ec2.Vpc("app-vpc", cidr_block="172.31.0.0/16", enable_dns_hostnames=True)

app_vpc_subnet = aws.ec2.Subnet(
    "app-vpc-subnet",
    cidr_block="172.31.0.0/20",
    availability_zone="eu-west-1a",
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
app_security_group = aws.ec2.SecurityGroup(
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


# The application's frontend: A Django service

# Creating a target group through which the Django frontend receives requests
tracker_server_targetgroup = aws.lb.TargetGroup(
    "t-server-targetgroup",
    port=port_tracker_server,
    protocol="TCP",
    target_type="ip",
    vpc_id=app_vpc.id,
)


# Creating a target group through which the Django frontend receives requests
server_targetgroup = aws.lb.TargetGroup(
    "p2p-server-targetgroup",
    port=port_p2p_tracker_server,
    protocol="TCP",
    target_type="ip",
    vpc_id=app_vpc.id,
)


############################ Django task - service ############################

# Creating a task definition for the second Django instance. This instance will
# act as the server, and will run indefinately
tracker_server_task_definition = aws.ecs.TaskDefinition(
    "tracker-server-task-definition",
    family="tracker-server-task-definition-family",
    cpu="256",
    memory="512",
    network_mode="awsvpc",
    requires_compatibilities=["FARGATE"],
    execution_role_arn=app_exec_role.arn,
    task_role_arn=app_task_role.arn,
    container_definitions=pulumi.Output.json_dumps(
        [
            {
                "name": "tracker-server-container",
                "image": tracker_server_image_name,
                "memory": 512,
                "essential": True,
                "portMappings": [{"containerPort": port_tracker_server, "hostPort": port_tracker_server, "protocol": "tcp"},
                                 {"containerPort": port_p2p_tracker_server, "hostPort": port_p2p_tracker_server, "protocol": "tcp"}],
                "environment": [
                    #   {"name": "SECRET_KEY", "value": django_secret_key},
                ],
                "logConfiguration": {
                    "logDriver": "awslogs",
                    "options": {
                        "awslogs-group": "p2p-infra-log-group",
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
tracker_server_service = aws.ecs.Service(
    "tracker-server-service",
    force_new_deployment=True,
    cluster=app_cluster.arn,
    desired_count=1,
    launch_type="FARGATE",
    task_definition=tracker_server_task_definition.arn,
    wait_for_steady_state=False,
    network_configuration=aws.ecs.ServiceNetworkConfigurationArgs(
        assign_public_ip=True,
        subnets=[app_vpc_subnet.id],
        security_groups=[app_security_group.id],
    ),
    opts=pulumi.ResourceOptions(depends_on=[]),
)
