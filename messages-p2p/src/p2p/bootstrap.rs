use crate::p2p::behaviours::{
    build_gossipsub_behaviour, build_identify_behaviour, build_kademlia_behaviour,
    build_relay_behaviour,
};
use crate::p2p::config;
use crate::p2p::config::save_config;
use axum::response::sse::KeepAlive;
use futures::StreamExt;
use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStore;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{gossipsub, identify, identity, kad, noise, relay, tcp, yamux, PeerId, Swarm};

#[derive(NetworkBehaviour)]
struct BootstrapNodeBehaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
    relay: relay::Behaviour,
    identify: identify::Behaviour,
}

pub struct BootstrapServer {
    keypair: Keypair,
    peer_id: PeerId,
    swarm: Swarm<BootstrapNodeBehaviour>,
    listen_ons: Vec<String>,
}

impl BootstrapServer {
    pub async fn new(keypair: Keypair, listen_ons: Vec<String>) -> anyhow::Result<Self> {
        let peer_id = keypair.public().to_peer_id();

        log::info!("peer id for relay server {:?}", peer_id.to_string());

        // Build behaviours
        let mut gossipsub = build_gossipsub_behaviour(&keypair)?;
        gossipsub.subscribe(&config::DEFAULT_TOPIC)?;

        let behaviour = |key: &identity::Keypair| -> BootstrapNodeBehaviour {
            BootstrapNodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
                relay: build_relay_behaviour(key),
                identify: build_identify_behaviour(key),
            }
        };

        let swarm = libp2p::SwarmBuilder::with_existing_identity(keypair.clone())
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(behaviour)?
            .build();

        Ok(Self {
            keypair,
            peer_id,
            swarm,
            listen_ons,
        })
    }

    pub fn data_connection(&self) -> (String, Vec<String>) {
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
                    println!("Listening on {address:?} and saving server config");
                    save_config(&self.peer_id, address)?;
                }
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Relay(event)) => {
                    log::debug!("<UNK> Relay event: {event:?}");
                    for addr in self.swarm.external_addresses() {
                        if addr.to_string().contains("p2p-circuit") {
                            log::debug!("✅ Reachable via relay: {addr}");
                        }
                    }
                }
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Identify(
                    identify::Event::Received {
                        info: identify::Info { observed_addr, .. },
                        ..
                    },
                )) => {
                    log::debug!("<UNK> Identifying from: {observed_addr}");
                    self.swarm.add_external_address(observed_addr.clone());
                }
                /* event for messages */
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Gossipsub(
                    gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: id,
                        message,
                    },
                )) => {
                    println!(
                        "Server: Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    );
                }

                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(
                    kad::Event::InboundRequest { request },
                )) => {
                    println!("🔗 FIND NODE command detected");
                    println!("request={request:?}");
                }
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(
                    kad::Event::OutboundQueryProgressed { result, .. },
                )) => {
                    match result {
                        kad::QueryResult::GetProviders(Ok(
                            kad::GetProvidersOk::FoundProviders { key, providers, .. },
                        )) => {
                            for peer in providers {
                                println!(
                                    "Peer {peer:?} provides key {:?}",
                                    std::str::from_utf8(key.as_ref()).unwrap()
                                );
                            }
                        }
                        _ => {
                            println!("🔗 Other Kademlia event detected= {result:?}");
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
