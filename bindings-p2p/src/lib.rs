mod dummy;

use messages_p2p::p2p::node::NetworkClientNode;
use messages_p2p::{Keypair, PeerId};
use std::collections::HashMap;

use messages_types::messages::ChatCommand;

use crate::dummy::MyEventHandler;
use crate::APIError::ConcurrencyError;
use messages_p2p::p2p::api::APIClient;
//use chrono::TimeZone;
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::sync::{Arc, OnceLock};
use std::thread;
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
    // Usa la variable de entorno si está presente
    let env = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());

    let _ = env_logger::Builder::new()
        .parse_filters(&env)
        .is_test(false)
        .try_init();

    log::info!("Logging initialized (level from RUST_LOG or default to info)");
}

pub struct Event {
    pub topic: String,
    pub message: String,
}
pub trait EventListener: Send + Sync {
    fn on_event(&self, event: Event) -> String;
}

const MAGIC_SERVER_LINK_ADDRESS: &str = "http://34.243.226.243:3000/tracker";

static LISTENER: OnceLock<Arc<dyn EventListener>> = OnceLock::new();
static DUMMY_NODE_TX: OnceLock<mpsc::Sender<ChatCommand>> = OnceLock::new();

static CLIENT: OnceLock<Mutex<APIClient>> = OnceLock::new();

// Creamos un runtime global
pub static TOKIO_RT: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("failed to create Tokio runtime"));

// Función que usa el runtime para correr async
pub fn block_on<F, R>(fut: F) -> R
where
    F: std::future::Future<Output = R> + Send + 'static,
    R: Send + 'static,
{
    std::thread::spawn(move || TOKIO_RT.block_on(fut))
        .join()
        .expect("Thread panicked during block_on execution")
}

#[derive(Debug, thiserror::Error)]
pub enum APIError {
    #[error("Error to connect to the server={addr} msg={msg}")]
    ConnectionError { addr: String, msg: String },
    #[error("Concurrency error msg={msg}")]
    ConcurrencyError { msg: String },
    #[error("RuntimeError msg={msg}")]
    RuntimeError { msg: String },
}
pub struct RuntimePendingContent {
    key: String,
    topic: String,
    content: String,
    wait_timeout: SystemTime,
}

pub struct Topic {
    pub name: String,
    pub description: String,
}

impl From<messages_p2p::Topic> for Topic {
    fn from(value: messages_p2p::Topic) -> Self {
        Topic {
            name: value.name,
            description: value.description,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionData {
    pub server_id: String,
    pub server_address: Vec<String>,
    pub client_id: Option<String>,
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

impl From<&messages_p2p::Votation> for Votation {
    fn from(temp_v: &messages_p2p::Votation) -> Self {
        let v = temp_v.clone();
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
pub struct Reputation {
    name: String,
    repu: f32,
}

#[derive(Debug, Clone)]
pub struct DataContent {
    pub id_votation: String,
    pub content: String,
    pub approved: StateContent,
}

#[derive(Debug, Clone)]
pub enum VoteStatusKind {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, Clone)]
pub struct VoteStatus {
    pub kind: VoteStatusKind,
    pub pending_data: Option<Vec<Pair>>,
}

#[derive(Debug, Clone)]
pub struct Pair {
    pub key: String,
    pub value: f32,
}

impl From<&messages_p2p::VoteStatus> for VoteStatus {
    fn from(status: &messages_p2p::VoteStatus) -> Self {
        match status {
            messages_p2p::VoteStatus::Pending(pairs) => VoteStatus {
                kind: VoteStatusKind::Pending,
                pending_data: Some(
                    pairs
                        .into_iter()
                        .map(|(k, v)| Pair {
                            key: k.to_string(),
                            value: *v,
                        })
                        .collect(),
                ),
            },
            messages_p2p::VoteStatus::Accepted => VoteStatus {
                kind: VoteStatusKind::Accepted,
                pending_data: None,
            },
            messages_p2p::VoteStatus::Rejected => VoteStatus {
                kind: VoteStatusKind::Rejected,
                pending_data: None,
            },
        }
    }
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

pub fn download_connection_data() -> ConnectionData {
    block_on(async {
        #[derive(Debug, Deserialize)]
        struct TrackerInfo {
            id: String,
            addresses: Vec<String>,
        }

        let response = reqwest::get(MAGIC_SERVER_LINK_ADDRESS).await.unwrap();
        let tracker_info: TrackerInfo = response.json().await.unwrap();
        ConnectionData {
            server_id: tracker_info.id,
            server_address: tracker_info.addresses.clone(),
            client_id: None,
        }
    })
}
pub fn start(
    server_address: String,
    server_peer_id: String,
    username: String,
) -> Result<ConnectionData, APIError> {
    block_on(async move {
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

        CLIENT.set(Mutex::new(node)).map_err(|_| ConcurrencyError {
            msg: "CLIENT not set".to_string(),
        })?;

        log::debug!(
            "Starting client with peer id: {:?} and server address: {:?}",
            peer_id,
            server_address
        );

        // ✅ Ejecutamos la tarea async en otro hilo
        let client = CLIENT.get().unwrap().clone();
        std::thread::spawn(move || {
            TOKIO_RT.block_on(async move {
                let mut guard = client.lock().await;
                if let Err(e) = guard.start().await {
                    log::error!("Client failed: {:?}", e);
                }
            });
        });

        Ok(ConnectionData {
            server_id: server_peer_id.clone(),
            server_address: vec![server_address.clone()],
            client_id: Some(peer_id.to_string()),
        })
    })
}

//TODO add name and description
pub fn remote_new_topic(name: String, description: String) -> Result<(), APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .remote_new_topic(&messages_p2p::Topic {
                name: name.clone(),
                description,
            })
            .await
            .map_err(|e| APIError::ConnectionError {
                addr: name.clone(),
                msg: e.to_string(),
            })
    })
}

pub fn register_topic(name: String, description: String) -> Result<(), APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .register_topic(&messages_p2p::Topic {
                name: name.clone(),
                description,
            })
            .await
            .map_err(|e| APIError::ConnectionError {
                addr: name.clone(),
                msg: e.to_string(),
            })
    })
}

