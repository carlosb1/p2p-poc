import hashlib
import secrets
import threading
import time
import os

from src import bindings_p2p

def generate_random_hash():
    # Generate 32 random bytes
    random_bytes = secrets.token_bytes(32)

    # Create a SHA-256 hash of the random bytes
    hash_object = hashlib.sha256(random_bytes)
    return hash_object.hexdigest()

class Agent:
    def __init__(self):
        connection_data = bindings_p2p.download_connection_data()
        username = generate_random_hash()
        peer_id = connection_data.server_id
        start_server_address = connection_data.server_address[-1]

        self._id = os.getpid()
        self._username = username
        self._stop_validation = False

        print(f'Downloaded = Connection data = {connection_data}')
        print(f'Username = {username}')


        connection_data = bindings_p2p.start(start_server_address, peer_id, username)
        print(f'Connection data = {connection_data}')

        self.validation_thread = threading.Thread(target=self.validation_loop, daemon=True)
        self.validation_thread.start()
        print('Started Validation Thread')


    def validation_loop(self):
        """Background loop to validate content automatically"""
        print(f"{self._username} = 🧠 Validator thread started.")
        while self._stop_validation:
            try:
                pending = bindings_p2p.get_runtime_content_to_validate()
                for p in pending:
                    print(f"{self._username} = 🔎 Pending: Auto-validating {p.topic}: {p.content}")

            except Exception as e:
                print(f"{self._username} = ❌ It is not started {e}")
            time.sleep(10)  # Poll every 10s



if __name__ == "__main__":
    agent = Agent() # start
    time.sleep(10)
