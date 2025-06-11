use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use libp2p::identity::Keypair;
use libp2p::PeerId;
use sled::Db;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

use messages_types::ChatCommand;

use crate::{db, DEFAULT_REPUTATION, MEMBERS_FOR_CONSENSUS, MIN_REPUTATION_THRESHOLD, TIMEOUT_SECS};
use crate::models::messages::ContentMessage;
use crate::protocol::MessageHandler;

pub struct LinkClient {
    peer_id: PeerId,
    command_tx: Sender<ChatCommand>,
    inner_handler: Arc<dyn MessageHandler + Send + Sync>,
    db: Arc<Db>,
    keypair: Keypair,
    content_to_evaluate: Vec<(String, String, String, Duration)>,
}

impl LinkClient {
    pub fn new(peer_id: PeerId, tx: Sender<ChatCommand>, db: Arc<Db>, keypair: Keypair) -> Self {
        LinkClient {
            peer_id,
            command_tx: tx,
            inner_handler: Arc::new(crate::handler::LinkHandler::new(peer_id.clone(), db.clone())),
            db,
            keypair,
            content_to_evaluate: Vec::new(),
        }
    }
    pub fn new_key_available(&self, topic: &str, content: &str) -> anyhow::Result<String> {
        let key_for_checking = db::create_key_without_status(topic, content);
        /*  check if content was added before */
        //TODO return reason
        let is_added = self.db.scan_prefix(key_for_checking.clone()).next().is_some();
        if is_added {
            log::info!("Content already added: {}", content);
            return Err(anyhow!("Cnotent already added: {}", content));
        }
        let key = db::create_key_for_voting_db(content, topic, "pending", 1);
        return Ok(key);
    }

    pub async fn ask_validation(&self, key: &str, topic: &str, content: &str) -> anyhow::Result<()> {
        let message = ContentMessage::Interested {
            id_votation: key.to_string(),
            content: content.to_string(),
        };
        self.send(topic.to_string(), &message).await?;
        Ok(())
    }
    pub fn add_validation_request(&mut self, key: String, topic: String, content: String) {
        self.content_to_evaluate.push((key, topic, content, Duration::from_secs(TIMEOUT_SECS)));
    }


    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    pub async fn register_topic(&self, topic: &str) -> anyhow::Result<()> {
        self.command_tx.send(ChatCommand::Subscribe(topic.to_string())).await?;
        Ok(())
    }

    pub async fn send(&self, topic: String, message: &ContentMessage) -> anyhow::Result<()> {
        self.command_tx.send(ChatCommand::Publish(topic, serde_json::to_vec(message)?)).await?;
        Ok(())
    }

    pub async fn listen(&mut self) -> anyhow::Result<()> {
        let check_interval = Duration::from_millis(500);
        let start = tokio::time::Instant::now();

        loop {
            if self.content_to_evaluate.is_empty() {
                log::debug!("No content to evaluate, waiting for requests...");
            }

            let mut index = 0;
            while index < self.content_to_evaluate.len() {
                let req_to_validate = self.content_to_evaluate.get(index).ok_or(anyhow!("No request to validate at index {}", index))?;
                let (key, topic, content, timeout) = req_to_validate;
                log::debug!("Checking content to evaluate: key={}, topic={}, content={}", key, topic, content);
                if start.elapsed() >= *timeout {
                    log::debug!("Timeout reached, stopping listen loop for topic:");
                    /* remove element if it achieves the timeout */
                    self.content_to_evaluate.remove(index);
                    index += 1;
                    return Ok(());
                }

                /* we want to receive all the possible voters, f32 is the reputation */
                let mut filtered_votes: Vec<(String, f32)> = Vec::new();
                for possible_voter_peer_id in db::get_voters(&self.db, &key, &topic).unwrap() {
                    let rep = db::get_reputation(&self.db, &possible_voter_peer_id.as_str(), &topic).unwrap_or_else(
                        || {
                            // if it is new one we save the default reputation
                            db::set_reputation(&self.db, &topic, &possible_voter_peer_id.as_str(), DEFAULT_REPUTATION)
                                .expect("Failed to set default reputation");
                            DEFAULT_REPUTATION
                        });
                    if rep >= MIN_REPUTATION_THRESHOLD {
                        filtered_votes.push((possible_voter_peer_id, rep));
                    }
                }
                log::debug!("Filtered votes for key {}: {:?}", key, filtered_votes);
                if filtered_votes.len() >= MEMBERS_FOR_CONSENSUS {
                    log::debug!("Enough votes collected for key {}: {:?}", key, filtered_votes);
                    let filtered_votes: Vec<(String, f32)> = filtered_votes[0..MEMBERS_FOR_CONSENSUS].to_vec();
                    let leader_peer = filtered_votes.first().expect("No leader available");
                    log::debug!("Selected leader for voting: {:?}", leader_peer);
                    let vote_request = ContentMessage::new_vote_leader_request(
                        key.clone(),
                        content.to_string(),
                        self.peer_id.to_string(),
                        filtered_votes.iter().map(|(peer_id, _)| peer_id.clone()).collect(),
                        leader_peer.0.clone(),
                        60,
                        &self.keypair,
                    ).expect("Failed to create vote request");
                    self.send(topic.to_string(), &vote_request).await.expect("Failed to send client message");
                }

                index += 1; // checking next content to evaluate
            }
            sleep(check_interval).await;
        }
    }
}
