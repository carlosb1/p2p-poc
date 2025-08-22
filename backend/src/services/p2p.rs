use std::time::Duration;
use axum::serve::Serve;
use mongodb::event::sdam::ServerClosedEvent;
use messages_p2p::{Keypair, PeerId};
use messages_p2p::p2p::api::APIClient;

//use messages_p2p::p2p::api::APIClient;
pub async fn init(){
    /*
    */
}

pub async fn new_remote_topic(topic: &str) {
    /*
    .remote_new_topic(&topic)
        .await

     */
}

pub async fn register_topic(title: &str, description: &str) {
/*
    let topic = messages_p2p::Topic {
        name: name.clone(),
        description,
    };
    let mut guard = self.client.lock().await;
    guard
        .register_topic(&topic)
 */
}

pub fn get_my_topics() -> Vec<String> {
    /*
                let mut guard = self.client.lock().await;
            guard
                .get_my_topics()
                .await
                .iter()
                .cloned()
                .map(Topic::from)
                .collect::<Vec<Topic>>()

     */
    vec![]
}

pub fn new_key_available(topic: &str, content: &str) -> String {
    "".to_string()
}

pub async fn get_runtime_content_to_validate()  {

}

//pub fn add_vote(&self, id_votation: String, topic: String, vote: Vote) -> PyResult<()> {

//pub fn voters(&self, key: String, topic: String) -> PyResult<Vec<String>> {

//    pub fn get_reputation(&self, peer_id: String, topic: String) -> PyResult<f32> {

// pub fn get_reputations(&self, topic: String) -> PyResult<Vec<Reputation>> {


//     pub fn all_content(&self) -> PyResult<Vec<DataContent>> {

//pub fn validate_content(&self, topic: String, content: String) -> PyResult<String> {

//pub fn get_status_vote(&self, key: String) -> PyResult<Option<Votation>> {

//pub fn get_status_voteses(&self) -> PyResult<Vec<Votation>> {

pub struct P2PClient {
   // pub client: APIClient,
}


impl P2PClient {
    pub fn new(server_peer_id: &str, server_address: &str) -> anyhow::Result<Self> {
        let keypair = Keypair::generate_ed25519();
        let peer_id = PeerId::from(keypair.public());
        let peer_id_str = peer_id.to_string();
        
        /*
        let client = APIClient::from_server_params(
            keypair,
            Some(peer_id_str.clone()),
            &server_peer_id,
            &server_address,
        )?;
        */
//        Ok(Self{client})
        Ok(P2PClient {})
    }

    pub async fn start(&self) { 
       // self.client.start().await.expect("Client could not start");
    }


    pub async fn get_runtime_content_to_validate(&self)  -> Vec<(String, String, String, Duration)>  {
        vec![]
    }
}