use libp2p::PeerId;
use rand::Rng;
use crate::p2p::node::ChatCommand;

pub fn generate_rand_msg() -> String {
    let mut rng = rand::rng();
    let random_number: u32 = rng.random_range(0..10000);
    format!("Random message: {}", random_number)
}



pub trait MessageHandler: Send + 'static {
    fn handle_message(&mut self, peer: PeerId, data: &[u8])-> Option<Vec<u8>>;
}


#[derive(Debug, Clone, Default)]
pub struct SimpleClientHandler;

// returns topic, and str message
impl MessageHandler for SimpleClientHandler {
    fn handle_message(&mut self, peer: PeerId, data: &[u8]) -> Option<Vec<u8>>{
        let str_message = String::from_utf8_lossy(data).to_string();
         if str_message.contains("hello world")  {
            log::debug!("Node: received hello world message from {}", peer);
            let random_msg = generate_rand_msg();
            let ret_msg = format!("Hello, world {:?}", random_msg);
            log::debug!("Node: sending back message: {:?}", ret_msg.clone());
            return Some( ret_msg.into_bytes());
         }
        None
    }
}

