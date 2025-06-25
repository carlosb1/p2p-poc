use std::collections::HashMap;
use std::sync::Arc;

use libp2p::identity;
use tokio::sync::{mpsc, Mutex};

use messages_types::ChatCommand;
use protocol_p2p::client::ValidatorClient;
use protocol_p2p::models::messages::ContentMessage;
use protocol_p2p::models::messages::Vote;

use crate::db::init_db;

mod db;
mod models;
pub mod protocol;

/// A simple mock server that relays messages to all subscribers by topic.
struct MockPubSubServer {
    subscribers: Mutex<HashMap<String, Vec<mpsc::Sender<Vec<u8>>>>>,
    command_rx: Mutex<mpsc::Receiver<ChatCommand>>,
}

impl MockPubSubServer {
    fn new(command_rx: mpsc::Receiver<ChatCommand>) -> Self {
        Self {
            subscribers: Mutex::new(HashMap::new()),
            command_rx: Mutex::new(command_rx),
        }
    }

    /// Registers a new subscriber for a topic
    pub async fn subscribe(&self, topic: &str, sender: mpsc::Sender<Vec<u8>>) {
        let mut subs = self.subscribers.lock().await;
        log::info!("New subscription {topic}");
        subs.entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(sender);
    }

    /// Publishes a message to all subscribers of a topic
    pub async fn publish(&self, topic: &str, message: Vec<u8>) {
        if let Some(subs) = self.subscribers.lock().await.get(topic) {
            for sub in subs {
                let _ = sub.try_send(message.clone());
            }
        }
    }

