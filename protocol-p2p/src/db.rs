use std::hash::{DefaultHasher, Hash, Hasher};

use serde::{Deserialize, Serialize};
use serde_json;
use sled;
use sled::Db;

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

/* Operations for db management */

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
            approved: if approved {
                StateContent::Approved
            } else {
                StateContent::Rejected
            },
        }
    }
}

pub fn include_new_validated_content(db: &Db, data_content: &DataContent) -> anyhow::Result<()> {
    let id_votation = data_content.id_votation.clone();
    let key = format!("content/{id_votation}");
    let value = serde_json::to_vec(data_content).expect("Failed to serialize reputation");
    db.insert(key, value)?;
    Ok(())
}

pub fn get_contents(db: &Db) -> Vec<DataContent> {
    db.scan_prefix(format!("content/"))
        .filter_map(|item| {
            if let Ok((_key, value)) = item {
                serde_json::from_slice::<DataContent>(&value).ok()
            } else {
                None
            }
        })
        .collect()
}

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

pub fn insert_and_update_status_vote(
    db: &sled::Db,
    id_votation: &str,
    status_vote: &Votation,
) -> anyhow::Result<()> {
    let value = serde_json::to_string(&status_vote)?;
    db.insert(id_votation, value.into_bytes())?;
    Ok(())
}

pub fn get_status_vote(db: &sled::Db, key: &String) -> Option<Votation> {
    db.get(key)
        .ok()?
        .and_then(|value| { serde_json::from_slice::<Votation>(&value).ok() }.or(None))
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

    let data = DataContent::new("vote_01".to_string(), "important_content".to_string(), true);
    include_new_validated_content(&db, &data).unwrap();

    let key = format!("/content/{}", data.id_votation);
    let raw = db.get(&key).unwrap().unwrap();
    let decoded: DataContent = serde_json::from_slice(&raw).unwrap();

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

    insert_and_update_status_vote(&db, &vote.id_votation, &vote).unwrap();
    let result = get_status_vote(&db, &vote.id_votation).unwrap();

    assert_eq!(result.id_votation, vote.id_votation);
    assert_eq!(result.status, "approved");
}
