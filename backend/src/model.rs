use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Link {
    pub url: String,
    pub tags: Vec<String>,
    pub description: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Vote {
    pub id: String,
    pub topic: String,
    pub vote: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Topic {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Reputation {
    pub peer_id: String,
    pub reputation: f32,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Content {
    pub id_votation: String,
    pub content: String,
    pub approved: bool,
}
 
#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct ContentToValidate {
    pub id_votation: String,
    pub topic: String,
    pub content: String,
    pub duration: Duration,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct WSContentData {
    pub my_topics: Vec<Topic>,
    pub content: Vec<Content>,
    pub content_to_validate: Vec<ContentToValidate>,
    pub my_pending_content: Vec<Content>,
    pub voters_by_key: HashMap<String, Votation>,
    pub reputations_by_topic: HashMap<String, Vec<Reputation>>,
    
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Votation {
    pub id_votation: String,
    pub timestamp: i64,
    pub content: String,
    pub status: String,
    pub leader_id: String,
    pub my_role: String,
    pub votes_id: Vec<(String, Option<f32>)>,
}