    /// Server task that listens to publish/subscribe commands
    pub async fn run(&self) {
        log::info!("MockPubSubServer is running...");
        let mut rx = self.command_rx.lock().await;
        while let Some(cmd) = rx.recv().await {
            match cmd {
                ChatCommand::Publish(topic, data) => {
                    self.publish(&topic, data).await;
                }
                ChatCommand::Subscribe(_) => {
                    log::debug!("Mocking subscribe command, not implemented in mock server.");
                }
                ChatCommand::SendOne(_, _) => {
                    //
                }
                ChatCommand::Quit => {
                    // handled externally by removing from subscribers
                }
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    log::info!("Starting Link Client...");
    let topic = "topic:agreement";
    let content = "news_abc_001";

    let (cmd_tx, cmd_rx) = mpsc::channel(64);
    let server = Arc::new(MockPubSubServer::new(cmd_rx));

    // Launch server loop
    let server_clone = server.clone();
    tokio::task::spawn(async move {
        server_clone.run().await;
    });

    let mut clients: Vec<Arc<ValidatorClient>> = Vec::new();
    let mut created_peer_ids = Vec::new();

    // Set up 6 clients
    let mut db_peers = Vec::new();
    for i in 0..6 {
        let cmd_cloned_tx = cmd_tx.clone();
        let server_clone = server.clone();

        let keypair = identity::Keypair::generate_ed25519();
        let peer_id = keypair.public().to_peer_id();
        let db = Arc::new(init_db(&format!("peer_{i}")).unwrap());
        db_peers.push(db.clone());
        let client = Arc::new(ValidatorClient::new(
            peer_id.clone(),
            cmd_cloned_tx,
            db.clone(),
            keypair.clone(),
        ));

        clients.push(client.clone());
        created_peer_ids.push(peer_id.clone());

        let _ = tokio::task::spawn(async move {
            let (tx, mut rx) = mpsc::channel(32);
            // the tricky part, client use tx from server and server the other tx to send to the client
            // Register client receiver as a subscriber
            server_clone.subscribe(topic, tx).await;
            client.register_topic(topic).await.unwrap();
            let handler = client.inner_handler.clone();
            let t_peer_id = client.peer_id().clone();
            log::info!("{:?} - Waiting for a new message", t_peer_id);
            while let Some(message) = rx.recv().await {
                log::info!(
                    "Client {} received message: {:?}",
                    t_peer_id.clone(),
                    String::from_utf8_lossy(&message)
                );
                if let Ok(_) = serde_json::from_slice::<ContentMessage>(&message) {
                    let response = handler
                        .lock()
                        .await
                        .handle_message(t_peer_id, &message, topic);
                    if let Some(response_message) = response {
                        client
                            .send(
                                topic.to_string(),
                                &serde_json::from_slice::<ContentMessage>(&response_message)
                                    .unwrap(),
                            )
                            .await
                            .expect("Failed to send message");
                    }
                } else {
                    log::warn!("Received invalid message format");
                }
            }
        });
    }

    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();

    let (tx, mut my_rx) = mpsc::channel::<Vec<u8>>(32);
    let db = Arc::new(init_db(&format!("myself_peer")).unwrap());

    log::info!("Myself peer ID: {}", peer_id.to_string());
    let myself = Arc::new(ValidatorClient::new(
        peer_id.clone(),
        cmd_tx.clone(),
        db.clone(),
        keypair.clone(),
    ));
    //created_peer_ids.push(peer_id.clone());

    // the tricky part, client use tx from server and server the other tx to send to the client
    // Register client receiver as a subscriber
    server.subscribe(topic, tx).await;
    myself.register_topic(topic).await.unwrap();

    /* listener for waiting validation, move to another function and do it sync? */
    let cloned_myself = myself.clone();
    let _ = tokio::task::spawn(async move {
        log::info!("Myself listening for messages...");
        cloned_myself
            .wait_for_validators()
            .await
            .expect("Failed to start listening");
    });

    /* running my handler */
    let cloned_myself = myself.clone();
    let _ = tokio::task::spawn(async move {
        //        let mut mocked_peer_ids = Vec::new();
        while let Some(message) = my_rx.recv().await {
            log::info!(
                "Client received message: {:?}",
                String::from_utf8_lossy(&message)
            );
            if let Ok(_) = serde_json::from_slice::<ContentMessage>(&message) {
                //get back the peer id from clients to resend the message
                if let Some(peer_id) = created_peer_ids.pop() {
                    cloned_myself
                        .inner_handler
                        .lock()
                        .await
                        .handle_message(peer_id, &message, topic);
                }
            } else {
                log::warn!("Received invalid message format");
            }
        }
    });
    let key = myself
        .new_key_available(topic, content)
        .expect("Failed to create key for voting");
    log::info!("Myself asking for validation");
    myself
        .ask_validation(&key.clone(), topic, content)
        .await
        .expect("Failed to ask for validation");
    log::info!("Waiting for all clients to initialize and listen...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    log::info!("--------------------------------------------------------------------");
    log::info!("All clients initialized, starting voting process...");

    for client in clients {
        log::info!(
            "-> Client {} is voting for key: {:?} and topic: {:?}",
            client.peer_id(),
            key,
            topic
        );
        client.add_vote(&key, &topic, Vote::Yes).await.unwrap()
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
    log::info!("--------------------------------------------------------------------");
    println!(
        "List status for each client for key={:?} and topic={:?}",
        key.clone(),
        topic
    );

    let myself_db = myself.db();
    let voters = db::get_voters(&myself_db, &key.clone(), topic).expect("Failed to get voters");

    assert!(voters.len() > 0, "No voters found for the key");
    for voter in voters {
        println!("Voter: {}", voter);
    }

    let reputations = db::get_reputations(&myself_db, topic);
    for reputation in reputations.iter() {
        println!("Reputation: {:?}", reputation);
    }
    assert!(reputations.len() > 0, "No voters found for the key");

    for content in db::get_contents(&myself_db) {
        println!("Validated content: {:?}", content);
    }

    for db in db_peers {
        println!("Validating contents in peer database...");
        for content in db::get_contents(&db) {
            println!("Validated content: {:?}", content);
        }
    }

    std::fs::remove_dir_all("myself_peer").unwrap();
    std::fs::remove_dir_all("peer_0").unwrap();
    std::fs::remove_dir_all("peer_1").unwrap();
    std::fs::remove_dir_all("peer_2").unwrap();
    std::fs::remove_dir_all("peer_3").unwrap();
    std::fs::remove_dir_all("peer_4").unwrap();
    std::fs::remove_dir_all("peer_5").unwrap();
}
