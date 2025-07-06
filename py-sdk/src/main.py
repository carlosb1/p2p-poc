import cmd
import threading
import time
import bindings_p2p
import secrets
import hashlib

VALIDATION_LOOP_ACTIVE = True

server_id = None
server_address = None



def generate_random_hash():
    # Generate 32 random bytes
    random_bytes = secrets.token_bytes(32)

    # Create a SHA-256 hash of the random bytes
    hash_object = hashlib.sha256(random_bytes)
    return hash_object.hexdigest()

class P2PCLI(cmd.Cmd):
    intro = "🕸️  Welcome to the P2P CLI. Type help or ? to list commands.\n"
    prompt = "(p2p) "


    def preloop(self):
        # Start background validation thread
        self.validation_thread = threading.Thread(target=self.validation_loop, daemon=True)
        self.validation_thread.start()

    def do_exit(self, arg):
        """Exit the CLI"""
        global VALIDATION_LOOP_ACTIVE
        VALIDATION_LOOP_ACTIVE = False
        print("👋 Exiting...")
        return True

    def do_download(self, arg):
        """Download connection data"""
        try:
            data = bindings_p2p.download_connection_data()
            print("✅ Downloaded connection data:")
            print(data)
            global server_id
            server_id = data.server_id
            global server_address
            server_address = data.server_address[-1]
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_start(self, arg):
        """Start client: start <server_address> <peer_id> <username>"""
        parts = arg.strip().split()
        global server_address
        global server_id

        if len(parts) != 3 and not (server_address and server_id):
            print("Usage: start <server_address> <peer_id> <username>")
            return

        # and not server_id and not server_address:
        if len(parts) == 3:
            start_server_address, peer_id, username = parts
        else:
            start_server_address = server_address
            peer_id = server_id
            username = generate_random_hash()
        try:
            print("start_server_address:", start_server_address)
            print("peer_id:", peer_id)
            print("username:", username)
            connection_data = bindings_p2p.start(start_server_address, peer_id, username)
            print("🚀 Client started. connection_data={:?}", connection_data)
        except Exception as e:
            print(f"❌ Failed to start: {e}")

    def do_my_topics(self, arg):
        """Get your topics"""
        try:
            topics = bindings_p2p.get_my_topics()
            print("📚 Your topics:")
            for t in topics:
                print(f"- {t.name}: {t.description}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_register_topic(self, arg):
        """Register your topic: register_topic <topic> <description>"""
        parts = arg.strip().split()
        if len(parts) != 2:
            print("Usage: register_topic <topic> <description>")
            return
        name, description = parts
        try:
            bindings_p2p.register_topic(name, description)
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_remote_register_topic(self, arg):
        """Register remote topics: remote_register_topic <topic> <description>"""
        parts = arg.strip().split()
        if len(parts) != 2:
            print("Usage: remote_register_topic <topic> <description>")
            return
        name, description = parts
        try:
            bindings_p2p.remote_new_topic(name, description)
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_all_content(self, arg):
        """List all available content"""
        try:
            data = bindings_p2p.all_content()
            print("📦 All content:")
            for d in data:
                print(f"- [{d.approved}] {d.content} (votation: {d.id_votation})")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_vote(self, arg):
        """Add vote: vote <id_votation> <topic> <yes|no>"""
        parts = arg.strip().split()
        if len(parts) != 3:
            print("Usage: vote <id_votation> <topic> <yes|no>")
            return
        id_votation, topic, good = parts
        try:
            vote_obj = bindings_p2p.Vote(good=(good.lower() == "yes"))
            bindings_p2p.add_vote(id_votation, topic, vote_obj)
            print("🗳️ Vote submitted.")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_voters(self, arg):
        """Voters: voters <id_votation> <topic>"""
        parts = arg.strip().split()
        if len(parts) != 2:
            print("Usage: vote <id_votation> <topic>")
            return
        id_votation, topic = parts
        try:
           voters = bindings_p2p.voters(id_votation, topic)
           for voter in voters:
               print(f"- {voter}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_reputation(self, arg):
        """Get reputation: reputation <peer_id> <topic>"""
        parts = arg.strip().split()
        if len(parts) != 2:
            print("Usage: reputation <peer_id> <topic>")
            return
        peer_id, topic = parts
        try:
            rep = bindings_p2p.get_reputation(peer_id, topic)
            print(f"⭐ Reputation: {rep}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_reputations(self, arg):
        """Get reputation: reputations  <topic>"""
        parts = arg.strip().split()
        if len(parts) != 2:
            print("Usage: reputation <peer_id> <topic>")
            return
        peer_id, topic = parts
        try:
            rep = bindings_p2p.get_reputation(peer_id, topic)
            print(f"⭐ Reputation: {rep}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_get_status_voteses(self,arg):
        """Get status voteses: status_voteses"""
        try:
            voteses = bindings_p2p.get_status_voteses()
            for status_vote in voteses:
                print(f"-> {status_vote}")
        except Exception as e:
            print(f"❌ Error: {e}")
    def do_status_vote(self, arg):
        """Get status of vote: status_vote <votation_key>"""
        key = arg.strip()
        if not key:
            print("Usage: status_vote <key>")
            return
        try:
            result = bindings_p2p.get_status_vote(key)
            print(f"📊 Votation status: {result}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_runtime_pending(self, arg):
        """Show runtime content to validate"""
        try:
            pending = bindings_p2p.get_runtime_content_to_validate()
            if not pending:
                print("✅ No content to validate.")
                return
            for p in pending:
                print(f"- {p.topic}: {p.content} (timeout: {p.wait_timeout})")
        except Exception as e:
            print(f"❌ Error: {e}")

    def do_validate(self, arg):
        """Manually validate content: validate <key> <topic> <content>"""
        parts = arg.strip().split(maxsplit=2)
        if len(parts) != 2:
            print("Usage: validate <topic> <content>")
            return

        topic, content = parts
        try:
            key = bindings_p2p.validate_content(topic, content)
            print("🛡️ Content validated. ")
            print(f"- {key}")
        except Exception as e:
            print(f"❌ Error: {e}")

    def validation_loop(self):
        """Background loop to validate content automatically"""
        global VALIDATION_LOOP_ACTIVE
        print("🧠 Validator thread started.")
        while VALIDATION_LOOP_ACTIVE:
            try:
                pending = bindings_p2p.get_runtime_content_to_validate()
                for p in pending:
                    print(f"🔎 Auto-validating {p.topic}: {p.content}")
            except Exception as e:
                print(f"❌ It is not started {e}")
            time.sleep(10)  # Poll every 10s


if __name__ == '__main__':
    P2PCLI().cmdloop()
