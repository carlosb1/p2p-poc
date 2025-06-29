use messages_p2p::PeerId;
use protocol_p2p::MessageHandler;
use crate::{Event, LISTENER};

#[derive(Clone, Debug, Default)]
pub struct MyEventHandler;

impl MessageHandler for MyEventHandler {
    fn handle_message(&mut self, peer: PeerId, data: &[u8], topic: &str) -> Option<Vec<u8>> {
        log::info!(
            "Node: received message from {}: {:?}",
            peer.clone(),
            String::from_utf8_lossy(data.clone())
        );
        match LISTENER.get() {
            Some(listener) => {
                let msg = String::from_utf8_lossy(data).to_string();
                let event = Event {
                    topic: topic.to_string(),
                    message: msg,
                };
                listener.on_event(event); // ✅ here you call it
            }
            None => {
                log::info!("The listener is not activated");
            }
        }
        None
    }
}