use libp2p::identity::Keypair;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::Behaviour;
use libp2p::{gossipsub, identify, relay, request_response};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::time::Duration;

use libp2p::{request_response::json::Behaviour as JsonBehaviour, StreamProtocol};

pub fn build_gossipsub_behaviour(
    client_pair_keys: &Keypair,
) -> anyhow::Result<gossipsub::Behaviour> {
    let message_id_fn = |message: &gossipsub::Message| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        gossipsub::MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub configuration
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message
        // signing)
        //        .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
        .mesh_outbound_min(1)
        .mesh_n_low(1)
        .allow_self_origin(true)
        .build()
        .map_err(io::Error::other)?; // Temporary hack because `build` does not return a proper `std::error::Error`.

    // build a gossipsub network behaviour
    let gossipsub = gossipsub::Behaviour::new(
        gossipsub::MessageAuthenticity::Signed(client_pair_keys.clone()),
        gossipsub_config,
    )
    .map_err(anyhow::Error::msg)?; // Temporary hack because `new` does not return a proper `std::error::Error`.
    Ok(gossipsub)
}

pub fn build_kademlia_behaviour(key: &Keypair) -> Behaviour<MemoryStore> {
    Behaviour::new(
        key.public().to_peer_id(),
        MemoryStore::new(key.public().to_peer_id()),
    )
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OneToOneRequest {
    pub(crate) content: Vec<u8>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct OneToOneResponse {
    content: Vec<u8>,
}

pub fn build_request_response_behaviour() -> JsonBehaviour<OneToOneRequest, OneToOneResponse> {
    JsonBehaviour::<OneToOneRequest, OneToOneResponse>::new(
        [(
            StreamProtocol::new("/json-one-to-one"),
            request_response::ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    )
}

pub fn build_relay_behaviour(key: &Keypair) -> relay::Behaviour {
    relay::Behaviour::new(key.public().to_peer_id(), Default::default())
}

pub fn build_identify_behaviour(key: &Keypair) -> identify::Behaviour {
    identify::Behaviour::new(identify::Config::new(
        "/ipfs/id/1.0.0".to_string(),
        key.public(),
    ))
}