pub fn get_my_topics() -> Vec<Topic> {
    block_on(async {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return vec![];
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .get_my_topics()
            .await
            .iter()
            .map(|t| Topic::from(t.clone()))
            .collect::<Vec<Topic>>()
    })
}

pub fn new_key_available(topic: String, content: String) -> Result<String, APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .new_key_available(&topic, &content)
            .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
    })
}

pub fn add_vote(id_votation: String, topic: String, vote: Vote) -> Result<(), APIError> {
    block_on(async move {
        let parser_vote = if vote.good {
            messages_p2p::Vote::Yes
        } else {
            messages_p2p::Vote::No
        };

        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .add_vote(&id_votation, &topic, parser_vote)
            .await
            .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
    })
}

pub fn voters(key: String, topic: String) -> Result<Vec<String>, APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .voters(&key, &topic)
            .await
            .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
    })
}

pub fn get_reputation(peer_id: String, topic: String) -> Result<f32, APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        locked
            .get_reputation(&peer_id, &topic)
            .map_err(|e| APIError::RuntimeError { msg: e.to_string() })
    })
}

pub fn get_reputations(topic: String) -> Vec<Reputation> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return vec![];
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        locked
            .get_reputations(&topic)
            .iter()
            .map(|(name, repu)| Reputation {
                name: name.clone(),
                repu: *repu,
            })
            .collect()
    })
}

pub fn all_content() -> Result<Vec<DataContent>, APIError> {
    block_on(async {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        let results = locked.all_content();
        Ok(results
            .iter()
            .map(|r| DataContent::from(r.clone()))
            .collect())
    })
}

pub fn validate_content(topic: String, content: String) -> Result<String, APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };

        let key = new_key_available(topic.clone(), content.clone())?;

        let client = mutex_client.clone();
        let mut locked = client.lock().await;
        locked
            .validate_content(&key, &topic, &content)
            .await
            .map_err(|e| APIError::RuntimeError { msg: e.to_string() })?;
        Ok(key.clone())
    })
}

pub fn get_status_vote(key: String) -> Result<Option<Votation>, APIError> {
    block_on(async move {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return Err(APIError::RuntimeError {
                msg: "Client is not initialized yet".to_string(),
            });
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        let result = locked.get_status_vote(&key);
        Ok(result.map(Votation::from))
    })
}

pub fn get_status_voteses() -> Vec<Votation> {
    block_on(async {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return vec![];
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        locked
            .get_status_voteses()
            .iter()
            .map(Votation::from)
            .collect()
    })
}

pub fn get_runtime_content_to_validate() -> Vec<RuntimePendingContent> {
    block_on(async {
        let Some(mutex_client) = CLIENT.get() else {
            log::debug!("Client is not initialized yet");
            return vec![];
        };
        let client = mutex_client.clone();
        let locked = client.lock().await;
        let results = locked
            .get_runtime_content_to_validate()
            .await
            .iter()
            .map(|(a, b, c, time)| RuntimePendingContent {
                key: a.clone(),
                topic: b.clone(),
                content: c.clone(),
                wait_timeout: SystemTime::UNIX_EPOCH + (*time),
            })
            .collect();
        results
    })
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread::sleep;
    use crate::{
        download_connection_data,
        start,
        remote_new_topic,
        register_topic,
        validate_content,
        voters,
        get_reputations,
        all_content,
    };

    #[test]
    fn full_client_flow() {
        let conn_data = download_connection_data();

        let start_result = start(
            conn_data.server_address.last().unwrap().clone(),
            conn_data.server_id.clone(),
            "rust_test_client".to_string(),
        );
        assert!(start_result.is_ok(), "❌ Error iniciando el cliente");

        let topic = "rust_test_topic2".to_string();
        let content = "contenido de prueba".to_string();

        let topic_result = remote_new_topic(topic.clone(), "un test topic".to_string());
        assert!(topic_result.is_ok(), "❌ Error creando tópico");

        sleep(Duration::from_secs(1));

        let reg_result = register_topic(topic.clone(), "".to_string());
        assert!(reg_result.is_ok(), "❌ Error registrando tópico");

        sleep(Duration::from_secs(1));

        let key_result = validate_content(topic.clone(), content.clone());
        assert!(key_result.is_ok(), "❌ Error validando contenido");
        let key = key_result.unwrap();

        sleep(Duration::from_secs(1));

        let voters_result = voters(key.clone(), topic.clone());
        assert!(voters_result.is_ok(), "❌ Error obteniendo votantes");
        println!("✅ Votantes: {:?}", voters_result.unwrap());

        let reputations = get_reputations(topic.clone());
        println!("📊 Reputaciones: {:?}", reputations);

         let content_result = all_content();
        assert!(content_result.is_ok(), "❌ Error obteniendo contenido");
        println!("📄 Contenido: {:?}", content_result.unwrap());

    
        println!("✅ Test de flujo completo finalizado con éxito");
        std::thread::sleep(std::time::Duration::from_secs(20));
    }
}

