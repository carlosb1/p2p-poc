use crate::p2p::config::{load_config, BootstrapConfig, Config};
use crate::p2p::node::NetworkClientNode;
use libp2p::identity;
use messages_types::ChatCommand;
use protocol_p2p::client::ValidatorClient;
use protocol_p2p::db::init_db;
use protocol_p2p::handler::ValidatorHandler;
use protocol_p2p::models::db::{DataContent, Votation};
use protocol_p2p::models::messages::Vote;
use protocol_p2p::Db;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

pub const BUFFER_SIZE: usize = 32;

#[derive(Clone)]
pub struct APIClient {
    pub peer_id: libp2p::PeerId,
    pub db: Arc<Db>,
    validator_client: Arc<ValidatorClient>,
    pub node: Arc<Mutex<Option<NetworkClientNode<ValidatorHandler>>>>,
    tx: mpsc::Sender<ChatCommand>,
}

impl APIClient {
    pub fn new(keypair: identity::Keypair, name_peer: Option<String>) -> anyhow::Result<Self> {
        // load config from file
        let config = load_config(None)?;
        Self::inner_from_config(keypair, &config, name_peer)
    }

    pub fn from_server_params(
        keypair: identity::Keypair,
        name_peer: Option<String>,
        peer_id_server: &str,
        address: &str,
    ) -> anyhow::Result<Self> {
        log::debug!(
            "name_peer={:?} peer_id_server={} address={}",
            name_peer,
            peer_id_server,
            address
        );
        let config = Config {
            bootstrap: BootstrapConfig {
                peer_id: peer_id_server.to_string(),
                address: address.to_string(),
            },
        };
        Self::inner_from_config(keypair, &config, name_peer)
    }

    pub fn from_config(
        keypair: identity::Keypair,
        name_peer: Option<String>,
        path: Option<String>,
    ) -> anyhow::Result<Self> {
        if let Some(path) = path {
            let config = load_config(Some(path))?;
            Self::inner_from_config(keypair, &config, name_peer)
        } else {
            Self::new(keypair, name_peer)
        }
    }

    pub fn inner_from_config(
        keypair: identity::Keypair,
        config: &Config,
        name_peer: Option<String>,
    ) -> anyhow::Result<Self> {
        let peer_id = keypair.clone().public().to_peer_id();
        log::debug!("New peer id: {peer_id}");

        let name_to_initialize = match name_peer {
            Some(peer_id) => peer_id,
            None => peer_id.to_string(),
        };
        let db = Arc::new(init_db(name_to_initialize.as_str())?);

        let (tx, rx) = mpsc::channel::<ChatCommand>(BUFFER_SIZE); // save tx if needed outside
        let validator_client =
            ValidatorClient::new(peer_id, tx.clone(), db.clone(), keypair.clone());
        let validator_handler = ValidatorHandler::new(peer_id, db.clone());
        let node =
            NetworkClientNode::new(keypair.clone(), config, validator_handler, (tx.clone(), rx))?;

        Ok(Self {
            peer_id,
            db,
            validator_client: Arc::new(validator_client),
            node: Arc::new(Mutex::new(Some(node))),
            tx,
        })
    }

    pub fn sender(&self) -> mpsc::Sender<ChatCommand> {
        self.tx.clone()
    }

    pub async fn validate_content(
        &self,
        key: &str,
        topic: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        self.validator_client
            .ask_validation(key, topic, content)
            .await?;
        Ok(())
    }

    pub async fn remote_new_topic(&self, topic: &str) -> anyhow::Result<()> {
        self.validator_client.remote_new_topic(topic).await
    }

    pub fn new_key_for_content(&self, topic: &str, content: &str) -> anyhow::Result<String> {
        self.validator_client.new_key_available(topic, content)
    }

    pub async fn register_topic(&self, topic: &str) -> anyhow::Result<()> {
        self.validator_client.register_topic(topic).await
    }

    pub async fn add_vote(&self, id_votation: &str, topic: &str, vote: Vote) -> anyhow::Result<()> {
        self.validator_client
            .add_vote(id_votation, topic, vote)
            .await
    }

    pub async fn voters(&self, key: &str, topic: &str) -> anyhow::Result<Vec<String>> {
        self.validator_client.get_voters(key, topic)
    }

    pub fn get_reputation(&self, peer_id: &str, topic: &str) -> anyhow::Result<f32> {
        self.validator_client.get_reputation(peer_id, topic)
    }
    pub fn get_reputations(&self, topic: &str) -> Vec<(String, f32)> {
        self.validator_client.get_reputations(topic)
    }

    pub fn all_content(&self) -> Vec<DataContent> {
        self.validator_client.all_content()
    }

    pub fn get_status_vote(&self, key: &str) -> Option<Votation> {
        self.validator_client.get_status_vote(key)
    }

    pub async fn spawn_node(&self) -> tokio::task::JoinHandle<()> {
        let node_runner = self.node.clone();
        tokio::spawn(async move {
            let mut guard = node_runner.lock().await;
            let node = guard.as_mut().unwrap();
            node.run().await.expect("Network client node failed");
        })
    }

    pub async fn spawn_validator(&self) -> tokio::task::JoinHandle<()> {
        let client = self.clone().validator_client;
        tokio::spawn(async move {
            client
                .wait_for_validators()
                .await
                .expect("Validator client failed");
        })
    }

    pub async fn start(
        &self,
    ) -> anyhow::Result<(tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)> {
        let j_1 = self.spawn_node().await;
        let j_2 = self.spawn_validator().await;
        Ok((j_1, j_2))
    }
}
