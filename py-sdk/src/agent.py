import threading
import time
import os

from src import bindings_p2p

class Agent:
    def __init__(self):
        self.validation_thread = threading.Thread(target=self.validation_loop, daemon=True)
        self.validation_thread.start()
        self._id = os.getpid()
        self._stop_validation = False

    def validation_loop(self):
        """Background loop to validate content automatically"""
        print(f"{self._id} = 🧠 Validator thread started.")
        while self._stop_validation:
            try:
                pending = bindings_p2p.get_runtime_content_to_validate()
                for p in pending:
                    print(f"{self._id} = 🔎 Pending: Auto-validating {p.topic}: {p.content}")
            except Exception as e:
                print(f"{self._id} = ❌ It is not started {e}")
            time.sleep(10)  # Poll every 10s

