use pyo3::pyclass;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, thiserror::Error)]
pub enum APIError {
    #[error("Error to connect to the server={addr} msg={msg}")]
    ConnectionError { addr: String, msg: String },
    #[error("Concurrency error msg={msg}")]
    ConcurrencyError { msg: String },
    #[error("RuntimeError msg={msg}")]
    RuntimeError { msg: String },
}
#[pyclass]
pub struct RuntimePendingContent {
    #[pyo3(get, set)]
    pub(crate) key: String,
    #[pyo3(get, set)]
    pub(crate) topic: String,
    #[pyo3(get, set)]
    pub(crate) content: String,
    #[pyo3(get, set)]
    pub(crate) wait_timeout: SystemTime,
}

#[pyclass]
pub struct Topic {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub description: String,
}

impl From<messages_p2p::Topic> for Topic {
    fn from(value: messages_p2p::Topic) -> Self {
        Topic {
            name: value.name,
            description: value.description,
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct ConnectionData {
    #[pyo3(get, set)]
    pub server_id: String,
    #[pyo3(get, set)]
    pub server_address: Vec<String>,
    #[pyo3(get, set)]
    pub client_id: Option<String>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Vote {
    #[pyo3(get, set)]
    pub good: bool,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct VoteId {
    #[pyo3(get, set)]
    pub key: String,
    #[pyo3(get, set)]
    pub value: f32,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Votation {
    #[pyo3(get, set)]
    pub id_votation: String,
    #[pyo3(get, set)]
    pub timestam: SystemTime,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub status: String,
    #[pyo3(get, set)]
    pub leader_id: String,
    #[pyo3(get, set)]
    pub my_role: String,
    #[pyo3(get, set)]
    pub votes_id: Vec<VoteId>,
}

impl From<messages_p2p::Votation> for Votation {
    fn from(v: messages_p2p::Votation) -> Self {
        Self {
            id_votation: v.id_votation,
            timestam: UNIX_EPOCH
                + std::time::Duration::from_millis(v.timestamp.timestamp_millis() as u64),
            content: v.content,
            status: v.status,
            leader_id: v.leader_id,
            my_role: v.my_role,
            votes_id: v
                .votes_id
                .into_iter()
                .filter_map(|(k, opt)| opt.map(|v| VoteId { key: k, value: v }))
                .collect(),
        }
    }
}

impl From<&messages_p2p::Votation> for Votation {
    fn from(temp_v: &messages_p2p::Votation) -> Self {
        let v = temp_v.clone();
        Self {
            id_votation: v.id_votation,
            timestam: UNIX_EPOCH
                + std::time::Duration::from_millis(v.timestamp.timestamp_millis() as u64),
            content: v.content,
            status: v.status,
            leader_id: v.leader_id,
            my_role: v.my_role,
            votes_id: v
                .votes_id
                .into_iter()
                .filter_map(|(k, opt)| opt.map(|v| VoteId { key: k, value: v }))
                .collect(),
        }
    }
}

#[pyclass]
#[derive(Debug, Clone)]
enum StateContent {
    Approved,
    Rejected,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Reputation {
    #[pyo3(get, set)]
    pub(crate) name: String,
    #[pyo3(get, set)]
    pub(crate) repu: f32,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct DataContent {
    #[pyo3(get, set)]
    pub id_votation: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub approved: StateContent,
}

#[pyclass]
#[derive(Debug, Clone)]
pub enum VoteStatusKind {
    Pending,
    Accepted,
    Rejected,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct VoteStatus {
    #[pyo3(get, set)]
    pub kind: VoteStatusKind,
    #[pyo3(get, set)]
    pub pending_data: Option<Vec<Pair>>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct Pair {
    #[pyo3(get, set)]
    pub key: String,
    #[pyo3(get, set)]
    pub value: f32,
}

impl From<&messages_p2p::VoteStatus> for VoteStatus {
    fn from(status: &messages_p2p::VoteStatus) -> Self {
        match status {
            messages_p2p::VoteStatus::Pending(pairs) => VoteStatus {
                kind: VoteStatusKind::Pending,
                pending_data: Some(
                    pairs
                        .into_iter()
                        .map(|(k, v)| Pair {
                            key: k.to_string(),
                            value: *v,
                        })
                        .collect(),
                ),
            },
            messages_p2p::VoteStatus::Accepted => VoteStatus {
                kind: VoteStatusKind::Accepted,
                pending_data: None,
            },
            messages_p2p::VoteStatus::Rejected => VoteStatus {
                kind: VoteStatusKind::Rejected,
                pending_data: None,
            },
        }
    }
}

impl From<messages_p2p::DataContent> for DataContent {
    fn from(d: messages_p2p::DataContent) -> Self {
        Self {
            id_votation: d.id_votation,
            content: d.content,
            approved: if d.approved == messages_p2p::StateContent::Approved {
                StateContent::Approved
            } else {
                StateContent::Rejected
            },
        }
    }
}
