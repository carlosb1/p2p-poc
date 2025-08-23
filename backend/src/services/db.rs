use mongodb::bson::{doc, oid::ObjectId, to_document};
use mongodb::{options::ClientOptions, Client, Collection};
use std::str::FromStr;
use futures_util::{StreamExt, TryStreamExt};
use crate::model::Link;

#[derive(Clone, Debug)]
pub struct DB {
    pub link_collection: Collection<Link>,
}


impl DB {
    pub async fn init(mongodb_uri: String, database_name: String, collection_name: String) -> anyhow::Result<Self> {
        let mut client_options = ClientOptions::parse(mongodb_uri).await?;
        client_options.app_name = Some(database_name.to_string());

        let client = Client::with_options(client_options)?;
        let database = client.database(database_name.as_str());

        let link_collection = database.collection(collection_name.as_str());

        println!("✅ Database connected successfully");

        Ok(Self {
            link_collection,
        })
    }

    pub async fn fetch_links(&self) -> anyhow::Result<Vec<Link>> {
        let mut cursor = self
            .link_collection.find(doc! {}).await.map_err(anyhow::Error::new)?;
        /* results */
        let mut result: Vec<Link> = Vec::new();
        while let Some(doc) = cursor.next().await {
            result.push(doc?);
        }
        Ok(result)
    }

    pub async fn insert_link(&self, link: &Link) -> anyhow::Result<String> {
        let result = self.link_collection.insert_one(link.clone()).await.map_err(anyhow::Error::new)?;
        let new_id = result
            .inserted_id
            .as_object_id()
            .expect("issue with new _id");
        Ok(new_id.to_string())
    }
    pub async fn get_link(&self, id: &str) -> anyhow::Result<Option<Link>> {
        let find = self.link_collection.find_one(doc! {"_id": id}).await.map_err(anyhow::Error::new)?;
        Ok(find)
    }

    pub async fn get_links(&self, ids: &[String]) -> anyhow::Result<Vec<Link>> {
        let object_ids: Vec<ObjectId> = ids.iter()
            .filter_map(|id| ObjectId::parse_str(id).ok())
            .collect();

        if object_ids.is_empty() {
            return Ok(Vec::new());
        }
        
        
        let cursor = self.link_collection
            .find(doc! { "_id": { "$in": object_ids } })
            .await
            .map_err(anyhow::Error::new)?;

        let results: Vec<Link> = cursor
            .try_collect()
            .await
            .map_err(anyhow::Error::new)?;
        Ok(results)
    }

    pub async fn edit_link(&self, id: &str, link: &Link) -> anyhow::Result<Option<Link>> {
        let update_doc = doc! {
            "$set": to_document(&link)?
        };
        let result = self.link_collection.find_one_and_update(doc! {"_id": id}, update_doc).await.map_err(anyhow::Error::new)?;
        Ok(result)
    }

    pub async fn delete_link(&self, id: &str) -> anyhow::Result<u64> {
        let oid = ObjectId::from_str(id).map_err(anyhow::Error::new)?;
        let filter = doc! {"_id": oid };
        let result = self
            .link_collection.delete_one(filter)
            .await
            .map_err(anyhow::Error::new)?;
        Ok(result.deleted_count)

    }
}