use std::sync::Arc;

use libp2p::identity;
use tokio::sync::mpsc;

use protocol_p2p::{client::LinkClient, db::init_db};
use protocol_p2p::models::messages::ContentMessage;

#[tokio::test]
async fn test_content_validation_flow() {
    let topic = "test_topic";
    let content = "content_xyz";

    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();

    let (tx, _rx) = mpsc::channel(32);
    let db = Arc::new(init_db(peer_id.to_string().as_str()).expect("DB init failed"));

    let client = LinkClient::new(peer_id.clone(), tx, db.clone());
    client.register_topic(topic).await.unwrap();

    let key = protocol_p2p::db::create_key_for_voting_db(content, topic, "pending", 1);
    let msg = ContentMessage::Interested {
        id_votation: key,
        content: content.to_string(),
    };

    client.send(topic.to_string(), &msg).await.unwrap();
}
