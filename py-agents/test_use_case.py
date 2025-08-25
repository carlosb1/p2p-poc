import threading
import time
from queue import Queue

import agents
from agents.dummy_rss_p2p_worker import CLOSE_QUEUE_CMD, dummy_rss_p2p_agent
from agents.tools.p2p import start_n_dummy_p2p_agents
from models import DBConfig
from bindings_p2p import Vote


FEEDS = [
    # Ejemplos (cámbialos por tus medios preferidos)
    "https://news.google.com/rss/search?q=site:elpais.com&hl=es-419&gl=ES&ceid=ES:es",
#    "https://feeds.bbci.co.uk/news/world/rss.xml",
#    "https://www.reddit.com/r/worldnews/.rss",
]


def test_use_case():
    topic_to_publish = agents.tools.p2p.TOPICS[0]
    db_config = DBConfig.from_config()
    num_agents_to_create=8
    #agents are listening when they are started
    stop_event_for_agents = threading.Event()
    t, running_agents = start_n_dummy_p2p_agents(num_agents_to_create, db_config, topic_to_publish, stop_event_for_agents )
    print("Waiting for agents be running.")
    #we set up clients
    p2p_client = agents.tools.p2p.new_client("AGENT1")
    p2p_client.register_topic(topic_to_publish)

    print(f"Publishing topic {topic_to_publish}")
    q = Queue()
    t = threading.Thread(target=dummy_rss_p2p_agent, args=(q,"AGENT1",p2p_client.validate_content, topic_to_publish, ))
    t.start()
    time.sleep(1)
    q.put(FEEDS)
    print("Waiting for feeds...")
    time.sleep(10)
    q.put(CLOSE_QUEUE_CMD)
    t.join()
    print("Checking if they can validate")
    time.sleep(10)
    for val in p2p_client.get_runtime_content_to_validate():
        print(f'content_to_validate->{val}')
    time.sleep(5)
    stop_event_for_agents.set()
    for val in p2p_client.all_content():
        print(f'all_content->{val}')



if __name__ == "__main__":
    test_use_case()






