use std::collections::HashMap;
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
pub struct WSTopic {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct WSReputation {
    pub peer_id: String,
    pub reputation: f32,
}



#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct WSContentData {
    pub my_topics: Vec<WSTopic>,
    pub content: Vec<Link>,
    pub content_to_validate: Vec<Link>,
    pub reputations_by_key: HashMap<String, Vec<WSReputation>>,
    
}