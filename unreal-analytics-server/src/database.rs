use futures::executor::block_on;
use futures::StreamExt;
use mongodb::results::InsertOneResult;
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client, Collection,
};
use rocket::http::ext::IntoCollection;
use rocket::serde::json::Value;

use crate::config;

#[derive(Debug)]
pub struct Database {
    pub client: mongodb::Client,
    pub database: mongodb::Database,
}

#[derive(Debug)]
pub struct PlayerStats {
    pub pie_sessions: u64,
    pub game_sessions: u64,
    pub unique_players: u64, // Number of unique IPs
}

impl std::fmt::Display for PlayerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "PIE Sessions: {}\nGame Sessions: {}\nUnique Players: {}",
            self.pie_sessions, self.game_sessions, self.unique_players
        )
    }
}

pub fn get_play_time(document: &Document) {
    //let start_time = document.get_str(key)
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
        let document = mongodb::bson::to_document(&mongodb::bson::to_bson(session)?)?;

        collection.insert_one(document, None).await
    }

    pub async fn get_players_stats(&self) -> mongodb::error::Result<PlayerStats> {
        let collection = self.database.collection::<Document>("sessions");

        let pie_filter =
            |is_pie: bool| doc! {"BP_SessionAnalyicsCollector_C.IsPlayInEditorSession": is_pie};
        let pie_session_cnt = collection.count_documents(pie_filter(true), None).await?;
        let game_session_cnt = collection.count_documents(pie_filter(false), None).await?;

        let ips = collection
            .distinct("BP_SessionAnalyicsCollector_C.ip", None, None)
            .await?;

        // let game_sessions = collection
        //     .find(pie_filter(false), None)
        //     .await?
        //     .map(|entry| match entry {});

        Ok(PlayerStats {
            pie_sessions: pie_session_cnt,
            game_sessions: game_session_cnt,
            unique_players: ips.len() as u64,
        })
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
