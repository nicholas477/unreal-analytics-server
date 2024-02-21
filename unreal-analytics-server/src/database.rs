use futures::executor::block_on;
use mongodb::bson::{doc, to_bson, Document};
use mongodb::{options::ClientOptions, Client};
use rocket::serde::json::{Json, Value};
use std::collections::HashMap;

pub struct Database {
    client: mongodb::Client,
    database: mongodb::Database,
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

    pub fn add_session(&self, session: &Value) {
        let collection = self.database.collection::<Document>("sessions");

        let document = mongodb::bson::to_bson(session).unwrap();
        let document2 = mongodb::bson::to_document(&document).unwrap();
        println!("Document: {:#?}", document2);

        block_on(collection.insert_one(document2, None)).unwrap();
    }
}

pub fn connect_to_db() -> Database {
    let client_options = ClientOptions::parse("mongodb://localhost:27017").unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("local");

    // println!("Printing database names...");
    // let db_names = block_on(client.list_database_names(None, None));
    // for db_name in db_names.unwrap() {
    //     println!("{}", db_name);
    // }

    // println!("Printing collection names in {}", db.name());
    // for collection_name in block_on(db.list_collection_names(None)).unwrap() {
    //     println!("{}", collection_name);
    // }

    //let col = db.collection("sessions");
    // client_options.app_name = Some("unreal-analytics-server".to_string());
    // let client = Client::with_options(client_options).unwrap();

    // println!("Printing database names!");
    // // List the names of the databases in that deployment.
    // for db_name in client.list_database_names(None, None).await.unwrap() {
    //     println!("{}", db_name);
    // }

    Database {
        client: client,
        database: db,
    }
}
