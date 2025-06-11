use std::sync::Arc;

use libp2p::PeerId;
use sled::Db;

use crate::{db, DEFAULT_REPUTATION, INCR_REPUTATION, MessageHandler, THRESHOLD_APPROVE};
use crate::db::{DataContent, Votation};
use crate::models::messages::ContentMessage;

pub struct LinkHandler {
    peer_id: PeerId,
    db: Arc<Db>,
}

impl LinkHandler {
    pub fn new(peer_id: PeerId, db: Arc<Db>) -> Self {
        LinkHandler { peer_id, db }
    }
}

impl MessageHandler for LinkHandler {
    fn handle_message(&mut self, source_peer: PeerId, data: &[u8], topic: &str) -> Option<Vec<u8>> {
        log::info!("Received message from {}: {:?}", source_peer, String::from_utf8_lossy(data));

        let db = &self.db;

        if let Ok(res) = serde_json::from_slice(data) {
            match res {
                ContentMessage::Interested { content, id_votation } => {
                    log::info!("Received Interested message for content: {}", content);
                    let str_response: String = serde_json::to_string(&ContentMessage::InterestedResponse { id_votation: id_votation.clone() }).unwrap();
                    return Some(str_response.into_bytes());
                }
                ContentMessage::InterestedResponse { id_votation } => {
                    // interested voters
                    db::store_voter(&db, &id_votation, source_peer.to_string().as_str(), topic).ok()?; //convert result to option
                    // check if you have enough voters
                }
                ContentMessage::VoteLeaderRequest {
                    id_votation,
                    content,
                    publisher_peer_id: _,
                    voters_peer_id,
                    leader_peer_id,
                    ttl_secs: _,
                    signature: _
                } => {
                    /* set up a new status vote, check if you are the leader or not */
                    let my_self_str_peer_id = self.peer_id.to_string();
                    let mut votation = Votation::new(
                        id_votation.clone(),
                        content.clone(),
                        "pending".to_string(),
                        leader_peer_id.clone(),
                        "role_voter".to_string(),
                        vec![(my_self_str_peer_id.clone(), None)],
                    );
                    if leader_peer_id == my_self_str_peer_id {
                        votation.leader_id = my_self_str_peer_id.clone();
                        votation.my_role = "role_leader".to_string();
                    }
                    db::insert_and_update_status_vote(&db, id_votation.as_str(), &votation).ok()?;
                }
                ContentMessage::ResultVote { id_votation, result } => {
                    // we receive a vote result

                    /* are you in the votation process */
                    let Some(mut votation) = db::get_status_vote(&db, &id_votation) else {
                        return None;
                    };

                    /* are you the leader?  */
                    if votation.leader_id != self.peer_id.to_string() {
                        return None;
                    }

                    let Some(entry) = votation.votes_id.iter_mut().find(|e| e.0 == source_peer.to_string()) else {
                        return None;
                    };

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
                        let peer_ids = votation.votes_id.iter().filter(|(_, v)| v.is_none())
                            .map(|(peer_id, _)| (peer_id.clone(), -INCR_REPUTATION)).collect::<Vec<(String, f32)>>();
                        log::debug!("Decreasing reputation for peers: {:?}", peer_ids);
                        db::update_reputations(&db, &topic, &peer_ids, DEFAULT_REPUTATION).ok()?;
                        return None;
                    }

                    // we validate the content
                    let votes: f32 = votation.votes_id.iter().map(|(_, vote)| vote.unwrap_or(0.0)).sum();
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
                    let peer_ids = votation.votes_id.iter()
                        .map(|(peer_id, _)| (peer_id.clone(), INCR_REPUTATION)).collect::<Vec<(String, f32)>>();
                    db::update_reputations(&db, &topic, &peer_ids, DEFAULT_REPUTATION).ok()?;
                    return Some(serde_json::to_string(&data).ok()?.into_bytes());
                }
                ContentMessage::IncludeNewValidatedContent { id_votation, content, approved } => {
                    let data_content = DataContent::new(id_votation, content, approved);
                    db::include_new_validated_content(&db, &data_content).ok()?;
                }
            }
        }

        None
    }
}

