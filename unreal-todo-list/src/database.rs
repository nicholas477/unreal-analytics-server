use futures::executor::block_on;
use futures::StreamExt;
use mongodb::options::{FindOneAndUpdateOptions, InsertOneOptions};
use mongodb::results::InsertOneResult;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection,
};

use chrono::{NaiveDateTime, TimeDelta, Utc};
use serde_json::json;

use crate::config;

pub fn convert_list_to_bson(list: &serde_json::Value) -> Option<Document> {
    let document = mongodb::bson::to_document(&mongodb::bson::to_bson(list).ok()?).ok()?;
    // println!("Converted to json");
    // println!("document: {:#?}", document);

    let list_id = document.get_i64("ListID").ok()?;
    let list_name = document.get_str("ListName").ok()?;
    let serialized_list = document.get_document("SerializedList").ok()?;

    Some(doc!["ListID": list_id, "ListName": list_name, "SerializedList": serialized_list])
}

#[derive(Debug)]
pub struct Database {
    pub client: mongodb::Client,
    pub database: mongodb::Database,
}

impl Database {
    pub fn print_info(&self) {
        println!("Printing database names...");
        let db_names = block_on(self.client.list_database_names(None, None));
        for db_name in db_names.unwrap() {
            println!("{}", db_name);
        }

        println!("Printing collection names in {}", self.database.name());
        for collection_name in block_on(self.database.list_collection_names(None)).unwrap() {
            println!("{}", collection_name);
        }
    }

    pub async fn update_todo_list(&self, list: &Document) -> mongodb::error::Result<()> {
        let collection = self.database.collection::<Document>("todolists");

        let list_id = list
            .get_i64("ListID")
            .or(Err(mongodb::error::Error::custom("JSON Error")))?;

        let list_name = list
            .get_str("ListName")
            .or(Err(mongodb::error::Error::custom("JSON Error")))?;

        let filter = doc! {
            "ListID": list_id
        };

        let update = doc! {
            "$set": list
        };

        let options = FindOneAndUpdateOptions::builder()
            .upsert(Some(true))
            .build();

        collection
            .find_one_and_update(filter, update, Some(options))
            .await?;

        println!("Updated todo list: {}", list_name);

        Ok(())
    }

    pub async fn delete_todo_list(&self, list_id: &i64) -> mongodb::error::Result<()> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = doc! {
            "ListID": list_id
        };

        collection.delete_one(filter, None).await?;

        println!("Deleted todo list: {}", list_id);

        Ok(())
    }

    pub async fn get_todo_lists(&self) -> mongodb::error::Result<Vec<Document>> {
        let collection = self.database.collection::<Document>("todolists");

        let todo_lists = collection.find(None, None).await?;

        let vec = todo_lists.collect::<Vec<_>>().await;
        vec.into_iter().collect()
    }
}

pub fn connect_to_db(config: &config::Config) -> Database {
    let client_options = ClientOptions::parse(&config.mongodb_connection_string).unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("local");

    Database {
        client,
        database: db,
    }
}
