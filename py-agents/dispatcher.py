# dispatcher.py
import threading

import redis

import models
import subprocess
import json
import os

from agents.tools.p2p import start_n_dummy_p2p_agents

url = os.environ.get("URL_QUEUE", f"redis://{models.LOCAL_HOST_NAME}:6379/0")
r = redis.from_url(url)

#setting mock agents
num_agents_to_create = 8
topic_to_publish = "mock"
db_config = models.DBConfig.from_config()
stop_event_for_agents = threading.Event()
# agents are listening when they are started
t, running_agents = start_n_dummy_p2p_agents(num_agents_to_create, db_config, topic_to_publish, stop_event_for_agents)
print("Waiting tasks...")

while True:
    _, task_json = r.blpop("task_queue")
    task = json.loads(task_json)
    print(f"[Dispatcher] Received task: {task}")

    # Throws the process
    subprocess.Popen(["python", "agent_worker.py", json.dumps(task)])