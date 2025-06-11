use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

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

pub fn create_key_without_status(content_id: &str, topic: &str) -> String {
    let content_id = default_hash(content_id);
    format!("vote_status/{}:{}", topic, content_id)
}

fn default_hash(content_id: &str) -> u64 {
    let mut s = DefaultHasher::new();
    content_id.hash(&mut s);
    let hash_content_id = s.finish();
    hash_content_id
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


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Jury {
    voters: Vec<String>,
}


pub fn store_voter(db: &sled::Db, id_votation: &String, source_peer: &str, topic: &str) -> anyhow::Result<()> {
    let key = format!("election/{topic}/jury/{id_votation}");
    if let Some(value) = db.get(&key)? {
        let mut jury: Jury = serde_json::from_slice(&value)?;
        if jury.voters.contains(&source_peer.to_string()) {
            log::debug!("Peer {} already voted in {}", source_peer, id_votation);
            return Ok(());
        }
        jury.voters.push(source_peer.to_string());
        db.insert(key.to_string(), serde_json::to_string(&jury)?.into_bytes())?;
    }
    Ok(())
}


pub fn get_voters(db: &sled::Db, id_votation: &str, topic: &str) -> anyhow::Result<Vec<String>> {
    let key = format!("election/{topic}/jury/{id_votation}");
    if let Some(value) = db.get(&key)? {
        let mut jury: Jury = serde_json::from_slice(&value)?;
        return Ok(jury.voters);
    }
    Ok(vec![])
}

/* Operations for db management */

pub fn get_reputation(db: &sled::Db, topic: &str, peer_id: &str) -> Option<f32> {
    let key = format!("election/{topic}/reputation/{peer_id}");
    if let Some(value) = db.get(key).ok()? {
        if let Ok(reputation) = serde_json::from_slice::<f32>(&value) {
            return Some(reputation);
        }
    }
    None
}

pub fn set_reputation(db: &sled::Db, topic: &str, peer_id: &str, reputation: f32) -> Result<(), sled::Error> {
    let key = format!("/election/{topic}/reputation/{peer_id}");
    let value = serde_json::to_vec(&reputation).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    Ok(())
}

pub(crate) fn get_reputations(db: &sled::Db, topic: &str) -> Vec<(String, f32)> {
    let results: Vec<(String, f32)> = db.scan_prefix(format!("/election/{topic}/reputation")).map(|item| {
        if let Ok((key, value)) = item {
            if let Ok(reputation) = serde_json::from_slice::<f32>(&value) {
                if let Ok(peer_id) = String::from_utf8(key.to_vec()) {
                    return Some((peer_id, reputation));
                }
            }
        }
        None
    }).filter_map(|x| x).collect();
    return results;
}

pub fn update_reputations(db: &sled::Db, topic: &str, reputations: &[(String, f32)], default_reputation: f32) -> anyhow::Result<()> {
    for (peer_id, reputation) in reputations {
        match get_reputation(db, topic, peer_id) {
            Some(existing_reputation) => {
                // increase reputation
                let new_reputation = existing_reputation + reputation;
                log::debug!("Updating reputation for peer {} in topic {} from {} to {}", peer_id, topic, existing_reputation, new_reputation);
                set_reputation(db, topic, peer_id, new_reputation)?;
            }
            None => {
                log::debug!("Peer is not in my reputation list...");
                set_reputation(db, topic, peer_id, default_reputation)?;
            }
        }
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StateContent {
    Approved,
    Rejected,

}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DataContent {
    pub id_votation: String,
    pub content: String,
    pub approved: StateContent,
}

impl DataContent {
    pub fn new(id_votation: String, content: String, approved: bool) -> Self {
        Self {
            id_votation,
            content,
            approved: if approved { StateContent::Approved } else { StateContent::Rejected },
        }
    }
}


pub fn include_new_validated_content(db: &Db,
                                     data_content: &DataContent) -> anyhow::Result<()> {
    let id_votation = data_content.id_votation.clone();
    let key = format!("/content/{id_votation}");
    let value = serde_json::to_vec(data_content).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    Ok(())
}


//db::store_status_vote(&id_votation, &content, "pending", &my_self_str_peer_id, "role_leader", &voters_peer_id);
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Votation {
    pub id_votation: String,
    pub content: String,
    pub status: String,
    pub leader_id: String,
    pub my_role: String,
    pub votes_id: Vec<(String, Option<f32>)>,
}

impl Votation {
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

/*  */

pub fn insert_and_update_status_vote(db: &sled::Db, id_votation: &str, status_vote: &Votation) -> anyhow::Result<()> {
    let value = serde_json::to_string(&status_vote)?;
    db.insert(id_votation, value.into_bytes())?;
    Ok(())
}

pub fn get_status_vote(db: &sled::Db, key: &String) -> Option<Votation> {
    db.get(key)
        .ok()?
        .and_then(|value| {
            serde_json::from_slice::<Votation>(&value).ok()
        }.or(None))
}


