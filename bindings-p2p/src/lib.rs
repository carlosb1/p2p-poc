mod dummy;

use messages_p2p::p2p::node::NetworkClientNode;
use messages_p2p::{Keypair, PeerId};

use messages_types::messages::ChatCommand;

use crate::dummy::MyEventHandler;
use crate::APIError::ConcurrencyError;
use messages_p2p::p2p::api::APIClient;
use std::sync::{Arc, OnceLock};
use std::thread;
//use chrono::TimeZone;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::runtime::Runtime;
use tokio::sync::{mpsc, Mutex};
use uniffi::{export, generate_bindings};

#[cfg(target_os = "android")]
pub fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Trace),
    );
    log::info!("Logging initialized for Android");
}

#[cfg(not(target_os = "android"))]
pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Debug) // for testing
        .try_init();
    log::info!("Logging initialized for CLI/Desktop");
}

pub struct Event {
    pub topic: String,
    pub message: String,
}
pub trait EventListener: Send + Sync {
    fn on_event(&self, event: Event) -> String;
}

static LISTENER: OnceLock<Arc<dyn EventListener>> = OnceLock::new();
static DUMMY_NODE_TX: OnceLock<mpsc::Sender<ChatCommand>> = OnceLock::new();

static CLIENT: OnceLock<Mutex<APIClient>> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum APIError {
    #[error("Error to connect to the server={addr} msg={msg}")]
    ConnectionError { addr: String, msg: String },
    #[error("Concurrency error msg={msg}")]
    ConcurrencyError { msg: String },
    #[error("RuntimeError msg={msg}")]
    RuntimeError { msg: String },
}

struct ConnectionData {
    server_peer_id: String,
    server_address: String,
    client_peer_id: String,
}
pub struct Vote {
    good: bool,
}

#[derive(Debug, Clone)]
pub struct VoteId {
    pub key: String,
    pub value: f32,
}

#[derive(Debug, Clone)]
pub struct Votation {
    pub id_votation: String,
    pub timestam: SystemTime,
    pub content: String,
    pub status: String,
    pub leader_id: String,
    pub my_role: String,
    pub votes_id: Vec<VoteId>,
}

impl From<messages_p2p::Votation> for Votation {
    fn from(v: messages_p2p::Votation) -> Self {
        Self {
            id_votation: v.id_votation,
            timestam: UNIX_EPOCH
                + std::time::Duration::from_millis(v.timestamp.timestamp_millis() as u64),
            content: v.content,
            status: v.status,
            leader_id: v.leader_id,
            my_role: v.my_role,
            votes_id: v
                .votes_id
                .into_iter()
                .filter_map(|(k, opt)| opt.map(|v| VoteId { key: k, value: v }))
                .collect(),
        }
    }
}

