use libp2p::{identity, PeerId};
use messages_p2p::p2p::bootstrap::BootstrapServer;
use messages_p2p::p2p::config::{load_config, BootstrapConfig, Config};
use messages_p2p::p2p::node::NetworkClientNode;
use messages_types::ChatCommand;
use protocol_p2p::client::ValidatorClient;
use protocol_p2p::db::init_db;
use protocol_p2p::handler::ValidatorHandler;
use protocol_p2p::models::messages::Vote;
use protocol_p2p::{db, Db};
use rand::distr::Alphanumeric;
use rand::{thread_rng, Rng};
use std::sync::Arc;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

pub const BUFFER_SIZE: usize = 32;

#[derive(Clone)]
pub struct TestClient {
    peer_id: libp2p::PeerId,
    db: Arc<Db>,
    validator_client: Arc<ValidatorClient>,
    pub node: Arc<Mutex<Option<NetworkClientNode<ValidatorHandler>>>>,
    tx: mpsc::Sender<ChatCommand>,
}
impl TestClient {
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

    pub fn all_content(&self) {
        self.validator_client.all_content()
    }

    pub fn get_status_vote(&self, key: &str) -> Option<String> {
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

    pub async fn spawn_validator(self) -> tokio::task::JoinHandle<()> {
        let client = self.validator_client;
        tokio::spawn(async move {
            client
                .wait_for_validators()
                .await
                .expect("Validator client failed");
        })
    }

    pub async fn start(
        self,
    ) -> anyhow::Result<(tokio::task::JoinHandle<()>, tokio::task::JoinHandle<()>)> {
        let j_1 = self.spawn_node().await;
        let j_2 = self.spawn_validator().await;
        Ok((j_1, j_2))
    }
}

pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Info)
        .try_init();
    log::info!("Logging initialized for Client");
}

#[tokio::test]
async fn validation_among_clients() {
    init_logging();
    let mut clients = vec![];

    for iden_peer in 0..2 {
        let name_peer = format!("peer_id{}", iden_peer);
        std::fs::remove_dir_all(name_peer.clone()).unwrap();
        let keypair = identity::Keypair::generate_ed25519();
        let client =
            TestClient::new(keypair, Some(name_peer)).expect("Failed to create test client");
        clients.push(client);
    }

    let client_asker = clients.first().unwrap().clone();

    // Lanzamos todos los nodos
    let mut join_handles = vec![];
    for client in &clients {
        let (t1_handler, t2_handler) = client
            .clone()
            .start()
            .await
            .expect("Failed to start client");
        join_handles.push(t1_handler);
        join_handles.push(t2_handler);
    }

    let topic_to_register = "chat-room";

    // Todos se registran al topic
    for client in &clients {
        client.register_topic(topic_to_register).await.unwrap();
        sleep(Duration::from_secs(1)).await
    }

    log::debug!("Confirming we are registered");
    // Envían mensajes
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    for (i, client) in clients.iter().enumerate() {
        let msg = format!("hello world {} {}", i, random_string);
        client
            .sender()
            .send(ChatCommand::Publish(
                topic_to_register.to_string(),
                msg.clone().into_bytes(),
            ))
            .await
            .unwrap();

        client
            .sender()
            .send(ChatCommand::Publish(
                "chat-room".to_string(),
                msg.into_bytes(),
            ))
            .await
            .unwrap();
    }

    // Solicita validación
    let key = client_asker
        .new_key_for_content(topic_to_register, "my second content")
        .unwrap();

    client_asker
        .validate_content(&key, topic_to_register, "my second content")
        .await
        .unwrap();

    // Observa estado durante 20s
    use std::time::{Duration, Instant};
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        println!("Tick at {:?}", start.elapsed());
        tokio::time::sleep(Duration::from_secs(1)).await;

        //     let status = client_asker.get_status_vote(&key).unwrap();
        //     println!("Status: {:?}", status);

        //     let reps = client_asker.get_reputations(topic_to_register);
        //      println!("Reputs: {:?}", reps);
    }

    println!("✅ Waiting 10 secs");

    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("✅ Done after 20 seconds");

    // Espera a que los tasks terminen (por limpieza)
    for handle in join_handles {
        let _ = handle.abort(); // Opcional: abortar o dejar vivir
    }
}

