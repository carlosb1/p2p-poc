use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use libp2p::identity::Keypair;
use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use sled::Db;
use tokio::sync::mpsc::Sender;
use tokio::sync::Mutex;
use tokio::time::sleep;

use messages_types::ChatCommand;

use crate::models::db::Votation;
use crate::models::db::{DataContent, VoteStatus};
use crate::models::messages::{ContentMessage, Vote, DEFAULT_TOPIC};
use crate::protocol::MessageHandler;
use crate::{
    db, models, DEFAULT_REPUTATION, MEMBERS_FOR_CONSENSUS, MIN_REPUTATION_THRESHOLD, TIMEOUT_SECS,
};

pub struct ValidatorClient {
    peer_id: PeerId,
    command_tx: Sender<ChatCommand>,
    pub inner_handler: Arc<Mutex<dyn MessageHandler + Send + Sync>>,
    db: Arc<Db>,
    keypair: Keypair,
    pub content_to_evaluate: Mutex<Vec<(String, String, String, Duration)>>,
}

impl ValidatorClient {
    pub fn new(peer_id: PeerId, tx: Sender<ChatCommand>, db: Arc<Db>, keypair: Keypair) -> Self {
        ValidatorClient {
            peer_id,
            command_tx: tx,
            inner_handler: Arc::new(Mutex::new(crate::handler::ValidatorHandler::new(
                peer_id.clone(),
                db.clone(),
            ))),
            db,
            keypair,
            content_to_evaluate: Mutex::new(Vec::new()),
        }
    }
    pub fn new_key_available(&self, topic: &str, content: &str) -> anyhow::Result<String> {
        let key_for_checking = db::create_key_without_status(topic, content);
        /*  check if content was added before */
        //TODO return reason
        let is_added = self
            .db
            .scan_prefix(key_for_checking.clone())
            .next()
            .is_some();
        if is_added {
            log::debug!("Content already added: {}", content);
            return Err(anyhow!("Content already added: {}", content));
        }
        let key = db::create_key_for_voting_db(content, topic, "pending", 1);
        Ok(key)
    }