#[derive(Debug, Clone)]
enum StateContent {
    Approved,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct DataContent {
    pub id_votation: String,
    pub content: String,
    pub approved: StateContent,
}
#[derive(Debug, Clone)]
pub struct Reputation {
    name: String,
    repu: f32,
}

impl From<messages_p2p::DataContent> for DataContent {
    fn from(d: messages_p2p::DataContent) -> Self {
        Self {
            id_votation: d.id_votation,
            content: d.content,
            approved: if d.approved == messages_p2p::StateContent::Approved {
                StateContent::Approved
            } else {
                StateContent::Rejected
            },
        }
    }
}

pub async fn start(
    server_address: String,
    server_peer_id: String,
    username: String,
) -> Result<(), APIError> {
    init_logging();
    let keypair = Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    let node = APIClient::from_server_params(
        keypair,
        Some(peer_id.to_string()),
        &server_peer_id,
        &server_address,
    )
    .map_err(|e| APIError::ConnectionError {
        addr: server_address.clone(),
        msg: e.to_string(),
    })?;

    // Guardamos el cliente sincronizado
    CLIENT.set(Mutex::new(node)).map_err(|_| ConcurrencyError {
        msg: "CLI not set".to_string(),
    })?;

    log::debug!(
        "Starting client with peer id: {:?} and server address: {:?}",
        peer_id,
        server_address
    );

    // Lanzamos el loop `start()` como tarea de fondo
    let client = CLIENT.get().unwrap().clone();
    tokio::spawn(async move {
        let mut guard = client.lock().await;
        if let Err(e) = guard.start().await {
            log::error!("Client failed: {:?}", e);
        }
    });

    Ok(())
}

pub async fn remote_new_topic(topic: String) -> Result<(), APIError> {
    let client = CLIENT.get().unwrap().clone();
    let mut locked = client.lock().await;
    locked
        .remote_new_topic(&topic)
        .await
        .map_err(|e| APIError::ConnectionError {
            addr: topic.clone(),
            msg: e.to_string(),
        })
}

pub async fn register_topic(topic: String) -> Result<(), APIError> {
    let client = CLIENT.get().unwrap().clone();
    let mut locked = client.lock().await;
    locked
        .register_topic(&topic)
        .await
        .map_err(|e| APIError::ConnectionError {
            addr: topic.clone(),
            msg: e.to_string(),
        })
}

pub async fn validate_content(key: String, topic: String, content: String) -> Result<(), APIError> {
    let client = CLIENT.get().unwrap().clone();
    let mut locked = client.lock().await;
    locked
        .validate_content(&key, &topic, &content)
        .await
        .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
}

pub async fn all_content() -> Result<Vec<DataContent>, APIError> {
    let client = CLIENT.get().unwrap().clone();
    let locked = client.lock().await;
    let results = locked.all_content();
    Ok(results
        .iter()
        .map(|r| DataContent::from(r.clone()))
        .collect())
}

pub async fn get_status_vote(key: String) -> Result<Option<Votation>, APIError> {
    let client = CLIENT.get().unwrap().clone();
    let locked = client.lock().await;
    let result = locked.get_status_vote(&key);
    Ok(result.map(Votation::from))
}

pub async fn add_vote(id_votation: String, topic: String, vote: Vote) -> Result<(), APIError> {
    let parser_vote = if vote.good {
        messages_p2p::Vote::Yes
    } else {
        messages_p2p::Vote::No
    };

    let client = CLIENT.get().unwrap().clone();
    let mut locked = client.lock().await;
    locked
        .add_vote(&id_votation, &topic, parser_vote)
        .await
        .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
}

pub async fn voters(key: String, topic: String) -> Result<Vec<String>, APIError> {
    let client = CLIENT.get().unwrap().clone();
    let mut locked = client.lock().await;
    locked
        .voters(&key, &topic)
        .await
        .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
}

pub async fn get_reputation(peer_id: String, topic: String) -> Result<f32, APIError> {
    let client = CLIENT.get().unwrap().clone();
    let locked = client.lock().await;
    locked
        .get_reputation(&peer_id, &topic)
        .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
}

pub async fn get_reputations(topic: String) -> Vec<Reputation> {
    let client = CLIENT.get().unwrap().clone();
    let locked = client.lock().await;
    locked
        .get_reputations(&topic)
        .iter()
        .map(|(name, repu)| Reputation {
            name: name.clone(),
            repu: *repu,
        })
        .collect()
}

pub fn dummy_start(server_address: String, peer_id: String, username: String) {
    init_logging();

    /* client initialization */
    let keypair = Keypair::generate_ed25519();

    /* server config */
    let bootstrap_config = messages_p2p::p2p::config::BootstrapConfig {
        peer_id: peer_id.clone(),
        address: server_address.clone(),
    };
    let config = messages_p2p::p2p::config::Config {
        bootstrap: bootstrap_config,
    };
    let handler = MyEventHandler::default();
    log::debug!(
        "Starting process for running service to connect to the network with peer id: {:?} \
    and server address: {:?}",
        peer_id.clone(),
        server_address.clone()
    );
    thread::spawn(move || {
        if let Err(e) = std::panic::catch_unwind(|| {
            log::debug!("Creating Tokio runtime for node");
            let rt = Runtime::new().expect("Failed to create Tokio runtime");
            rt.block_on(async move {
                log::debug!("Setting blocking code and node tx");
                let (tx, rx) = mpsc::channel::<ChatCommand>(32);
                let mut node = NetworkClientNode::new(keypair, &config, handler, (tx, rx))
                    .expect("Failed to create node");
                if DUMMY_NODE_TX.get().is_none() {
                    DUMMY_NODE_TX
                        .set(node.command_sender())
                        .expect("Failed to set NODE_TX");
                    log::debug!("NODE_TX initialized");
                } else {
                    log::warn!("NODE_TX was already initialized — skipping set");
                }
                node.run().await.expect("Node run failed");
            });
        }) {
            log::error!("Thread panicked: {:?}", e);
        }
    });
}

pub fn dummy_set_listener(listener: Arc<dyn EventListener>) {
    let _ = LISTENER.set(listener);
}

pub fn dummy_raw_message(topic: String, message: String) {
    log::info!("Sending message: {} to topic: {}", message, topic);

    if let Some(tx) = DUMMY_NODE_TX.get() {
        let tx = tx.clone();
        thread::spawn(move || {
            if let Err(e) = std::panic::catch_unwind(move || {
                let rt = Runtime::new().expect("Failed to create Tokio runtime");
                rt.block_on(async move {
                    let cmd = ChatCommand::Publish(topic.clone(), message.clone().into_bytes());
                    if let Err(e) = tx.send(cmd).await {
                        log::error!("Failed to send command: {:?}", e);
                    } else {
                        log::info!("Message sent successfully!");
                    }
                });
            }) {
                log::error!("Thread panicked while sending: {:?}", e);
            }
        });
    } else {
        log::warn!("NODE_TX not initialized. Did you forget to call `start()`?");
    }
}

uniffi::include_scaffolding!("bindings_p2p");
