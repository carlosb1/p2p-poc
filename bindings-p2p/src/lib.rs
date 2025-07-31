mod types;

use messages_p2p::{Keypair, PeerId};

//use chrono::TimeZone;
use crate::types::{
    APIError, ConnectionData, DataContent, Reputation, RuntimePendingContent, Topic, Votation,
    Vote, VoteId,
};
use log::info;
use messages_p2p::p2p::api::APIClient;
use pyo3::{pyclass, pyfunction, pymethods};
use serde::Deserialize;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

use once_cell::sync::Lazy;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use tokio::runtime::Runtime;

static RUNTIME: Lazy<Runtime> =
    Lazy::new(|| Runtime::new().expect("Failed to create Tokio runtime"));

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

#[pymodule]
fn bindings_p2p(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<ClientWrapper>()?;
    m.add_function(wrap_pyfunction!(download_connection_data, m)?)?;
    Ok(())
}

#[pyclass]
pub struct ClientWrapper {
    client: Arc<tokio::sync::Mutex<APIClient>>,
    peer_id: PeerId,
    server_address: String,
    server_peer_id: String,
    username: String,
}

#[pymethods]
impl ClientWrapper {
    #[new]
    pub fn new_py(
        server_address: String,
        server_peer_id: String,
        username: String,
    ) -> PyResult<Self> {
        RUNTIME.block_on(async {
            let keypair = Keypair::generate_ed25519();
            let peer_id = PeerId::from(keypair.public());
            let peer_id_str = peer_id.to_string();

            let client = APIClient::from_server_params(
                keypair,
                Some(peer_id_str.clone()),
                &server_peer_id,
                &server_address,
            )
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;

            let wrapper = ClientWrapper {
                client: Arc::new(Mutex::new(client)),
                peer_id,
                server_address,
                server_peer_id,
                username,
            };

            {
                let mut guard = wrapper.client.lock().await;
                guard
                    .start()
                    .await
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            }

            Ok(wrapper)
        })
    }

    pub fn remote_new_topic(&self, name: String, description: String) -> PyResult<()> {
        RUNTIME.block_on(async {
            let topic = messages_p2p::Topic {
                name: name.clone(),
                description,
            };
            let mut guard = self.client.lock().await;
            guard
                .remote_new_topic(&topic)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    pub fn register_topic(&self, name: String, description: String) -> PyResult<()> {
        RUNTIME.block_on(async {
            let topic = messages_p2p::Topic {
                name: name.clone(),
                description,
            };
            let mut guard = self.client.lock().await;
            guard
                .register_topic(&topic)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(())
        })
    }

    pub fn get_my_topics(&self) -> PyResult<Vec<Topic>> {
        let topics = RUNTIME.block_on(async {
            let mut guard = self.client.lock().await;
            guard
                .get_my_topics()
                .await
                .iter()
                .cloned()
                .map(Topic::from)
                .collect::<Vec<Topic>>()
        });
        Ok(topics)
    }

    pub fn new_key_available(&self, topic: String, content: String) -> PyResult<String> {
        let topics = RUNTIME
            .block_on(async {
                let client = self.client.clone();
                let mut guard = client.lock().await;
                guard.new_key_available(&topic, &content)
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()));
        topics
    }

    pub fn add_vote(&self, id_votation: String, topic: String, vote: Vote) -> PyResult<()> {
        let vote = RUNTIME
            .block_on(async {
                let client = self.client.clone();
                let parser_vote = if vote.good {
                    messages_p2p::Vote::Yes
                } else {
                    messages_p2p::Vote::No
                };
                let mut locked = client.lock().await;
                let dis_vote = format!("{:?}", parser_vote);
                info!("add id_votation: {id_votation} topic: {topic}, vote: {dis_vote}");
                locked.add_vote(&id_votation, &topic, parser_vote).await
            })
            .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
        Ok(vote)
    }

    pub fn voters(&self, key: String, topic: String) -> PyResult<Vec<String>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let mut locked = client.lock().await;
            locked
                .voters(&key, &topic)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    pub fn get_reputation(&self, peer_id: String, topic: String) -> PyResult<f32> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let locked = client.lock().await;
            locked
                .get_reputation(&peer_id, &topic)
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))
        })
    }

    pub fn get_reputations(&self, topic: String) -> PyResult<Vec<Reputation>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let locked = client.lock().await;
            let reps = locked
                .get_reputations(&topic)
                .iter()
                .map(|(name, repu)| Reputation {
                    name: name.clone(),
                    repu: *repu,
                })
                .collect();
            Ok(reps)
        })
    }

    pub fn all_content(&self) -> PyResult<Vec<DataContent>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let locked = client.lock().await;
            let results = locked.all_content();
            Ok(results.iter().cloned().map(DataContent::from).collect())
        })
    }

    pub fn validate_content(&self, topic: String, content: String) -> PyResult<String> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let key = {
                let mut guard = client.lock().await;
                guard
                    .new_key_available(&topic, &content)
                    .map_err(|e| PyRuntimeError::new_err(e.to_string()))?
            };

            let mut guard = client.lock().await;
            guard
                .validate_content(&key, &topic, &content)
                .await
                .map_err(|e| PyRuntimeError::new_err(e.to_string()))?;
            Ok(key)
        })
    }

    pub fn get_status_vote(&self, key: String) -> PyResult<Option<Votation>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let locked = client.lock().await;
            let result = locked.get_status_vote(&key);
            Ok(result.map(Votation::from))
        })
    }

    pub fn get_status_voteses(&self) -> PyResult<Vec<Votation>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let locked = client.lock().await;
            Ok(locked
                .get_status_voteses()
                .iter()
                .cloned()
                .map(Votation::from)
                .collect())
        })
    }

    pub fn get_runtime_content_to_validate(&self) -> PyResult<Vec<RuntimePendingContent>> {
        let client = self.client.clone();
        RUNTIME.block_on(async {
            let results = client
                .lock()
                .await
                .get_runtime_content_to_validate()
                .await
                .iter()
                .map(|(a, b, c, time)| RuntimePendingContent {
                    key: a.clone(),
                    topic: b.clone(),
                    content: c.clone(),
                    wait_timeout: UNIX_EPOCH + (*time),
                })
                .collect();
            Ok(results)
        })
    }
}

const MAGIC_SERVER_LINK_ADDRESS: &str = "http://54.247.33.216:3000/tracker";

#[pyfunction]
pub fn download_connection_data() -> ConnectionData {
    #[derive(Debug, Deserialize)]
    struct TrackerInfo {
        id: String,
        addresses: Vec<String>,
    }
    let data = RUNTIME.block_on(async {
        let response = reqwest::get(MAGIC_SERVER_LINK_ADDRESS).await.unwrap();
        let tracker_info: TrackerInfo = response.json().await.unwrap();
        ConnectionData {
            server_id: tracker_info.id,
            server_address: tracker_info.addresses.clone(),
            client_id: None,
        }
    });
    return data;
}