pub fn run_relay_server(
    keypair: identity::Keypair,
    listen_ons: Vec<String>,
    topics: Vec<String>,
    p2p_port: i32,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        match BootstrapServer::new(keypair, listen_ons, topics, p2p_port).await {
            Ok(mut server) => {
                if let Err(e) = server.run().await {
                    eprintln!("Server run failed: {e:?}");
                }
            }
            Err(e) => {
                eprintln!("Failed to start server: {e:?}");
            }
        }
    })
}

#[tokio::test]
async fn validation_among_clients_2() {
    init_logging();
    let mut clients = vec![];

    let p2p_port = 15000;
    let all_address = format!("/ip4/0.0.0.0/tcp/{p2p_port}").to_string();
    let loopback_address = format!("/ip4/127.0.0.1/tcp/{p2p_port}").to_string();

    let listen_ons = vec![all_address.clone(), loopback_address.clone()];
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = PeerId::from(keypair.public());

    /* topic */
    let topic_to_register = "topic1".to_string();
    /* Running relay server */
    run_relay_server(
        keypair,
        listen_ons,
        vec![topic_to_register.clone()],
        p2p_port,
    );
    sleep(Duration::from_secs(10)).await;
    //////////////////////////////

    for iden_peer in 0..7 {
        let name_peer = format!("peer_id{}", iden_peer);
        let _ = std::fs::remove_dir_all(name_peer.clone());
        let keypair = identity::Keypair::generate_ed25519();
        let client = TestClient::from_server_params(
            keypair,
            Some(name_peer),
            &peer_id.to_string(),
            loopback_address.clone().as_str(),
        )
        .unwrap();
        clients.push(client);
    }

    let client_asker = clients.first_mut().unwrap().clone();

    let mut join_handles = vec![];
    for client in clients.clone() {
        let (t1_handler, t2_handler) = client.start().await.expect("Failed to start client");
        join_handles.push(t1_handler);
        join_handles.push(t2_handler);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    client_asker
        .remote_new_topic(&topic_to_register)
        .await
        .unwrap();

    for client in clients.clone() {
        client
            .register_topic(topic_to_register.clone().as_str())
            .await
            .unwrap();
        sleep(Duration::from_secs(1)).await
    }

    log::debug!("Awaiting 10 secs getting new key");
    // Espera a que Gossipsub haga al menos un heartbeat y cree la malla
    tokio::time::sleep(Duration::from_secs(10)).await;
    let key = client_asker
        .new_key_for_content(topic_to_register.as_str(), "my second content")
        .unwrap();

    log::debug!("Awaiting 10 secs for validation");
    tokio::time::sleep(Duration::from_secs(10)).await;
    client_asker
        .validate_content(&key, topic_to_register.as_str(), "my second content")
        .await
        .unwrap();

    use std::time::Duration;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        println!("Tick at {:?}", start.elapsed());
        println!("key={:?}", key.clone());
        tokio::time::sleep(Duration::from_secs(1)).await;
        let voters = db::get_voters(&client_asker.db, &key, &topic_to_register);
        println!("voters: {:?}", voters);
        let status = client_asker.get_status_vote(&key);
        println!("Status: {:?}", status);
        let reps = client_asker.get_reputations(topic_to_register.as_str());
        println!("Reputs: {:?}", reps);
    }

    println!("All clients initialized, starting voting process...");

    for client in clients.clone() {
        println!(
            "-> Client {} {} is voting for key: {:?} and topic: {:?}",
            client.peer_id,
            client.peer_id.to_string(),
            key,
            topic_to_register
        );
        client
            .add_vote(&key, &topic_to_register, Vote::Yes)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    println!("✅ Waiting 10 secs");

    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("✅ Done after 20 seconds");

    for client in clients.clone() {
        println!("Validating contents in peer database...");
        for content in db::get_contents(&client.db) {
            println!("Validated content: {:?}", content);
        }
    }

    // Espera a que los tasks terminen (por limpieza)
    for handle in join_handles {
        let _ = handle.abort(); // Opcional: abortar o dejar vivir
    }
}
