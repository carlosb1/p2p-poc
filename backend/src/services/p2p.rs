use std::time::Duration;
use axum::serve::Serve;
use futures_util::FutureExt;
use mongodb::event::sdam::ServerClosedEvent;
use messages_p2p::{DataContent, Keypair, PeerId, Votation, Vote};
use messages_p2p::p2p::api::APIClient;
use serde::Deserialize;
//use messages_p2p::p2p::api::APIClient;

#[derive(Debug, Deserialize)]
struct TrackerInfo {
    id: String,
    addresses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ConnectionData {
    pub server_id: String,
    pub server_address: Vec<String>,
    pub client_id: Option<String>,
}

pub const MAGIC_SERVER_LINK_ADDRESS: &str = "http://34.244.185.56:3000/tracker";
pub async fn download_server_params_from_address() -> ConnectionData {
    let response = reqwest::get(MAGIC_SERVER_LINK_ADDRESS).await.unwrap();
    let tracker_info: TrackerInfo = response.json().await.unwrap();
    let connection_data = ConnectionData {
        server_id: tracker_info.id,
        server_address: tracker_info.addresses.clone(),
        client_id: None,
    };
    connection_data

}

pub struct P2PClient {
   // pub client: APIClient,
    
    pub counter: u32,
    pub client: APIClient,
}


impl P2PClient {
    pub fn new(server_peer_id: &str, server_address: &str) -> anyhow::Result<Self> {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let peer_id_str = peer_id.to_string();
        

        let client = APIClient::from_server_params(
            keypair,
            Some(peer_id_str.clone()),
            &server_peer_id,
            &server_address,
        )?;
        Ok(P2PClient {counter: 0, client})
    }
    
    pub fn new_key_available(&self, topic: &str, content: &str) -> anyhow::Result<String> {
        self.client.new_key_available(topic, content)
    }


    pub async fn start(&self) { 
       self.client.start().await.expect("Client could not start");
    }

    pub async fn new_remote_topic(&self, topic: &str, description: &str) -> anyhow::Result<()> {
        let topic = messages_p2p::Topic {
            name: topic.to_string(),
            description: description.to_string(),
        };
        self.client.remote_new_topic(&topic).await
    }

    pub async fn register_topic(&self, title: &str, description: &str) -> anyhow::Result<()> {
        let topic = messages_p2p::Topic {
            name: title.to_string(),
            description: description.to_string(),
        };
        self.client.register_topic(&topic).await
    }

    pub async fn get_my_topics(&self) -> Vec<(String,String)> {
        self.client.get_my_topics().await.iter().map(|t| (t.name.clone(), t.description.clone())).collect()
    }

    pub fn get_reputations(&self, topic: String) -> anyhow::Result<Vec<(String,f32)>> {
        let reputations = self.client.get_reputations(topic.as_str());
        Ok(vec![])
    }

    pub fn get_reputation(&self, peer_id: String, topic: String) -> anyhow::Result<f32>  {
        self.client.get_reputation(peer_id.as_str(), topic.as_str())
        
    }
    

    pub async fn get_runtime_content_to_validate(&self)  -> Vec<(String, String, String, Duration)>  {
        let elems = self.client.get_runtime_content_to_validate().await;
        let counter_str = self.counter.to_string();
        let values = vec![(format!("key{:}",counter_str), 
              format!("topic{:}",counter_str), 
              format!("content{:}",counter_str),
              Duration::from_secs(self.counter as u64))];
        
        values
    }

    pub async fn add_vote(&self, id_votation: String, topic: String, vote: bool) -> anyhow::Result<()> {
        let parsed_vote =  if vote {Vote::Yes} else {Vote::No};
        let res = self.client.add_vote(id_votation.as_str(), topic.as_str(), parsed_vote).await;
        res
    }

    pub fn all_content(&self) -> Vec<DataContent> {
        self.client.all_content()
    }

    pub async fn voters(&self, key: String, topic: String) -> anyhow::Result<Vec<String>> {
        let values = self.client.voters(&key, &topic).await;
        Ok(vec![])
    }
    pub async fn validate_content(&self, key: String, topic: String, content: String) -> anyhow::Result<String> {
        let res = self.client.validate_content(&key, &topic, &content).await;
        Ok("".to_string())
    }
    pub fn get_status_votes(&self, key: String) -> Option<Votation> {
        self.client.get_status_vote(key.as_str())
    }
    pub fn get_status_voteses(&self) -> Vec<Votation> {
        self.client.get_status_voteses()
    }
}

#[cfg(test)]
mod tests {
    use tokio::time;
    use super::*;
    #[tokio::test]
    pub async fn check_download_server_link() {
        let connection_data = download_server_params_from_address().await;
        println!("server_address={:?}", connection_data.server_address);
        println!("server_id={:?}", connection_data.server_id);
        let server_id = connection_data.server_id.clone();
        let index = connection_data.server_address.len() - 2;
        let server_address = connection_data.server_address.clone().get(index).unwrap().clone();
        println!("server_address={:?}", server_address);
        let p2p_client = P2PClient::new(server_id.as_str(), server_address.as_str()).unwrap();
        p2p_client.start().await;

        time::sleep(Duration::from_secs(5)).await;
        
        p2p_client.new_remote_topic("test", "test").await.unwrap();

        println!("listener");
        time::sleep(Duration::from_secs(5)).await;
        
        let values = p2p_client.get_runtime_content_to_validate().await;

        println!("values={:?}", values);
        p2p_client.start().await;

    }
}