import threading
from queue import Queue
from time import sleep, time
from typing import List

from bindings_p2p import bindings_p2p
from bindings_p2p.bindings_p2p import ClientWrapper, Vote

import threading
from models import DBConfig

TOPICS =  ["TOPIC4", "TOPIC5", "TOPIC6"]

def new_client(name_agent: str) -> ClientWrapper:
    data = bindings_p2p.download_connection_data()
    return ClientWrapper(data.server_address[-2], data.server_id, name_agent)

def vote_loop(agents, topic, stop_event: threading.Event):
    while not stop_event.is_set():  # check exit condition
        for agent in agents:
            contents = agent.get_my_pending_to_contents_to_validate()
            for content in contents:
                print(f"content to vote-{content}")
                id_votation = content.id_votation
                print(f"-{id_votation}")
                # TODO pending add topic in datacontent
                agent.add_vote(id_votation, topic, Vote(True))
        time.sleep(1)  # avoid busy loop

    print("Vote loop exiting cleanly")


def start_n_dummy_p2p_agents(num_agents_to_create: int, db_config: DBConfig, topic: str, stop_event = threading.Event()):
    # creating threads
    agents = []
    for num_agent in range(num_agents_to_create):
        # Using `args` to pass positional arguments and `kwargs` for keyword arguments
        name_agent = f"agent-{num_agent}"
        print(f"Creating agent {name_agent}")
        agent = agents.tools.p2p.new_client(name_agent)
        print(f"Registering topic {topic}")
        agent.register_topic(topic, topic)
        agents.append(agent)

    t = threading.Thread(target=vote_loop, args=(agents, topic,stop_event,))
    t.start()
    return t, agents