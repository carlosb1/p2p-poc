use std::sync::Arc;

use libp2p::PeerId;
use sled::Db;

use crate::db::{DataContent, Votation};
use crate::models::messages::ContentMessage;
use crate::{db, MessageHandler, DEFAULT_REPUTATION, INCR_REPUTATION, THRESHOLD_APPROVE};

pub struct ValidatorHandler {
    peer_id: PeerId,
    db: Arc<Db>,
}

impl ValidatorHandler {
    pub fn new(peer_id: PeerId, db: Arc<Db>) -> Self {
        ValidatorHandler { peer_id, db }
    }
}

impl MessageHandler for ValidatorHandler {
    fn handle_message(&mut self, source_peer: PeerId, data: &[u8], topic: &str) -> Option<Vec<u8>> {
        log::info!(
            "{:?} - Received message from {}: {:?}",
            self.peer_id.clone(),
            source_peer,
            String::from_utf8_lossy(data)
        );

        let db = &self.db;

        if let Ok(res) = serde_json::from_slice(data) {
            match res {
                ContentMessage::RegisterTopic { topic } => {
                    log::info!("New topic {:?}", topic);
                }
                ContentMessage::Interested {
                    content,
                    id_votation,
                } => {
                    log::info!("Received Interested message for content: {}", content);
                    let str_response: String =
                        serde_json::to_string(&ContentMessage::InterestedResponse {
                            id_votation: id_votation.clone(),
                        })
                        .unwrap();
                    return Some(str_response.into_bytes());
                }
                ContentMessage::InterestedResponse { id_votation } => {
                    // interested voters
                    log::info!(
                        "Received response for votation: {} from {}",
                        id_votation,
                        source_peer.to_string()
                    );
                    db::store_voter(&db, &id_votation, source_peer.to_string().as_str(), topic)
                        .ok()?; /*
                    convert result to option
                    check if you have enough voters
                     */
                }
                ContentMessage::VoteLeaderRequest {
                    id_votation,
                    content,
                    publisher_peer_id: _,
                    voters_peer_id,
                    leader_peer_id,
                    ttl_secs: _,
                    signature: _,
                } => {
                    log::info!(
                        "Received  VoteLeaderRequest (petition to be part of the vote) for votation: {}",
                        id_votation
                    );
                    /* set up a new status vote, check if you are the leader or not */
                    let my_self_str_peer_id = self.peer_id.to_string();
                    let mut votation = Votation::new(
                        id_votation.clone(),
                        content.clone(),
                        "pending".to_string(),
                        leader_peer_id.clone(),
                        "role_voter".to_string(),
                        voters_peer_id
                            .iter()
                            .map(|id| (id.to_string(), None))
                            .collect(),
                    );
                    if leader_peer_id == my_self_str_peer_id {
                        votation.leader_id = my_self_str_peer_id.clone();
                        votation.my_role = "role_leader".to_string();
                    }
                    db::insert_and_update_status_vote(&db, id_votation.as_str(), &votation).ok()?;
                }
                ContentMessage::ResultVote {
                    id_votation,
                    result,
                } => {
                    log::info!("Received ResultVote for votation: {}", id_votation);

                    // we receive a vote result

                    /* are you in the votation process */
                    let Some(mut votation) = db::get_status_vote(&db, &id_votation) else {
                        return None;
                    };

                    /* are you the leader?  */
                    log::info!(
                        "leader_id={:?}, my_self_str_peer_id={:?} source_peer={:?}",
                        votation.leader_id,
                        self.peer_id.to_string(),
                        source_peer.to_string()
                    );

                    // I am the leader?
                    if votation.leader_id != self.peer_id.to_string() {
                        return None;
                    }

                    // are you part of the voters?
                    let Some(entry) = votation
                        .votes_id
                        .iter_mut()
                        .find(|e| e.0 == source_peer.to_string())
                    else {
                        return None;
                    };

                    // did you vote before?
                    if entry.1.is_some() {
                        return None;
                    }

                    // Check the vote
                    let int_result = result as u8;
                    entry.1 = Some(int_result as f32);

                    db::insert_and_update_status_vote(&db, id_votation.as_str(), &votation).ok()?;

                    // are pending votes?
                    if !votation.votes_id.iter().all(|(_, v)| v.is_some()) {
                        //decrease reputation
                        let peer_ids = votation
                            .votes_id
                            .iter()
                            .filter(|(_, v)| v.is_none())
                            .map(|(peer_id, _)| (peer_id.clone(), -INCR_REPUTATION))
                            .collect::<Vec<(String, f32)>>();
                        log::debug!("Decreasing reputation for peers: {:?}", peer_ids);
                        db::update_reputations(&db, &topic, &peer_ids, DEFAULT_REPUTATION).ok()?;
                        return None;
                    }

                    // we validate the content
                    let votes: f32 = votation
                        .votes_id
                        .iter()
                        .map(|(_, vote)| vote.unwrap_or(0.0))
                        .sum();
                    let total_votes = votation.votes_id.len() as f32;

                    let percent_accept = votes / total_votes;
                    let approved = percent_accept >= THRESHOLD_APPROVE;

                    // send the result
                    let data = ContentMessage::IncludeNewValidatedContent {
                        id_votation: id_votation.clone(),
                        content: votation.content.clone(),
                        approved,
                    };

                    // update the reputation for voters
                    let peer_ids = votation
                        .votes_id
                        .iter()
                        .map(|(peer_id, _)| (peer_id.clone(), INCR_REPUTATION))
                        .collect::<Vec<(String, f32)>>();
                    db::update_reputations(&db, &topic, &peer_ids, DEFAULT_REPUTATION).ok()?;
                    return Some(serde_json::to_string(&data).ok()?.into_bytes());
                }
                ContentMessage::IncludeNewValidatedContent {
                    id_votation,
                    content,
                    approved,
                } => {
                    log::info!(
                        "Received IncludeNewValidatedContent for votation: {}",
                        id_votation
                    );
                    let data_content = DataContent::new(id_votation, content, approved);
                    db::include_new_validated_content(&db, &data_content).ok()?;
                }
            }
        }

        None
    }
}
