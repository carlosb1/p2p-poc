import threading
import time
import logging

from cli import P2PCLI, VALIDATION_LOOP_ACTIVE  # adapta el import a tu archivo
import bindings_p2p

# Configura logging a DEBUG
import os
os.environ["RUST_LOG"] = "info"

logging.basicConfig(level=logging.DEBUG)

# Variables compartidas
VALIDATION_LOOP_ACTIVE = True
content_to_test = "contenttotest"
topic = "topic1"
description = "tag"


def client_behavior(name, validate=False):
    cli = P2PCLI()
    cli.preloop()

    print(f"[{name}] ⬇️ Downloading connection data...")
    cli.do_download("")

    print(f"[{name}] 🚀 Starting client...")
    cli.do_start("")
    time.sleep(5)

    print(f"[{name}] 📝 Registering topic...")
    cli.do_register_topic(f"{topic} {description}")
    time.sleep(5)

    if validate:
        print(f"[{name}] 🧪 Validating content...")
        received_key = bindings_p2p.validate_content(topic, content_to_test)
        print(f"[{name}] ✅ Received key: {received_key}")


        print(f"[{name}] 🕒 Waiting 20s for gossip mesh + processing...")
        time.sleep(20)

        print("Voters")
        cli.do_voters(f"{received_key} {topic}")
        print("Pendings")
        cli.do_runtime_pending("")
        print("Available status")
        cli.do_get_status_voteses("")
        print("Reputations")
        cli.do_reputations(topic)

    print(f"[{name}] 💤 Keeping process alive to maintain mesh...")
    time.sleep(30)  # Mantén el proceso activo


from multiprocessing import Process



def test_full_flow():
    client1 = Process(target=client_behavior, args=("client1", True))
    clients = []
    for number_client in range(7):
        client = Process(target=client_behavior, args=(f"client{number_client}", False))
        clients.append(client)

    client1.start()
    time.sleep(5)
    for client in clients:
        client.start()

    time.sleep(5)

    client1.join()

    for client in clients:
        client.join()

    print("✅ Test complete.")

if __name__ == "__main__":
    test_full_flow()
