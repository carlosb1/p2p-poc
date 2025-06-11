use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use libp2p::{identity, PeerId, Multiaddr, gossipsub::{self, IdentTopic as Topic}, kad::{self, store::MemoryStore}, noise, tcp, yamux, swarm::{Swarm, SwarmEvent, NetworkBehaviour}, request_response};
use tokio::sync::{mpsc, Mutex};
use futures::StreamExt;
use rand::Rng;
use crate::p2p::behaviours::{OneToOneRequest, OneToOneResponse};
use libp2p::request_response::json::{Behaviour as JsonBehaviour};
use crate::p2p::config::{Config, load_config, print_config, DEFAULT_TOPIC};
use crate::p2p::behaviours::{build_gossipsub_behaviour, build_kademlia_behaviour, build_request_response_behaviour};
use protocol_p2p::MessageHandler;
use messages_types::ChatCommand;


const BUFFER_SIZE: usize = 32;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub kademlia: kad::Behaviour<MemoryStore>,
    pub gossipsub: gossipsub::Behaviour,
    pub request_response: JsonBehaviour::<OneToOneRequest, OneToOneResponse>,
}

pub struct ClientNode<H: MessageHandler> {
    peer_id: PeerId,
    swarm: Swarm<NodeBehaviour>,
    command_rx: mpsc::Receiver<ChatCommand>,
    command_tx: mpsc::Sender<ChatCommand>,
    handler: H
}

pub fn generate_rand_msg() -> String {
    let mut rng = rand::rng();
    let random_number: u32 = rng.random_range(0..10000);
    format!("Random message: {}", random_number)
}

#[derive(Debug, Clone, Default)]
pub struct SimpleClientHandler;

// returns topic, and str message
impl MessageHandler for SimpleClientHandler {
    fn handle_message(&mut self, peer: PeerId, data: &[u8], topic: &str) -> Option<Vec<u8>>{
        let str_message = String::from_utf8_lossy(data).to_string();
        if str_message.contains("hello world")  {
            log::debug!("Node: received hello world message from {}", peer);
            let random_msg = generate_rand_msg();
            let ret_msg = format!("Hello, world {:?}", random_msg);
            log::debug!("Node: sending back message: {:?}", ret_msg.clone());
            return Some( ret_msg.into_bytes());
        }
        None
    }
}



pub fn run_node() -> anyhow::Result<Arc<Mutex<ClientNode<SimpleClientHandler>>>> {
    let handler = SimpleClientHandler::default();
    let config = load_config(None)?;

    let mut node = Arc::new(Mutex::new(ClientNode::new(config, handler)?));

    // Spawn the background node task
    let node_runner = node.clone();
    tokio::spawn(async move {
        node_runner
            .lock()
            .await
            .run()
            .await
            .expect("Background node failed");
    });
    Ok(node.clone())
}

impl<H: MessageHandler> ClientNode<H>{
    pub fn from_config(node_config: Config, handler: H) -> anyhow::Result<Self> {
        // bootstrap info
        let server_peer_id: PeerId = node_config.bootstrap.peer_id.parse()?;
        let server_addr: Multiaddr = node_config.bootstrap.address.parse()?;

        log::info!("Server node config: ");
        print_config(&server_peer_id, Some(&server_addr), None);

        // client identity
        let client_keypair = identity::Keypair::generate_ed25519();
        let client_peer_id = client_keypair.public().to_peer_id();

        let mut gossipsub = build_gossipsub_behaviour(&client_keypair)?;
        gossipsub.subscribe(&DEFAULT_TOPIC)?;


        let behaviour = |key: &identity::Keypair| {
            Ok(NodeBehaviour {
                kademlia: build_kademlia_behaviour(key),
                gossipsub,
                request_response: build_request_response_behaviour()
            })
        };

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(client_keypair.clone())
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(|key| behaviour(key))?
            .build();

        swarm.behaviour_mut().kademlia.add_address(&server_peer_id, server_addr.clone());
        swarm.dial(server_addr.clone())?;
        swarm.behaviour_mut().kademlia.bootstrap()?;

        let (tx, rx) = mpsc::channel::<ChatCommand>(BUFFER_SIZE); // save tx if needed outside

        Ok(Self {
            peer_id: client_peer_id,
            swarm,
            command_rx: rx,
            command_tx: tx.clone(),
            handler
        })
    }


