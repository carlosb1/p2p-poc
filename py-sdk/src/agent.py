import hashlib
import secrets
import threading
import time
import os
import random

from src import bindings_p2p
from src.bindings_p2p import Vote

TOPIC = "test1111"


def generate_random_hash():
    # Generate 32 random bytes
    random_bytes = secrets.token_bytes(32)

    # Create a SHA-256 hash of the random bytes
    hash_object = hashlib.sha256(random_bytes)
    return hash_object.hexdigest()
import multiprocessing

class Agent(multiprocessing.Process):
    def __init__(self, id, command_queue, response_queue):
        super().__init__()

        connection_data = bindings_p2p.download_connection_data()
        username = generate_random_hash()
        self._peer_id = connection_data.server_id
        self._start_server_address = connection_data.server_address[-1]

        self._id = os.getpid()
        self._username = username
        self._stop_validation = False

        print(f'Downloaded = Connection data = {connection_data}')
        print(f'Username = {username}')
        self.validation_thread = threading.Thread(target=self.validation_loop, daemon=True)

        self.id = id
        self.command_queue = command_queue
        self.response_queue = response_queue

    def run(self):
        connection_data = bindings_p2p.start(self._start_server_address, self._peer_id, self._username)
        print(f'Connection data = {connection_data}')


        self.validation_thread.start()
        print('Started Validation Thread')
        print(f"[Agent-{self.id}] Starting")
        while True:
            cmd = self.command_queue.get()
            if cmd is None:
                print(f"[Agent-{self.id}] Stopping")
                break
            method, args = cmd
            if method == "register_topic":
                self.register_topic(*args)
            elif method == "ask_for_new_topic":
                self.ask_for_new_topic(*args)
            elif method == "ask_for_validation":
                result = self.ask_for_validation(*args)
                self.response_queue.put(result)
            elif method == "get_voters":
                voters = self.get_voters(*args)
                self.response_queue.put(voters)  # Dummy
            elif method == "reputations":
                reps = self.reputations(*args)
                self.response_queue.put(reps)
            elif method == "content":
                content = self.content()
                self.response_queue.put(content)

    def validation_loop(self):
        """Background loop to validate content automatically"""
        print(f"{self._username} = 🧠 Validator thread started.")
        while self._stop_validation:
            try:
                pending = bindings_p2p.get_runtime_content_to_validate()
                for p in pending:
                    print(f"{self._username} = 🔎 Pending: Auto-validating {p.topic}: {p.content}")
                    time.sleep(random.randint(1, 4))
                    if p.content.startswith("y"):
                        print(f"{self._username} = vote yes")
                        vote = Vote(good=True)
                    else:
                        print(f"{self._username} = vote no")
                        vote = Vote(good=False)
                    bindings_p2p.add_vote(p.key, p.topic, vote)
            except Exception as e:
                print(f"{self._username} = ❌ It is not started {e}")
            time.sleep(10)  # Poll every 10s

    def ask_for_new_topic(self, topic, description):
        bindings_p2p.remote_new_topic(topic, description)

    def register_topic(self, topic, description):
        bindings_p2p.register_topic(topic, description)

    def ask_for_validation(self, topic, content) -> str:
        return bindings_p2p.validate_content(topic, content)

    def get_voters(self, topic, key) -> list:
        return bindings_p2p.voters(key, topic)

    def content(self):
        return bindings_p2p.all_content()

    def reputations(self, topic) -> list:
        return bindings_p2p.get_reputations(topic)

# === Main ===
if __name__ == "__main__":
    agents = []
    queues = []

    for i in range(7):
        cmd_q = multiprocessing.Queue()
        res_q = multiprocessing.Queue()
        agent = Agent(i + 1, cmd_q, res_q)
        agent.start()
        agents.append((agent, cmd_q, res_q))
        time.sleep(3)

    # ask_for_new_topic and register_topic
    agents[0][1].put(("ask_for_new_topic", (TOPIC, "")))
    time.sleep(1)
    for _, cmd_q, _ in agents:
        cmd_q.put(("register_topic", (TOPIC, "")))
        time.sleep(1)

    # ask_for_validation
    agents[0][1].put(("ask_for_validation", (TOPIC, "yes content")))
    key = agents[0][2].get()
    print("Key:", key)
    time.sleep(20)

    # get_voters
    agents[0][1].put(("get_voters", (TOPIC, key)))
    voters = agents[0][2].get()
    print("Voters:", voters)

    # reputations
    agents[0][1].put(("reputations", (TOPIC,)))
    reps = agents[0][2].get()
    print("Reputations:", reps)

    # content
    agents[0][1].put(("content", ()))
    content = agents[0][2].get()
    print("Content:", content)

    # stop all
    for _, cmd_q, _ in agents:
        cmd_q.put(None)
    for proc, _, _ in agents:
        proc.join()

    print("finished to wait")
