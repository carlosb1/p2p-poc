use std::hash::{Hash, Hasher};
use std::sync::Arc;

use libp2p::identity;
use tokio::sync::{mpsc, Mutex};

use messages_types::ChatCommand;
use protocol_p2p::client::LinkClient;

use crate::db::init_db;
use crate::protocol::MessageHandler;

mod db;
mod models;
pub mod protocol;


#[tokio::main]
async fn main() {
    let topic = "topic:tech";
    let content = "hash_of_news_item_xyz";
    /* set up client */
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();
    let channel = mpsc::channel(32);
    let db = Arc::new(init_db(peer_id.to_string().as_str()).expect("Failed to initialize database"));
    let mut client = LinkClient::new(peer_id.clone(), channel.0, db.clone(), keypair.clone());
    /* registered for one topic*/
    client.register_topic(topic).await.expect("Failed to register topic");

    let key = client.new_key_available(topic, content).expect("Failed to create key for voting");

    // Create key for voting database
    client.ask_validation(&key, &topic, &content).await.expect("Failed to ask for validation");

    let shared_client = Arc::new(Mutex::new(client));
    tokio::spawn(async move {
        shared_client.lock().await.listen().await.unwrap();
    });
}