    pub fn from_config_path(path: String, handler: H) -> anyhow::Result<Self> {
        let node_config = load_config(Some(path))?;
        Self::from_config(node_config, handler)
    }
    pub fn new(node_config: Config, handler: H) -> anyhow::Result<Self> {
         Self::from_config(node_config, handler)
    }

    pub fn command_sender(&self) -> mpsc::Sender<ChatCommand> {
        self.command_tx.clone()
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        let peer_id_str = self.peer_id.to_string();

        loop {
            tokio::select! {
                /* manage commands to call */
                Some(cmd) = self.command_rx.recv() => {
                    match cmd {
                        ChatCommand::Subscribe(topic_name) => {
                            let topic = Topic::new(topic_name);
                            if self.swarm.behaviour_mut().gossipsub.subscribe(&topic).is_ok() {
                                log::info!("✅ Subscribed to topic: {topic}");
                            }
                        }
                        ChatCommand::Publish(topic_name, msg) => {
                            let topic = Topic::new(topic_name);
                            log::info!("🟢 Publishing: {} with topic {:?}", String::from_utf8_lossy(&msg), topic.clone());
                            if self.swarm.behaviour_mut().gossipsub.publish(topic.clone(), msg).is_ok() {
                                log::info!("📤 Published to topic: {topic}");
                            }
                        },
                        ChatCommand::SendOne(peer_id, msg) => {
                            log::info!("🟢 Sending one-to-one message: {} to peer: {peer_id}", String::from_utf8_lossy(&msg));
                            self.swarm.behaviour_mut().request_response.send_request(
                                &PeerId::from_str(&peer_id).map_err(|e| anyhow::Error::msg(e))?,
                                OneToOneRequest {
                                    content: msg
                                }
                            );
                        },
                        ChatCommand::Quit => {
                            log::info!("👋 Quitting the node: {peer_id_str}");
                            return Ok(());
                        }
                        other => {
                            log::debug!("❌ Command not supported {:?}", other);
                        }
                    }
                }
                event = self.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            log::debug!("🧩 Listening on: {:?}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            log::debug!("✅ Connected to: {peer_id}");
                        }
                        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message { propagation_source, message_id, message })) => {
                            //propagation source is peer origin of the message
                            log::debug!("📬 Received message from the forwarded {propagation_source} with id: {message_id}");
                            log::debug!("📜 Message with topic {:?} and data: {:?}",
                                &message.topic,
                                String::from_utf8_lossy(&message.data.clone()));

                            // TODO check source
                            //let source_peer = message.source;
                            match message.source {
                                Some(source) => {
                                    log::debug!("📬 Message source: {source} with message_id={message_id}");
                                    let response_command = self.handler.handle_message(source,&message.data.clone(), &message.topic.to_string());
                                    //If has to handle the message with another message, you can send it
                                    if let Some(command) = response_command {
                                        if let Err(er) = self.command_tx.send(ChatCommand::Publish((&message.topic.clone()).to_string(), command)).await {
                                            log::error!("❌ Failed to send command: {er}");;
                                        }
                                    } else {
                                        log::info!(
                                            "📨 Got message: '{}' from {propagation_source} (id: {message_id})",
                                            String::from_utf8_lossy(&message.data.clone())
                                        );
                                    }

                                },
                                None => {
                                    log::debug!("📬 Message has no source for id_message={message_id}");
                                }
                            }


                        }
                        SwarmEvent::Behaviour(NodeBehaviourEvent::Kademlia(event)) => {
                            log::debug!("🧠 Kademlia event: {:?}", event);
                        }
                        other => {
                            log::debug!("🔎 Other event: {:?}", other);
                        }
                    }
                }
            }
        }
    }
}
