use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[derive(Clone)]
pub struct Link {
    pub url: String,
    pub tags: Vec<String>,
    pub description: Option<String>,
}