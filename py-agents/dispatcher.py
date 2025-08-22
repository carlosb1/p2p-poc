# dispatcher.py
import redis
import models
import subprocess
import json
import os

url = os.environ.get("URL_QUEUE", f"redis://{models.LOCAL_HOST_NAME}:6379/0")
r = redis.from_url(url)

print("Waiting tasks...")

while True:
    _, task_json = r.blpop("task_queue")
    task = json.loads(task_json)
    print(f"[Dispatcher] Received task: {task}")

    # Throws the process
    subprocess.Popen(["python", "agent_worker.py", json.dumps(task)])