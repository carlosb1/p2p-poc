pub mod p2p;
pub use libp2p::identity::Keypair;
pub use libp2p::PeerId;
pub use protocol_p2p::models::db::{DataContent, StateContent, Votation, VoteStatus};
pub use protocol_p2p::models::messages::Vote;
