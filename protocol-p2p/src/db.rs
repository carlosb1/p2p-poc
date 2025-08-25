use crate::models::db::{DataContent, StateContent, Topic, Votation, VoteStatus};
use crate::models::messages::Vote;
use crate::{db, models};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json;
use sled;
use sled::{CompareAndSwapError, Db};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;

pub fn create_key_for_voting_db(
    content_id: &str,
    topic: &str,
    status: &str,
    number_round: u32,
) -> String {
    let hash_content_id = default_hash(content_id);

    format!(
        "vote_status/{}:{}:{}:{}",
        topic, hash_content_id, status, number_round
    )
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

/* voter operations */
pub fn store_voter(
    db: &sled::Db,
    id_votation: &str,
    source_peer: &str,
    topic: &str,
) -> anyhow::Result<()> {
    let key = format!("election/{topic}/jury/{id_votation}");
    log::debug!(
        "Storing voter {} for votation {} with key {:?}",
        source_peer,
        id_votation,
        key
    );
    let jury = if let Some(value) = db.get(&key)? {
        let mut jury: Jury = serde_json::from_slice(&value)?;
        if jury.voters.contains(&source_peer.to_string()) {
            log::debug!("Peer {} already voted in {}", source_peer, id_votation);
            return Ok(());
        }
        jury.voters.push(source_peer.to_string());
        jury
    } else {
        log::debug!("Including a new jury process for votation {}", id_votation);
        Jury {
            voters: vec![source_peer.to_string()],
        }
    };
    db.insert(key.to_string(), serde_json::to_vec(&jury)?)?;
    Ok(())
}

pub fn get_voters(db: &sled::Db, id_votation: &str, topic: &str) -> anyhow::Result<Vec<String>> {
    let key = format!("election/{topic}/jury/{id_votation}");
    log::debug!("getting voter with key {:?}", key);
    if let Some(value) = db.get(&key)? {
        let jury: Jury = serde_json::from_slice(&value)?;
        return Ok(jury.voters);
    }
    Ok(vec![])
}

/* reputations dbs */

pub fn get_reputation(db: &sled::Db, topic: &str, peer_id: &str) -> Option<f32> {
    let key = format!("election/{topic}/reputation/{peer_id}");
    log::debug!("getting reputation with key {:?}", key);
    if let Some(value) = db.get(key).ok()? {
        if let Ok(reputation) = serde_json::from_slice::<f32>(&value) {
            return Some(reputation);
        }
    }
    None
}

pub fn set_reputation(
    db: &sled::Db,
    topic: &str,
    peer_id: &str,
    reputation: f32,
) -> Result<(), sled::Error> {
    let key = format!("election/{topic}/reputation/{peer_id}");
    log::debug!(
        "Setting reputation for peer {} in topic {} with key {:?}",
        peer_id,
        topic,
        key
    );
    let value = serde_json::to_vec(&reputation).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    Ok(())
}

pub fn get_reputations(db: &sled::Db, topic: &str) -> Vec<(String, f32)> {
    let results: Vec<(String, f32)> = db
        .scan_prefix(format!("election/{topic}/reputation"))
        .map(|item| {
            if let Ok((key, value)) = item {
                if let Ok(reputation) = serde_json::from_slice::<f32>(&value) {
                    if let Ok(peer_id) = String::from_utf8(key.to_vec()) {
                        return Some((peer_id, reputation));
                    }
                }
            }
            None
        })
        .filter_map(|x| x)
        .collect();
    return results;
}

pub fn update_reputations(
    db: &sled::Db,
    topic: &str,
    reputations: &[(String, f32)],
    default_reputation: f32,
) -> anyhow::Result<()> {
    for (peer_id, reputation) in reputations {
        match get_reputation(db, topic, peer_id) {
            Some(existing_reputation) => {
                // increase reputation
                let new_reputation = existing_reputation + reputation;
                log::debug!(
                    "Updating reputation for peer {} in topic {} from {} to {}",
                    peer_id,
                    topic,
                    existing_reputation,
                    new_reputation
                );
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

/* Topics db */
pub async fn get_topics(db: &Db) -> Vec<Topic> {
    db.scan_prefix("topics/")
        .filter_map(|item| {
            if let Ok((_key, value)) = item {
                serde_json::from_slice::<Topic>(&value).ok()
            } else {
                None
            }
        })
        .collect()
}

pub async fn save_topic(db: &Db, topic: &Topic) -> anyhow::Result<()> {
    let key = "topics/";
    let value = serde_json::to_string(topic)?.into_bytes();
    db.insert(key, value)?;
    db.flush()?;
    Ok(())
}

/*  Content db */
pub fn include_new_validated_content(
    db: &Db,
    data_content: &models::db::DataContent,
) -> anyhow::Result<()> {
    let id_votation = data_content.id_votation.clone();
    let key = format!("content/{id_votation}");
    let value = serde_json::to_vec(data_content).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    db.flush()?;
    Ok(())
}

pub fn get_contents(db: &Db) -> Vec<models::db::DataContent> {
    db.scan_prefix("content/")
        .filter_map(|item| {
            if let Ok((_key, value)) = item {
                serde_json::from_slice::<models::db::DataContent>(&value).ok()
            } else {
                None
            }
        })
        .collect()
}

/* Save my pending content to validate, if I am proposed */
pub fn my_pending_content_to_validate(db: &Db, data_content: &DataContent) -> anyhow::Result<()> {
    let id_votation = data_content.id_votation.clone();
    let key = format!("my_pending_content_to_validate/{id_votation}");
    let value = serde_json::to_vec(data_content).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    db.flush()?;
    Ok(())
}

pub fn get_my_pending_to_contents_to_validate(db: &Db) -> Vec<models::db::DataContent> {
    db.scan_prefix("my_pending_content_to_validate/")
        .filter_map(|item| {
            if let Ok((_key, value)) = item {
                serde_json::from_slice::<models::db::DataContent>(&value).ok()
            } else {
                None
            }
        })
        .collect()
}

/* votes db operations */

pub fn get_votes(db: &Db, id_votation: &str) -> Vec<(String, Vote)> {
    let key = format!("election/vote/{id_votation}");
    db.scan_prefix(key)
        .filter_map(|item| {
            if let Ok((find_key, value)) = item {
                let value = serde_json::from_slice::<(&str, Vote)>(&value)
                    .ok()
                    .map(|(s, vote)| (s.to_string(), vote));
                value
            } else {
                None
            }
        })
        .collect()
}

pub fn exists_vote(db: &Db, id_votation: &str, peer_id: &str) -> anyhow::Result<bool> {
    let key = format!("election/vote/{id_votation}/{peer_id}");
    let exists = db.contains_key(key)?; // propagates error if any
    Ok(exists)
}

pub fn add_vote(db: &Db, id_votation: &str, peer_id: &str, vote: &Vote) -> anyhow::Result<()> {
    let key = format!("election/vote/{id_votation}/{peer_id}");
    db.insert(
        key.to_string(),
        serde_json::to_vec(&(peer_id, vote.clone()))?,
    )?;
    Ok(())
}
//TODO restructure keys (naming and where we save the tables)
/* Status vote db opers */
pub fn new_status_vote(
    db: &sled::Db,
    id_votation: &str,
    votation: &Votation,
) -> anyhow::Result<()> {
    let id_status_vote = format!("pending_content/{id_votation}");
    let vec_new = serde_json::to_string(&votation)?.into_bytes();

    match db.compare_and_swap(id_status_vote, None::<Vec<u8>>, Some(vec_new))? {
        Ok(_) => Ok(()),
        Err(CompareAndSwapError {
            current,
            proposed: _,
        }) => Err(anyhow::anyhow!(
            "Compare-and-swap failed: current value was {:?}",
            current
        )),
    }
}

pub fn get_status_vote(db: &sled::Db, id_votation: &str) -> Option<Votation> {
    let id_status_vote = format!("pending_content/{id_votation}");
    db.get(id_status_vote)
        .ok()?
        .and_then(|value| { serde_json::from_slice::<Votation>(&value).ok() }.or(None))
}

pub fn get_status_voteses(db: &sled::Db) -> Vec<Votation> {
    let key = format!("pending_content/");
    db.scan_prefix(key)
        .filter_map(|item| {
            if let Ok((_key, value)) = item {
                serde_json::from_slice::<Votation>(&value).ok()
            } else {
                None
            }
        })
        .collect()
}

pub fn compare_and_swap_status_vote(
    db: &sled::Db,
    id_votation: &str,
    old: &Votation,
    new: &Votation,
) -> anyhow::Result<()> {
    let id_status_vote = format!("pending_content/{id_votation}");
    let vec_old = serde_json::to_string(&old)?.into_bytes();
    let vec_new = serde_json::to_string(&new)?.into_bytes();
    match db.compare_and_swap(id_status_vote, Some(vec_old), Some(vec_new))? {
        Ok(_) => Ok(()),
        Err(CompareAndSwapError {
            current,
            proposed: _,
        }) => Err(anyhow::anyhow!(
            "Compare-and-swap failed: current value was {:?}",
            current
        )),
    }
}

#[test]
fn test_create_key_for_voting_db() {
    let key = create_key_for_voting_db("content123", "topicA", "pending", 1);
    assert!(key.starts_with("vote_status/topicA:"));
    assert!(key.ends_with(":pending:1"));
}

#[test]
fn test_create_key_without_status() {
    let key = create_key_without_status("content123", "topicA");
    assert!(key.starts_with("vote_status/topicA:"));
}

#[test]
fn test_store_and_get_voters() {
    use std::collections::HashSet;

    let tmp_dir = tempfile::tempdir().unwrap();
    let db = init_db(tmp_dir.path().to_str().unwrap()).unwrap();
    let id_votation = "vot1";
    let topic = "topicX";

    store_voter(&db, &id_votation, "peer1", topic).unwrap();
    store_voter(&db, &id_votation, "peer2", topic).unwrap();

    let voters = get_voters(&db, &id_votation, topic).unwrap();
    let voters_set: HashSet<_> = voters.into_iter().collect();
    assert_eq!(voters_set.len(), 2);
    assert!(voters_set.contains("peer1"));
    assert!(voters_set.contains("peer2"));
}

#[test]
fn test_set_get_reputation() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("debug"));
    let tmp_dir = tempfile::tempdir().unwrap();
    let db = init_db(tmp_dir.path().to_str().unwrap()).unwrap();

    let topic = "rep_topic";
    let peer_id = "peerX";

    set_reputation(&db, topic, peer_id, 4.5).unwrap();
    let rep = get_reputation(&db, topic, peer_id).unwrap();
    assert_eq!(rep, 4.5);
}

#[test]
fn test_include_and_read_validated_content() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db = init_db(tmp_dir.path().to_str().unwrap()).unwrap();

    let data =
        models::db::DataContent::new("vote_01".to_string(), "important_content".to_string(), true);
    include_new_validated_content(&db, &data).unwrap();

    let key = format!("/content/{}", data.id_votation);
    let raw = db.get(&key).unwrap().unwrap();
    let decoded: models::db::DataContent = serde_json::from_slice(&raw).unwrap();

    assert_eq!(decoded.id_votation, "vote_01");
    assert_eq!(decoded.content, "important_content");
    matches!(decoded.approved, StateContent::Approved);
}

#[test]
fn test_insert_and_get_status_vote() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db = init_db(tmp_dir.path().to_str().unwrap()).unwrap();

    let vote = Votation::new(
        "vote_id_123".to_string(),
        "content_xyz".to_string(),
        "approved".to_string(),
        "leader123".to_string(),
        "leader".to_string(),
        vec![("peer1".to_string(), Some(3.0))],
    );

    db::new_status_vote(&db, &vote.id_votation, &vote).unwrap();
    let result = get_status_vote(&db, &vote.id_votation).unwrap();

    assert_eq!(result.id_votation, vote.id_votation);
    assert_eq!(result.status, "approved");
}

#[test]
fn test_insert_and_get_status_vote_and_update() {
    let tmp_dir = tempfile::tempdir().unwrap();
    let db = init_db(tmp_dir.path().to_str().unwrap()).unwrap();

    let vote = Votation::new(
        "vote_id_123".to_string(),
        "content_xyz".to_string(),
        "approved".to_string(),
        "leader123".to_string(),
        "leader".to_string(),
        vec![("peer1".to_string(), Some(3.0))],
    );

    new_status_vote(&db, &vote.id_votation, &vote).unwrap();
    let mut result = get_status_vote(&db, &vote.id_votation).unwrap();
    (*result.votes_id.get_mut(0).unwrap()).0 = "peer_modified".to_string();

    compare_and_swap_status_vote(&db, &result.id_votation, &vote, &result).unwrap();
    let mut result = get_status_vote(&db, &vote.id_votation).unwrap();
    println!("result {:?}", result);

    assert_eq!(result.id_votation, vote.id_votation);
    assert_eq!(result.status, "approved");
    let votes = get_status_voteses(&db);
    println!("votes {:?}", votes);
}
