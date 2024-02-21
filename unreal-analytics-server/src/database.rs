use futures::executor::block_on;
use mongodb::bson::{doc, to_bson, Document};
use mongodb::results::InsertOneResult;
use mongodb::{options::ClientOptions, Client};
use rocket::serde::json::{Json, Value};
use std::collections::HashMap;

use crate::config;

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

    pub async fn add_session(&self, session: &Value) -> mongodb::error::Result<InsertOneResult> {
        let collection = self.database.collection::<Document>("sessions");

        let document = mongodb::bson::to_bson(session).unwrap();
        let document2 = mongodb::bson::to_document(&document).unwrap();
        println!("Document: {:#?}", document2);

        collection.insert_one(document2, None).await
    }
}

pub fn connect_to_db(config: &config::Config) -> Database {
    let client_options = ClientOptions::parse(&config.mongodb_connection_string).unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("local");

    Database {
        client: client,
        database: db,
    }
}
