import time

import bindings_p2p
from bindings_p2p.bindings_p2p import ClientWrapper

TOPIC = "OTROMAS"
data = bindings_p2p.download_connection_data()
client = ClientWrapper(data.server_address[-1], data.server_id, "username11")
client.remote_new_topic(TOPIC, "")
time.sleep(3)
client.register_topic(TOPIC, "")
time.sleep(3)

agents = []
for i in range(9):
    agent = ClientWrapper(data.server_address[-1], data.server_id, "username" + str(i))
    time.sleep(3)
    agent.register_topic(TOPIC, "")
    agents.append(agent)

time.sleep(5)
key = client.validate_content(TOPIC, "my content")
print(key)

topics = client.get_my_topics()
print(topics)

voters = client.voters(key, TOPIC)
print(voters)
