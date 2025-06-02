use std::hash;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

use libp2p::{identity, PeerId};
use libp2p::identity::ed25519::Keypair;
use sled::Db;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::sleep;

use messages_types::ChatCommand;

use crate::db::{get_votes, init_db, StatusVote};
use crate::models::messages::ContentMessage;
use crate::protocol::MessageHandler;

mod db;
mod models;
pub mod protocol;


const TIMEOUT_SECS: u64 = 10; // Set a timeout for the voting process
const MEMBERS_FOR_CONSENSUS: usize = 5; // Number of members required for consensus
const MIN_REPUTATION_THRESHOLD: f32 = 80.0; // Minimum reputation threshold for voters

const THRESHOLD_APPROVE: f32 = 0.6; // 60% threshold for approval;

pub struct LinkClient {
    peer_id: PeerId,
    command_tx: mpsc::Sender<ChatCommand>,
    inner_handler: Arc<dyn MessageHandler + Send + Sync>,
}

pub struct LinkHandler {
    peer_id: PeerId,
}

impl LinkHandler {
    pub fn new(peer_id: PeerId) -> Self {
        LinkHandler { peer_id }
    }
}

impl MessageHandler for LinkHandler {
    fn handle_message(&mut self, source_peer: PeerId, data: &[u8]) -> Option<Vec<u8>> {
        log::info!("Received message from {}: {:?}", source_peer, String::from_utf8_lossy(data));
        let Ok(db) = db::init_db(self.peer_id.to_string().as_str()) else {
            log::error!("Failed to initialize database");
            return None;
        };


        if let Ok(res) = serde_json::from_slice(data) {
            match res {
                ContentMessage::Interested { content, id_votation } => {
                    log::info!("Received Interested message for content: {}", content);
                    //TODO add extravalidation that we are registered in this topic

                    let str_response: String = serde_json::to_string(&ContentMessage::InterestedResponse { id_votation: id_votation.clone() }).unwrap();
                    return Some(str_response.into_bytes());
                }
                ContentMessage::InterestedResponse { id_votation } => {
                    log::info!("Received InterestedResponse message for this votation: {:?} from this peer {:?}", id_votation, source_peer.to_string());
                    db::save_interested_for_votation(&id_votation, &source_peer.to_string());
                    // we can check who is interested in this votation
                    // Here you would typically handle the interested response, e.g., store it or respond
                }
                ContentMessage::VoteLeaderRequest {
                    id_votation,
                    content,
                    publisher_peer_id,
                    voters_peer_id,
                    leader_peer_id,
                    ttl_secs,
                    signature
                } => {
                    log::info!("Received VoteLeaderRequest for content: {}, for id_votation: {}", content, &id_votation);
                    let my_self_str_peer_id = self.peer_id.to_string();
                    if leader_peer_id == my_self_str_peer_id {
                        log::debug!("I am the leader for this id_votation: {}", &id_votation);
                        let _ = db::store_new_status_vote(&db, &id_votation, &content, "pending", &my_self_str_peer_id, "role_leader", &voters_peer_id);
                    } else if voters_peer_id.contains(&my_self_str_peer_id)
                    {
                        log::debug!("I was chosen as a voter for this id_votation: {}", &id_votation);
                        let _ = db::store_new_status_vote(&db, &id_votation, &content, "pending", &leader_peer_id, "role_voter", &voters_peer_id);
                    } else {
                        log::debug!("You didnt was chosen as a voter or leader for this id_votation: {}", &id_votation);
                    }
                }
                ContentMessage::ResultVote { id_votation, result } => {
                    log::info!("Received ResultVote for id_votation: {:?}, result: {:?}", id_votation, result);
                    let Some(mut status_vote) = db::get_status_vote(&db, &id_votation) else {
                        log::warn!("No status found for id_votation: {}", id_votation);
                        return None;
                    };

                    //TODO review if it is necessary role param
                    if status_vote.leader_id != self.peer_id.to_string() {
                        log::debug!("I am not the leader, I am not responsible for this");
                        return None;
                    }
                    // check our reputation, we check the vote is in the list, we check our blacklist
                    let mut posi_found = status_vote.votes_id.iter().find(|(peer_id, _)| *peer_id == source_peer.to_string());
                    if !posi_found.is_none() {
                        log::warn!("Vote from peer {} is not in the list of votes for id_votation: {}", source_peer, id_votation);
                        return None;
                    }

                    let mut found = posi_found.unwrap();
                    if found.1.is_some() {
                        log::warn!("Vote from peer {} is already registered for id_votation: {}", source_peer, id_votation);
                        return None;
                    }
                    // check blacklist and reputation?
                    //TODO votes includes the leader?
                    let result_vote = result as u8;

                    if let Some(entry) = status_vote.votes_id.iter_mut().find(|e| e.0 == source_peer.to_string()) {
                        entry.1 = Some(result_vote as f32);
                    }

                    db::update_status_vote(&db, &status_vote).expect(format!("It could not update the vote process for {source_peer}").as_str());

                    let all_votes = status_vote.votes_id.iter().all(|&(_, vote)| vote.is_some());
                    if !all_votes {
                        log::debug!("Not all votes are present for id_votation: {}", id_votation);
                        return None;
                    }
                    // Check if we have enough votes to make a decision
                    let votes: f32 = status_vote.votes_id.iter().map(|(_, vote)| vote.unwrap_or(0.0)).sum();
                    let total_votes = status_vote.votes_id.len() as f32;

                    let percent_accept = votes / total_votes;
                    let approved = percent_accept >= THRESHOLD_APPROVE; // 60% threshold for approval

                    log::debug!("We van get a conlusion for id_votation: {}, votes: {}, total_votes: {}, percent_accept: {}, approved: {}",
                        id_votation, votes, total_votes, percent_accept, approved);

                    let data = ContentMessage::IncludeNewValidatedContent {
                        id_votation: id_votation.clone(),
                        content: status_vote.content.clone(),
                    };

                    update_reputation_db(&db, &status_vote);
                    return Some(serde_json::to_string(&data).expect("Failed to serialize IncludeNewValidatedContent message").into_bytes());
                }
                ContentMessage::IncludeNewValidatedContent { id_votation, content } => {
                    log::info!("Received IncludeNewValidatedContent for id_votation: {}, content: {}", id_votation, content);
                    db::include_new_validated_content(&db, &id_votation, &content)
                        .expect("Failed to include new validated content in the database");
                }
                _ => {
                    log::info!("Received other message type: {:?}", res);
                    // Handle other message types as needed
                }
            }
        }

        // Here you would typically process the message and return a response if needed
        None
    }
}

