use crate::p2p::behaviours::{
    build_gossipsub_behaviour, build_identify_behaviour, build_kademlia_behaviour, build_relay_behaviour,
    build_request_response_behaviour, OneToOneRequest, OneToOneResponse,
};
use futures::StreamExt;
use libp2p::gossipsub::IdentTopic;
use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStore;
use libp2p::request_response::json::Behaviour as JsonBehaviour;
use libp2p::swarm::{NetworkBehaviour, SwarmEvent};
use libp2p::{gossipsub, identify, kad, noise, relay, tcp, yamux, Multiaddr, PeerId, Swarm};
use protocol_p2p::models::messages::{ContentMessage, DEFAULT_TOPIC};

#[derive(NetworkBehaviour)]
struct BootstrapNodeBehaviour {
    kademlia: kad::Behaviour<MemoryStore>,
    gossipsub: gossipsub::Behaviour,
    relay: relay::Behaviour,
    identify: identify::Behaviour,
    request_response: JsonBehaviour<OneToOneRequest, OneToOneResponse>,
}

pub struct BootstrapServer {
    keypair: Keypair,
    peer_id: PeerId,
    swarm: Swarm<BootstrapNodeBehaviour>,
    listen_ons: Vec<String>,
    external_multi_address: String,
    p2p_port: i32,
}

impl BootstrapServer {
    pub async fn new(
        keypair: Keypair,
        listen_ons: Vec<String>,
        topics: Vec<String>,
        p2p_port: i32,
    ) -> anyhow::Result<Self> {
        let peer_id = keypair.public().to_peer_id();

        log::info!("peer id for relay server {:?}", peer_id.to_string());

        // Build behaviours
        let mut gossipsub = build_gossipsub_behaviour(&keypair)?;
        gossipsub.subscribe(&DEFAULT_TOPIC)?;

        for topic in topics {
            log::info!("Subscribing to topic={:?}", topic);
            gossipsub.subscribe(&IdentTopic::new(topic.clone()))?;
        }
        let behaviour = |key: &Keypair| -> BootstrapNodeBehaviour {
            BootstrapNodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
                relay: build_relay_behaviour(key),
                identify: build_identify_behaviour(key),
                request_response: build_request_response_behaviour(),
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

        let address = reqwest::get("http://checkip.amazonaws.com")
            .await?
            .text()
            .await?
            .trim()
            .to_string();
        let external_multi_address = format!("/ip4/{}/tcp/{}", address, p2p_port);

        Ok(Self {
            keypair,
            peer_id,
            swarm,
            listen_ons,
            p2p_port,
            external_multi_address,
        })
    }

    pub fn data_connection(&self) -> (String, Vec<String>) {
        let peer_id_str = self.peer_id.to_string();
        let mut listen_ons = self.listen_ons.clone();
        listen_ons.push(self.external_multi_address.clone());
        let whole_address = format!(
            "{}/p2p/{}",
            self.external_multi_address,
            self.peer_id.to_string().as_str()
        );
        listen_ons.push(whole_address);
        (peer_id_str, listen_ons)
    }
    pub async fn run(&mut self) -> anyhow::Result<()> {
        /* setting up addresses */
        for addr in &self.listen_ons {
            self.swarm.listen_on(addr.parse()?)?;
        }

        /* checking IP */
        let manual_address = self.external_multi_address.parse::<Multiaddr>()?;
        log::debug!("listen on: {:?}", self.listen_ons);
        log::debug!("my external public IP: {}", manual_address);
        self.swarm.add_external_address(manual_address);

        loop {
            match self.swarm.select_next_some().await {
                SwarmEvent::NewListenAddr { address, .. } => {
                    log::debug!("Listening on {address:?} and saving server config");
                    //save_config(&self.peer_id, address)?;
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
                    log::debug!(
                        "Server: Got message: '{}' with id: {id} from peer: {peer_id}",
                        String::from_utf8_lossy(&message.data),
                    );

                    //parsing message

                    if let Ok(res) = serde_json::from_slice::<ContentMessage>(&message.data) {
                        log::debug!("Got message from peer: {res:?}");
                        if let ContentMessage::RegisterTopic { topic } = res {
                            log::debug!("Registering topic: {topic:?}");
                            let subscribed = self
                                .swarm
                                .behaviour_mut()
                                .gossipsub
                                .subscribe(&IdentTopic::new(topic))?;
                            log::debug!("Result topic subscribed: {subscribed:?}");
                        }
                    }
                }

                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(
                    kad::Event::InboundRequest { request },
                )) => {
                    log::debug!("🔗 FIND NODE command detected");
                    log::debug!("request={request:?}");
                }
                SwarmEvent::Behaviour(BootstrapNodeBehaviourEvent::Kademlia(
                    kad::Event::OutboundQueryProgressed { result, .. },
                )) => {
                    match result {
                        kad::QueryResult::GetProviders(Ok(
                            kad::GetProvidersOk::FoundProviders { key, providers, .. },
                        )) => {
                            for peer in providers {
                                log::debug!(
                                    "Peer {peer:?} provides key {:?}",
                                    std::str::from_utf8(key.as_ref())?
                                );
                            }
                        }
                        _ => {
                            log::debug!("🔗 Other Kademlia event detected= {result:?}");
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
