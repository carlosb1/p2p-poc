pub use protocol::MessageHandler;
pub use sled::Db;

pub mod client;
pub mod handler;
pub mod models;
pub mod protocol;

pub mod db;

const DEFAULT_REPUTATION: f32 = 90.0;

const TIMEOUT_SECS: u64 = 3600;
const MEMBERS_FOR_CONSENSUS: usize = 5;
const MIN_REPUTATION_THRESHOLD: f32 = 80.0;

const INCR_REPUTATION: f32 = 5.0;
const THRESHOLD_APPROVE: f32 = 0.6;
