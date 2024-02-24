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

use chrono::{NaiveDateTime, TimeDelta, Utc};
use rocket::Data;

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
    pub avg_play_time: chrono::TimeDelta,
}

impl std::fmt::Display for PlayerStats {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "PIE Sessions: {}\nGame Sessions: {}\nUnique Players: {}\nAverage Play Time: {}",
            self.pie_sessions,
            self.game_sessions,
            self.unique_players,
            crate::utils::session_duration_to_string(&self.avg_play_time)
        )
    }
}

pub async fn fixup_database(database: &Database) -> mongodb::error::Result<()> {
    let collection = database.database.collection::<Document>("sessions");
    let mut cursor = collection.find(None, None).await?;

    while cursor.advance().await? {
        let mut document = cursor.deserialize_current()?;
        match (
            convert_date_time(&mut document, "StartTime"),
            convert_date_time(&mut document, "EndTime"),
        ) {
            (Some(start_time), Some(end_time)) => {
                let id = document.get_object_id("_id").unwrap();
                let filter = doc! {"_id": id};
                let update = doc! {"$set": doc!{"BP_SessionAnalyicsCollector_C.StartTime": start_time, "BP_SessionAnalyicsCollector_C.EndTime": end_time}};

                collection.update_one(filter, update, None).await?;
            }
            _ => return Ok(()),
        }
    }

    Ok(())
}

// Converts a document's date time from a string to a datetime
fn convert_date_time(document: &mut Document, field_name: &str) -> Option<chrono::DateTime<Utc>> {
    if let Ok(session_obj) = document.get_document_mut("BP_SessionAnalyicsCollector_C") {
        if let Ok(time_str) = session_obj.get_str(field_name) {
            if let Some(time_date) =
                NaiveDateTime::parse_from_str(time_str, "%Y.%m.%d-%H.%M.%S").ok()
            {
                session_obj.insert(field_name, time_date.and_utc());
                return Some(time_date.and_utc());
            }
        }
    }

    None
}

fn get_time(document: &Document, field_name: &str) -> Option<chrono::DateTime<Utc>> {
    if let Ok(session_obj) = document.get_document("BP_SessionAnalyicsCollector_C") {
        if let Ok(date_time) = session_obj.get_datetime(field_name) {
            return Some(date_time.clone().into());
        }
    }

    None
}

fn get_time_delta(document: &Document) -> Option<chrono::TimeDelta> {
    if let (Some(start_time), Some(end_time)) = (
        get_time(document, "StartTime"),
        get_time(document, "EndTime"),
    ) {
        return Some(end_time - start_time);
    }

    None
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
        let mut document = mongodb::bson::to_document(&mongodb::bson::to_bson(session)?)?;

        convert_date_time(&mut document, "StartTime");
        convert_date_time(&mut document, "EndTime");

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

        let game_sessions = collection.find(pie_filter(false), None).await?;
        let play_times = game_sessions.map(|session_res| match session_res {
            Ok(session) => get_time_delta(&session).ok_or("".to_string()),
            Err(e) => Err(e.to_string()),
        });

        let vec = play_times.collect::<Vec<_>>().await;

        let mut num_play_times: i32 = 0;
        let mut avg_play_time = chrono::TimeDelta::new(0, 0).unwrap();
        for play_time_res in vec {
            if let Ok(play_time) = play_time_res {
                avg_play_time = avg_play_time + play_time;
                num_play_times = num_play_times + 1;
            }
        }

        avg_play_time = avg_play_time / num_play_times;

        Ok(PlayerStats {
            pie_sessions: pie_session_cnt,
            game_sessions: game_session_cnt,
            unique_players: ips.len() as u64,
            avg_play_time: avg_play_time,
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