fn update_reputation_db(p0: &Db, p1: &StatusVote) {
    todo!()
}

//services functions
fn get_reputation_from_me(my_peer_id: &PeerId) -> Option<f32> {
    let topics = db::get_reputation(my_peer_id);
    topics
}

fn get_reputation_list() -> Vec<(String, f32)> {
    let reputations: Vec<(String, f32)> = db::get_reputations();
    reputations
}


impl LinkClient {
    pub async fn register_topic(&self, topic: &str) -> anyhow::Result<()> {
        // Send the register command to the channel
        self.command_tx.send(ChatCommand::Subscribe(topic.to_string())).await?;
        Ok(())
    }

    pub async fn send(&self, topic: String, message: &ContentMessage) -> anyhow::Result<()> {
        // Send the message to the channel
        self.command_tx.send(ChatCommand::Publish(topic, serde_json::to_vec(message)?)).await?;
        Ok(())
    }

    pub async fn send_one_to_one(&self, topic: String, message: &ContentMessage) -> anyhow::Result<()> {
        // Send the message to the channel
        self.command_tx.send(ChatCommand::Publish(topic, serde_json::to_vec(message)?)).await?;
        Ok(())
    }

    pub async fn quit(&self) -> anyhow::Result<()> {
        // Send the quit command to the channel
        self.command_tx.send(ChatCommand::Quit).await?;
        Ok(())
    }
    pub fn new(peer_id: PeerId, tx: Sender<ChatCommand>) -> Self {
        let db = db::init_db(peer_id.to_string().as_str()).expect("Failed to initialize database");
        LinkClient {
            peer_id,
            command_tx: tx,
            inner_handler: Arc::new(LinkHandler::new(peer_id.clone())),
        }
    }
    pub fn peer_id(&self) -> PeerId {
        self.peer_id.clone()
    }
}


