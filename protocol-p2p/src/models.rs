use base64::Engine;
use serde::{Deserialize, Serialize};

pub mod db {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub enum VoteStatus {
        Pending(
            Vec<(String, f32)>
        ),
        Accepted,
        Rejected,
    }
}


pub mod messages {
    use anyhow::anyhow;
    use base64::Engine;
    use base64::engine::general_purpose;
    use libp2p::identity::Keypair;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize)]
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
        ResultVote { id_votation: String, result: Vote },
        IncludeNewValidatedContent { id_votation: String, content: String, approved: bool },

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
            let msg_bytes = serde_json::to_vec(&temp_msg).unwrap();

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
