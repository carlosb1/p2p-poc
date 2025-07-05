use serde::{Deserialize, Serialize};

pub mod db {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Serialize};
    use std::fmt;

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Topic {
        pub name: String,
        pub description: String,
    }

    impl Topic {
        pub fn new(name: &str, description: &str) -> Self {
            Topic {
                name: name.to_string(),
                description: description.to_string(),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Votation {
        pub id_votation: String,
        pub timestamp: DateTime<Utc>,
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
                timestamp: Utc::now(),
                content,
                status,
                leader_id,
                my_role,
                votes_id,
            }
        }
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub enum VoteStatus {
        Pending(Vec<(String, f32)>),
        Accepted,
        Rejected,
    }

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

    // Topic
    impl fmt::Display for Topic {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "Topic: {}\nDescription: {}", self.name, self.description)
        }
    }

    // Votation
    impl fmt::Display for Votation {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let votes: Vec<String> = self
                .votes_id
                .iter()
                .map(|(id, score)| match score {
                    Some(s) => format!("{}: {:.2}", id, s),
                    None => format!("{}: No vote", id),
                })
                .collect();

            write!(
                f,
                "Votation [{}]\nContent: {}\nStatus: {}\nLeader: {}\nRole: {}\nTimestamp: {}\nVotes:\n  {}",
                self.id_votation,
                self.content,
                self.status,
                self.leader_id,
                self.my_role,
                self.timestamp,
                votes.join("\n  ")
            )
        }
    }

    // VoteStatus
    impl fmt::Display for VoteStatus {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                VoteStatus::Pending(votes) => {
                    let votes_str: Vec<String> = votes
                        .iter()
                        .map(|(id, val)| format!("{}: {:.2}", id, val))
                        .collect();
                    write!(f, "Pending Votes:\n  {}", votes_str.join("\n  "))
                }
                VoteStatus::Accepted => write!(f, "Status: Accepted"),
                VoteStatus::Rejected => write!(f, "Status: Rejected"),
            }
        }
    }

    // StateContent
    impl fmt::Display for StateContent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                StateContent::Approved => write!(f, "Approved"),
                StateContent::Rejected => write!(f, "Rejected"),
            }
        }
    }

    // DataContent
    impl fmt::Display for DataContent {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "Votation ID: {}\nContent: {}\nState: {}",
                self.id_votation, self.content, self.approved
            )
        }
    }
}

pub mod messages {
    use base64::engine::general_purpose;
    use base64::Engine;
    use libp2p::gossipsub::IdentTopic;
    use libp2p::identity::Keypair;
    use once_cell::sync::Lazy;
    use serde::{Deserialize, Serialize};

    pub static DEFAULT_TOPIC: Lazy<IdentTopic> = Lazy::new(|| IdentTopic::new("chat-room"));

    #[derive(Debug, Serialize, Deserialize, Clone, Copy)]
    pub enum Vote {
        Yes = 1,
        No = 0,
    }

    #[derive(Debug, Serialize, Deserialize)]
    #[serde(tag = "type")] // para serializar como { "type": "RequestVote", ... }
    pub enum ContentMessage {
        Interested {
            content: String,
            id_votation: String,
        },
        InterestedResponse {
            id_votation: String,
        },
        VoteLeaderRequest {
            id_votation: String,
            content: String,
            publisher_peer_id: String,
            voters_peer_id: Vec<String>,
            leader_peer_id: String,
            ttl_secs: u64,
            signature: String,
        },
        ResultVote {
            id_votation: String,
            result: Vote,
        },
        IncludeNewValidatedContent {
            id_votation: String,
            content: String,
            approved: bool,
        },
        RegisterTopic {
            topic: String,
        },
    }

    impl ContentMessage {
        pub fn new_vote_leader_request(
            id_votation: String,
            content: String,
            publisher_peer_id: String,
            voters_peer_id: Vec<String>,
            leader_peer_id: String,
            ttl_secs: u64,
            keypair: &Keypair,
        ) -> anyhow::Result<Self> {
            // Serialize the message without the signature
            let temp_msg = serde_json::json!({
                "type": "VoteLeaderRequest",
                "id_votation": id_votation,
                "content": content,
                "publisher_peer_id": publisher_peer_id,
                "voters": voters_peer_id,
                "leader": leader_peer_id,
                "ttl_secs": ttl_secs,
            });

            // Convert to canonical string
            let msg_bytes = serde_json::to_vec(&temp_msg)?;

            // Sign with the publisher's private key
            let signature_bytes = keypair.sign(&msg_bytes)?;
            let signature_b64 = general_purpose::STANDARD.encode(signature_bytes);

            Ok(ContentMessage::VoteLeaderRequest {
                id_votation,
                content,
                publisher_peer_id,
                voters_peer_id,
                leader_peer_id,
                ttl_secs,
                signature: signature_b64,
            })
        }
    }
}
