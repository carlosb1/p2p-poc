use std::hash::{DefaultHasher, Hash, Hasher};

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use serde_json;
use sled;
use sled::Db;

use crate::models::db::{RaftNodeState, VoteStatus};

pub fn create_key_for_voting_db(content_id: &str, topic: &str, status: &str, number_round: u32) -> String {
    let hash_content_id = default_hash(content_id);

    format!("vote_status/{}:{}:{}:{}", topic, hash_content_id, status, number_round)
}

fn default_hash(content_id: &str) -> u64 {
    let mut s = DefaultHasher::new();
    content_id.hash(&mut s);
    let hash_content_id = s.finish();
    hash_content_id
}

pub fn create_key_without_status(content_id: &str, topic: &str) -> String {
    let content_id = default_hash(content_id);
    format!("vote_status/{}:{}", topic, content_id)
}


pub fn init_db(path: &str) -> anyhow::Result<Db> {
    log::info!("Initializing database at path: {}", path);
    let db = sled::open(path)?;
    if db.was_recovered() {
        log::debug!("Database was recovered from previous instance.");
    } else {
        log::debug!("Fresh database or clean shutdown last time.");
    }
    Ok(db)
}


pub fn store_vote(db: &sled::Db, peer_id: &str, reputation: f32) -> Result<(), Box<dyn std::error::Error>> {
    //        let key = format!("election/votes/{}", peer_id);
    //        let value = serde_json::to_vec(msg)?;
    //        db.insert(key, value)?;
    Ok(())
}

pub fn get_votes(db: &sled::Db, peer_id: &str) -> anyhow::Result<Vec<(String, f32)>> {
    let key = format!("election/votes/{}", peer_id);
    if let Some(value) = db.get(key)? {
        if let Ok(VoteStatus::Pending(voting_message)) = serde_json::from_slice(value.as_ref()) {
            return Ok(voting_message);
        }
        Err(anyhow::anyhow!("Failed to deserialize vote status"))
    } else {
        Err(anyhow::anyhow!("Vote not found"))
    }
}


fn store_node_state(db: &sled::Db, state: &RaftNodeState) -> Result<(), Box<dyn std::error::Error>> {
    //    db.insert("election/state", serde_json::to(state)?)?;
    Ok(())
}


//db::store_status_vote(&id_votation, &content, "pending", &my_self_str_peer_id, "role_leader", &voters_peer_id);
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusVote {
    pub id_votation: String,
    pub content: String,
    pub status: String,
    pub leader_id: String,
    pub my_role: String,
    pub votes_id: Vec<(String, Option<f32>)>,
}

impl StatusVote {
    pub fn new(
        id_votation: String,
        content: String,
        status: String,
        leader_id: String,
        my_role: String,
        votes_id: Vec<(String, Option<f32>)>,
    ) -> Self {
        Self {
            id_votation,
            content,
            status,
            leader_id,
            my_role,
            votes_id,
        }
    }
}

pub(crate) fn store_new_status_vote(db: &sled::Db, id_votation: &str, content: &str, status: &str, leader_id: &str, my_role: &str, votes_id: &Vec<String>) -> anyhow::Result<()> {
    let status_vote = StatusVote::new(id_votation.to_string(),
                                      content.to_string(),
                                      status.to_string(),
                                      leader_id.to_string(),
                                      my_role.to_string(),
                                      votes_id.iter().map(|v| (v.clone(), None)).collect());
    let value = serde_json::to_string(&status_vote)?;
    db.insert(id_votation, value.into_bytes())?;
    Ok(())
}

pub(crate) fn get_status_vote(db: &sled::Db, key: &String) -> Option<StatusVote> {
    db.get(key)
        .ok()?
        .and_then(|value| {
            serde_json::from_slice::<StatusVote>(&value).ok()
        }.or(None))
}

pub(crate) fn save_interested_for_votation(p0: &String, p1: &String) {
    todo!()
}

pub(crate) fn get_reputation(p0: &PeerId) -> Option<f32> {
    todo!()
}

pub(crate) fn get_reputations() -> Vec<(String, f32)> {
    todo!()
}

pub(crate) fn update_status_vote(db: &sled::Db, status_vote: &StatusVote) -> anyhow::Result<()> {
    db.insert(&status_vote.id_votation, serde_json::to_string(status_vote).unwrap().into_bytes())?;
    Ok(())
}

pub(crate) fn include_new_validated_content(p0: &Db, p1: &String, p2: &String) -> anyhow::Result<()> {
    Ok(())
}