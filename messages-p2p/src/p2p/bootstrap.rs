use std::path::PathBuf;
use futures::StreamExt;
use libp2p::{gossipsub, identity, kad, noise, PeerId, Swarm, tcp, yamux};
use libp2p::kad::Mode;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use once_cell::sync::Lazy;
use tokio::{io, select};
use tokio::io::AsyncBufReadExt;
use tracing_subscriber::EnvFilter;
use crate::p2p::behaviours::{build_gossipsub_behaviour, build_kademlia_behaviour};
use crate::p2p::config::{Config, load_config, print_config, save_config};
use crate::p2p::config;


static DEFAULT_LISTENS_ON: Lazy<Vec<String>> = Lazy::new(|| {
    vec!["/ip4/0.0.0.0/tcp/0", "/ip4/127.0.0.1/tcp/34291", "/ip4/0.0.0.0/tcp/34291"]
        .into_iter()
        .map(String::from)
        .collect()
});


#[derive(NetworkBehaviour)]
struct BootstrapNodeBehaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
}

fn handle_input_line(kademlia: &mut kad::Behaviour<MemoryStore>, line: String) {
    println!("line={:?}", line);
}


pub struct BootstrapServer {
    keypair: identity::Keypair,
    peer_id: PeerId,
    swarm: Swarm<BootstrapNodeBehaviour>,
    listen_ons: Vec<String>,
}

impl BootstrapServer {
    pub async fn new() -> anyhow::Result<Self> {
        let listen_ons = DEFAULT_LISTENS_ON.to_vec();
        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();

        log::info!("peer id for relay server {:?}", peer_id.to_string());


        // Build behaviours
        let mut gossipsub = build_gossipsub_behaviour(&keypair)?;
        gossipsub.subscribe(&config::DEFAULT_TOPIC)?;

        let behaviour = |key: &identity::Keypair| -> BootstrapNodeBehaviour {
            BootstrapNodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
            }
        };

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(behaviour)?
            .build();

        Ok(Self {
            keypair,
            peer_id,
            swarm,
            listen_ons
        })
    }

    pub fn data_connection(&self) -> (String, Vec<String> ) {
        let peer_id_str = self.peer_id.to_string();
        let listen_ons = self.listen_ons.clone();
        (peer_id_str, listen_ons)
    }
pub async fn run(&mut self) -> anyhow::Result<()> {

        /* setting up addresses */
        for addr in &self.listen_ons {
            self.swarm.listen_on(addr.parse()?)?;
        }

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    println!("Listening on {:?} and saving server config", address);
                    save_config(&self.peer_id, address)?;
                }
                /* event for messages */
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                                                                                 propagation_source: peer_id,
                                                                                 message_id: id,
                                                                                 message,
                                                                             })) => {
                    println!(
                        "Server: Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    );
                },

                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(kad::Event::InboundRequest { request })) => {
                    println!("🔗 FIND NODE command detected");
                    println!("request={:?}", request);
                },
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(kad::Event::OutboundQueryProgressed { result, ..})) => {
                    match result {
                        kad::QueryResult::GetProviders(Ok(kad::GetProvidersOk::FoundProviders { key, providers, .. })) => {
                            for peer in providers {
                                println!(
                                    "Peer {peer:?} provides key {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap()
                                );
                            }
                        },
                        _ => { println!("🔗 Other Kademlia event detected= {:?}", result); }
                    }
                }
                _ => {}
            }
        }
    }
}
