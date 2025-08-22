use meilisearch_sdk::client::Client;
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};
use crate::model::Link;
use uuid::Uuid;

pub const MASTER_KEY: &str = "MASTER_KEY";
const MODEL: &'static str = "links";

pub struct LLMDB {
    client: Client,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LLMDBLink {
    pub link: Link,
    pub id: String,
}

impl LLMDBLink {
    pub fn new(link: Link) -> LLMDBLink {
        let id = Uuid::new_v4();
        let id_str = id.to_string();
        LLMDBLink{link, id: id_str}
    }
}



impl LLMDB {
    pub async fn init(url_llmdb: String) -> anyhow::Result<Self> {
        let client = Client::new(url_llmdb, Some(MASTER_KEY))?;
        Ok(LLMDB{client})
    }
    
    pub async fn insert_link(&self, link: &LLMDBLink) -> anyhow::Result<String> {
        let links = self.client.index(MODEL);
        let result = links.add_documents(&[link], Some("id")).await?;
        Ok(result.status)
    }
    pub async fn query(&self, query: &str) -> anyhow::Result<Vec<LLMDBLink>> {
        let entries = self.client.index(MODEL).search().with_query(query).execute::<LLMDBLink>().await.unwrap().hits;
                    
        let mut results = Vec::new();
        for search_entry in entries {
            results.push(search_entry.result);
        }
        Ok(results)
    }

    //TODO edit link 
    pub async fn edit_link(&self, id: &str, link: &LLMDBLink) -> anyhow::Result<Option<LLMDBLink>> {
        let links = self.client.index(MODEL);
        links.add_or_update(&[link], Some("id")).await?;
        Ok(None)
    }

    pub async fn delete_link(&self, id: &str) -> anyhow::Result<()> {
        let links = self.client.index(MODEL);
        links.delete_document(id).await?;
        Ok(())
    }

}

