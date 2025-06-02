use libp2p::PeerId;

pub trait MessageHandler: Send + 'static {
    fn handle_message(&mut self, peer: PeerId, data: &[u8])-> Option<Vec<u8>>;
}
