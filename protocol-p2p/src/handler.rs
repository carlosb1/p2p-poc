use crate::models::messages::{ContentMessage, Vote};
use crate::{
    db, models, MessageHandler, DEFAULT_REPUTATION,
    EXPIRY_DURATION_IN_DAYS, INCR_REPUTATION, THRESHOLD_APPROVE,
};
use chrono::Utc;
use libp2p::PeerId;
use sled::Db;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Debug, Clone)]
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
        log::debug!(
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
                    log::debug!("Received Interested message for content: {}", content);
                    let str_response: String =
                        serde_json::to_string(&ContentMessage::InterestedResponse {
                            id_votation: id_votation.clone(),
                        })
                        .unwrap();
                    return Some(str_response.into_bytes());
                }
                ContentMessage::InterestedResponse { id_votation } => {
                    // interested voters
                    log::debug!(
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
                    log::debug!(
                        "Received  VoteLeaderRequest (petition to be part of the vote) for votation: {}",
                        id_votation
                    );
                    /* set up a new status vote, check if you are the leader or not */
                    let my_self_str_peer_id = self.peer_id.to_string();
                    let mut votation = models::db::Votation::new(
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

                    if db::get_status_vote(&db, id_votation.as_str()).is_none() {
                        db::new_status_vote(&db, id_votation.as_str(), &votation).ok()?;
                    } else {
                        log::warn!("Trying to insert again a votation");
                    }
                }
                ContentMessage::ResultVote {
                    id_votation,
                    result,
                } => {
                    log::debug!("Received ResultVote for votation: {}", id_votation);

                    // we receive a vote result

                    /* are you in the votation process */
                    let str_peer_id = source_peer.to_string();

                    let Some(mut votation) = db::get_status_vote(&db, &id_votation) else {
                        return None;
                    };

                    log::debug!("status extracted votation={:?}", votation);

                    /* are you the leader?  */
                    log::debug!(
                        "leader_id={:?}, my_self_str_peer_id={:?} source_peer={:?}",
                        votation.leader_id,
                        self.peer_id.to_string(),
                        source_peer.to_string()
                    );

                    // I am the leader? only leader can count votes
                    if votation.leader_id != self.peer_id.to_string() {
                        log::debug!(
                            "Discarding result vote petition I am not the leader votation_leader={:?} and my id={:?}",
                            votation,
                            self.peer_id.to_string()
                        );
                        return None;
                    }

                    let Ok(exist) = db::exists_vote(db, &id_votation.as_str(), &str_peer_id) else {
                        log::warn!("Exists vote failed");
                        return None;
                    };

                    if exist {
                        log::warn!(
                            "Discarding replicated vote for  votation={} peer_id={}",
                            id_votation,
                            self.peer_id
                        );
                        return None;
                    }

                    if db::add_vote(db, &id_votation.as_str(), &str_peer_id, &result).is_err() {
                        log::warn!(
                            "It could no possible to add a vote={:?} for votation={} and peer_id={}",
                            result,
                            &id_votation,
                            &str_peer_id
                        );
                        return None;
                    }

                    let votes_and_its_points: Vec<(String, Vote)> =
                        db::get_votes(db, &id_votation.as_str());
                    let recollected_votes: HashSet<String> = votes_and_its_points
                        .iter()
                        .map(|(x, _)| x.to_string())
                        .collect();
                    let expected_votes: HashSet<String> = votation
                        .votes_id
                        .iter()
                        .map(|(x, _)| x.to_string())
                        .collect();

                    /* expire votation */
                    log::debug!("Pending for votation={:?}", votation);
                    log::debug!("Recollected votes={:?}", recollected_votes);
                    log::debug!("Expected_votes votes={:?}", expected_votes);

                    let expires_at = votation.timestamp + EXPIRY_DURATION_IN_DAYS;
                    let now = Utc::now();
                    if !expected_votes.is_subset(&recollected_votes) && now > expires_at {
                        log::debug!("⛔ Vote expired, we are going to decrease reputation");

                        let not_votes: HashSet<String> = recollected_votes
                            .difference(&expected_votes)
                            .cloned()
                            .collect();

                        /* update reputations */
                        let reputations = not_votes
                            .iter()
                            .map(|v| (v.clone(), -INCR_REPUTATION))
                            .collect::<Vec<(String, f32)>>();
                        db::update_reputations(db, &topic, &reputations, DEFAULT_REPUTATION)
                            .ok()?;
                    }

                    if expected_votes.is_subset(&recollected_votes) {
                        /* update reputations */
                        log::debug!("Updating reputations for all votes");
                        let reputations = expected_votes
                            .iter()
                            .map(|v| (v.clone(), INCR_REPUTATION))
                            .collect::<Vec<(String, f32)>>();
                        db::update_reputations(db, &topic, &reputations, DEFAULT_REPUTATION)
                            .ok()?;

                        let filtered_votes: Vec<(String, Vote)> = votes_and_its_points
                            .into_iter()
                            .filter(|(peer_id, _)| expected_votes.contains(peer_id))
                            .collect();
                        // start  process to approve with all the votation
                        let votes: f32 = filtered_votes
                            .iter()
                            .map(|(_, vote)| vote.clone() as u8 as f32)
                            .sum();

                        let total_votes = filtered_votes.len() as f32;
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
                }
                /* we register included new validated content */
                ContentMessage::IncludeNewValidatedContent {
                    id_votation,
                    content,
                    approved,
                } => {
                    log::debug!(
                        "Received IncludeNewValidatedContent for votation: {}",
                        id_votation
                    );
                    let data_content = models::db::DataContent::new(id_votation, content, approved);
                    db::include_new_validated_content(&db, &data_content).ok()?;
                }
            }
        }

        None
    }
}