#[tokio::main]
async fn main() {
    let topic = "topic:tech";
    let content = "hash_of_news_item_xyz"; // Could be hash(link+timestamp)

    let keypair = identity::Keypair::generate_ed25519();
    let peer_id = keypair.public().to_peer_id();
    log::debug!("My Peer ID: {:?}", peer_id);

    // buffer is 32 size
    let channel = mpsc::channel(32);


    // 1. Register this node as available to vote in this topic
    let client = LinkClient::new(peer_id, channel.0);
    client.register_topic(topic).await.expect("Failed to register topic");
    // 2. User publishes a new content that needs voting

    let db = init_db(peer_id.to_string().as_str()).expect("Failed to initialize database");
    // 2.b we check if it was voted
    let key_for_checking = db::create_key_without_status(topic, content);
    if let Ok(found) = db.contains_key(key_for_checking.clone()) {
        if found {
            log::debug!("Key found in DB: {} you can not revote in this version", &key_for_checking);
            return;
        }
    } else {
        log::error!("Failed to check key in DB: {}", &key_for_checking);
        return;
    }


    /* We start a new process for voting as a new thread*/

    let key = db::create_key_for_voting_db(content, topic, "pending", 1);

    /* start process for voting */
    let interested = ContentMessage::Interested {
        id_votation: key.clone(),
        content: content.to_string(),
    };
    client.send(topic.to_string(), &interested).await.expect("Failed to send interested message");

    tokio::spawn(async move {
        //let vote_request_id = generate_vote_request_id(content_id);
        // 3. Discover potential voters from DHT or local registry
        let db = init_db(peer_id.to_string().as_str()).expect("Failed to initialize database");

        let check_interval = Duration::from_millis(500);
        let timeout = Duration::from_secs(TIMEOUT_SECS);
        let start = tokio::time::Instant::now();

        loop {
            if start.elapsed() >= timeout {
                println!("Timeout reached without finding match.");
                break;
            }

            if let Ok(votes) = db::get_votes(&db, &key) {
                /* filtered for votes  */
                let mut filtered_votes: Vec<(String, f32)> = votes
                    .iter()
                    .filter(|(_, reputation)| *reputation >= MIN_REPUTATION_THRESHOLD)
                    .cloned()
                    .collect();
                /* We check if we have enough peers for voting */
                if filtered_votes.len() >= MEMBERS_FOR_CONSENSUS {
                    /* we choose leader with high reput */
                    // 4. Load local reputation DB and filter the best
                    // 5. Select N nodes to form the voting group (e.g., 5-7)
                    // 6. Choose a leader (deterministic, e.g. highest rep or first in sorted list)
                    let filtered_votes: Vec<(String, f32)> = filtered_votes[0..MEMBERS_FOR_CONSENSUS].to_vec();
                    let leader_peer = filtered_votes.first().expect("It could not choose a leader from a non empty list");


                    // 7. Create the vote request to send to the group
                    let vote_request = ContentMessage::new_vote_leader_request(key.clone()
                                                                               , content.to_string()
                                                                               , client.peer_id().to_string()
                                                                               , filtered_votes.iter().map(|(peer_id, _)| peer_id.clone()).collect()
                                                                               , leader_peer.0.clone()
                                                                               , 60
                                                                               , &keypair).expect("Failed to create vote request");


                    // 8. Send the vote request to the leader via libp2p::request_response
                    client.send(topic.to_string(), &vote_request).await.expect("Failed to send client message");
                    
                    return;
                    // We have enough votes, proceed with the voting logic
                } else {
                    log::debug!("Current votes: {:?}, waiting for more...", votes);
                }
            }

            sleep(check_interval).await;
        }
    });
}


fn register_myself_to_topic(topic: &str) {}