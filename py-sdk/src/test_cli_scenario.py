import threading
import time
import logging

from cli import P2PCLI, VALIDATION_LOOP_ACTIVE  # adapta el import a tu archivo
import bindings_p2p

# Configura logging a DEBUG
import os
os.environ["RUST_LOG"] = "debug"

logging.basicConfig(level=logging.DEBUG)

# Variables compartidas
VALIDATION_LOOP_ACTIVE = True
content_to_test = "contenttotest"
topic = "examplechat"
description = "tag"

received_key = None


def client_behavior(name, validate=False):
    cli = P2PCLI()
    cli.preloop()

    print(f"[{name}] Downloading connection data...")
    cli.do_download("")

    print(f"[{name}] Starting client...")
    cli.do_start("")

    print(f"[{name}] Registering topic...")
    cli.do_register_topic(f"{topic} {description}")

    if validate:
        print(f"[{name}] Validating content...")
        key = bindings_p2p.validate_content(topic, content_to_test)
        print(f"[{name}] Received key: {key}")
        global received_key
        received_key = key


def voter_behavior():
    cli = P2PCLI()
    cli.preloop()
    time.sleep(3)  # Wait until content is validated

    if received_key:
        print(f"[voter] Getting voters...")
        cli.do_voters(f"{received_key} {topic}")
    else:
        print("❌ No key to use for voting.")


def test_full_flow():
    # Cliente 1: registra y valida
    client1 = threading.Thread(target=client_behavior, args=("client1", True))
    # Cliente 2: solo registra
    client2 = threading.Thread(target=client_behavior, args=("client2", False))

    client1.start()
    time.sleep(1)
    client2.start()

    client1.join()
    client2.join()

    # Lanzamos el que usa voters con la clave
    voter = threading.Thread(target=voter_behavior)
    voter.start()
    voter.join()

    # Terminamos el validador background
    global VALIDATION_LOOP_ACTIVE
    VALIDATION_LOOP_ACTIVE = False
    print("✅ Test complete.")


if __name__ == "__main__":
    test_full_flow()
