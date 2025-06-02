# 🕸️ P2P Chat — Work in Progress

**P2P Chat** is a decentralized messaging system written in Rust using [`libp2p`](https://libp2p.io/). It is designed as an experimental playground to explore peer-to-peer protocols, message passing, and distributed system design.

> ⚠️ This project is a **Work in Progress** and not ready for production. It is shared to demonstrate system design skills, Rust expertise, and interest in decentralized communication technologies — useful for technical assessments or as part of a resume.

---

## 🔧 Features (WIP)

- ⚙️ **Rust-based architecture** with async I/O powered by `tokio`
- 🌐 **libp2p** for peer discovery, pub-sub, and direct messaging
- 💬 **Chat messaging system** with topic-based subscriptions
- 🧠 **Custom trait-based message handler architecture** for extensibility
- 💾 Optional **sled-based local storage** for persistence
- 🔒 Future plans include encryption, reputation handling, and consensus support

---

## 🧠 Why This Project?

This is part of a broader effort to:

- Deepen understanding of P2P networking fundamentals
- Experiment with decentralized message passing protocols
- Showcase advanced Rust development skills in real-world networking scenarios

It complements my resume by reflecting strong interest and capability in:

- Systems programming
- Distributed systems
- Protocol design
- Clean and modular Rust codebases

---

## 🛠️ Build & Run

```bash
# Clone the project
git clone https://github.com/your-username/p2p-chat.git
cd p2p-chat

# Build the project
cargo build

# Run a node (more instructions coming soon)
cargo run --bin node
