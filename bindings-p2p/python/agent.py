import time

import bindings_p2p
from bindings_p2p import Vote
from bindings_p2p.bindings_p2p import ClientWrapper

TOPIC = "test"
data = bindings_p2p.download_connection_data()
client = ClientWrapper(data.server_address[-2], data.server_id, "username11")
# client.remote_new_topic(TOPIC, "")
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
import uuid

key = client.validate_content(TOPIC, "my content" + str(uuid.uuid4())[:9])
print(key)

topics = client.get_my_topics()
print(topics)

voters = client.voters(key, TOPIC)
print(voters)

for _ in range(0, 2):
    vals = client.get_runtime_content_to_validate()
    for val in vals:
        print(val)
    time.sleep(1)

time.sleep(30)

for agent in agents:
    print(agent)
    contents = agent.get_my_pending_to_contents_to_validate()
    for content in contents:
        print(content)
        id_votation = content.id_votation
        # TODO pending add topic in datacontent
        agent.add_vote(id_votation, TOPIC, Vote(True))
    time.sleep(1)

time.sleep(10)

# broadcast content to all
content = client.all_content()
for c in content:
    print(f"validated: {c}")