    pub async fn ask_validation(
        &self,
        key: &str,
        topic: &str,
        content: &str,
    ) -> anyhow::Result<()> {
        self.add_validation_request(key.to_string(), topic.to_string(), content.to_string())
            .await;
        // send petition
        let message = ContentMessage::Interested {
            id_votation: key.to_string(),
            content: content.to_string(),
        };
        self.send(topic.to_string(), &message).await?;
        Ok(())
    }
    async fn add_validation_request(&self, key: String, topic: String, content: String) {
        self.content_to_evaluate.lock().await.push((
            key,
            topic,
            content,
            Duration::from_secs(TIMEOUT_SECS),
        ));
    }

    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }

    pub fn db(&self) -> Arc<Db> {
        self.db.clone()
    }

    pub async fn remote_new_topic(&self, topic: &str) -> anyhow::Result<()> {
        self.command_tx
            .send(ChatCommand::Publish(
                DEFAULT_TOPIC.to_string(),
                serde_json::to_vec(&ContentMessage::RegisterTopic {
                    topic: topic.to_string(),
                })?,
            ))
            .await
            .map_err(|e| anyhow!("Failed to send vote: {}", e))?;
        Ok(())
    }

    pub async fn register_topic(&self, topic: &str) -> anyhow::Result<()> {
        self.command_tx
            .send(ChatCommand::Subscribe(topic.to_string()))
            .await?;
        Ok(())
    }

    pub async fn add_vote(&self, id_votation: &str, topic: &str, vote: Vote) -> anyhow::Result<()> {
        let Some(votation) = db::get_status_vote(&self.db, id_votation) else {
            log::debug!("You are not included in this votation={}", id_votation);
            return Ok(());
        };

        let data = serde_json::to_vec(&ContentMessage::ResultVote {
            id_votation: id_votation.to_string(),
            result: vote,
        })?;

        if (votation.leader_id == self.peer_id.to_string()) {
            log::debug!(
                "I am the leader with id={} for votation={} can I added it locally",
                votation.leader_id.clone(),
                id_votation
            );
            self.inner_handler
                .lock()
                .await
                .handle_message(self.peer_id, &data, topic);
            return Ok(());
        }

        log::debug!("Sending a remote publish message");
        self.command_tx
            .send(ChatCommand::Publish(topic.to_string(), data))
            .await
            .map_err(|e| anyhow!("Failed to send vote: {}", e))?;
        Ok(())
    }

    pub async fn send(&self, topic: String, message: &ContentMessage) -> anyhow::Result<()> {
        self.command_tx
            .send(ChatCommand::Publish(topic, serde_json::to_vec(message)?))
            .await?;
        Ok(())
    }

    pub fn get_voters(&self, key: &str, topic: &str) -> anyhow::Result<Vec<String>> {
        db::get_voters(&self.db, &key, &topic)
    }

    pub fn get_reputation(&self, peer_id: &str, topic: &str) -> anyhow::Result<f32> {
        db::get_reputation(&self.db, topic, peer_id).ok_or(anyhow!("topic not found"))
    }
    pub fn get_reputations(&self, topic: &str) -> Vec<(String, f32)> {
        db::get_reputations(&self.db, topic)
    }

    pub fn all_content(&self) -> Vec<DataContent> {
        db::get_contents(&self.db)
    }

    pub fn get_status_vote(&self, key: &str) -> Option<Votation> {
        db::get_status_vote(&self.db, key)
    }

    pub fn get_status_voteses(&self) -> Vec<Votation> {
        db::get_status_voteses(&self.db)
    }

    pub async fn get_content_to_evaluate(&self) -> Vec<(String, String, String, Duration)> {
        //key, topic, content, duration
        let content_to_evaluate = self.content_to_evaluate.lock().await;
        content_to_evaluate.clone()
    }

    /* operations to check my pending content to evaluate */

    pub async fn my_pending_content_to_validate(
        &self,
        data_content: &DataContent,
    ) -> anyhow::Result<()> {
        db::my_pending_content_to_validate(&self.db, data_content)
    }

    pub async fn get_my_pending_to_contents_to_validate(&self) -> Vec<DataContent> {
        db::get_my_pending_to_contents_to_validate(&self.db)
    }

    pub async fn wait_for_validators(&self) -> anyhow::Result<()> {
        let check_interval = Duration::from_millis(500);
        let start = tokio::time::Instant::now();

        loop {
            let mut content_to_evaluate = self.content_to_evaluate.lock().await;

            let mut index = 0;
            while index < content_to_evaluate.len() {
                let req_to_validate = content_to_evaluate
                    .get(index)
                    .ok_or(anyhow!("No request to validate at index {}", index))?;
                let (key, topic, content, timeout) = req_to_validate;
                log::debug!(
                    "Checking content to evaluate: key={}, topic={}, content={} timeout={:?}",
                    key,
                    topic,
                    content,
                    timeout
                );

                log::debug!("start elapsed: {:?}", start.elapsed());

                if start.elapsed() >= *timeout {
                    content_to_evaluate.remove(index);
                    //                    index+= 1; // increment index to check next content
                    continue;
                }

                /* we want to receive all the possible voters, f32 is the reputation */
                let mut filtered_votes: Vec<(String, f32)> = Vec::new();
                for possible_voter_peer_id in db::get_voters(&self.db, &key, &topic)? {
                    let rep =
                        db::get_reputation(&self.db, &possible_voter_peer_id.as_str(), &topic)
                            .unwrap_or_else(|| {
                                // if it is new one we save the default reputation
                                db::set_reputation(
                                    &self.db,
                                    &topic,
                                    &possible_voter_peer_id.as_str(),
                                    DEFAULT_REPUTATION,
                                )
                                .expect("Failed to set default reputation");
                                DEFAULT_REPUTATION
                            });
                    if rep >= MIN_REPUTATION_THRESHOLD {
                        filtered_votes.push((possible_voter_peer_id, rep));
                    }
                }
                log::debug!("Filtered votes for key {}: {:?}", key, filtered_votes);
                if filtered_votes.len() >= MEMBERS_FOR_CONSENSUS {
                    log::debug!(
                        "Enough votes collected for key {}: {:?}",
                        key,
                        filtered_votes
                    );
                    let filtered_votes: Vec<(String, f32)> =
                        filtered_votes[0..MEMBERS_FOR_CONSENSUS].to_vec();
                    let leader_peer = filtered_votes.first().expect("No leader available");
                    log::debug!("Selected leader for voting: {:?}", leader_peer);
                    let vote_request = ContentMessage::new_vote_leader_request(
                        key.clone(),
                        content.to_string(),
                        self.peer_id.to_string(),
                        filtered_votes
                            .iter()
                            .map(|(peer_id, _)| peer_id.clone())
                            .collect(),
                        leader_peer.0.clone(),
                        60,
                        &self.keypair,
                    )
                    .expect("Failed to create vote request");
                    self.send(topic.to_string(), &vote_request)
                        .await
                        .expect("Failed to send client message");
                    content_to_evaluate.remove(index); // ✅ remove it
                    continue; // skip index++
                }

                index += 1; // checking next content to evaluate
            }
            sleep(check_interval).await;
        }
    }
}
