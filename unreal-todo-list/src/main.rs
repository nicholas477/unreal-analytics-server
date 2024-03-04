pub mod auth;
pub mod config;
pub mod database;

use futures::SinkExt;
use once_cell::sync::OnceCell;
use rocket::serde::json::serde_json;
use serde_json::Value;
use std::sync::Arc;
use std::sync::RwLock;
use ws::Message;

#[macro_use]
extern crate rocket;

use rocket_ws as ws;

#[derive(Debug)]
pub struct ServerState {
    pub db: database::Database,
    pub default_config: config::Config,
    pub config: RwLock<config::Config>, // Config can be changed later
    pub secrets: config::Secrets,
}

impl ServerState {
    // Acquires a read lock on config and returns a copy of it
    pub fn read_config(&self) -> Option<config::Config> {
        let lock = self.config.read().ok()?;

        Some(lock.clone())
    }
}

pub static SERVER_STATE: OnceCell<Arc<ServerState>> = OnceCell::new();
pub fn get_server_state() -> Arc<ServerState> {
    SERVER_STATE.get().unwrap().clone()
}

async fn initialize() {
    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);

    let state = Arc::new(ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
    });

    SERVER_STATE.set(state.clone()).unwrap();
}

async fn create_update_client_msg() -> Option<Vec<Message>> {
    let todo_lists = get_server_state().db.get_todo_lists().await.ok()?;
    let mut messages: Vec<Message> = Vec::new();
    messages.reserve(todo_lists.len());

    for bson_list in todo_lists {
        let mut json_list = serde_json::to_value(&bson_list).unwrap();
        json_list.as_object_mut()?.insert(
            "MessageType".to_string(),
            Value::String("TodoListUpdate".to_string()),
        );

        messages.push(Message::Text(
            serde_json::to_string_pretty(&json_list).ok()?,
        ));
    }

    Some(messages)
}

async fn update_list_database(msg_json: &Value) -> mongodb::error::Result<()> {
    if let Some(document) = database::convert_list_to_bson(msg_json) {
        get_server_state().db.update_todo_list(&document).await?;
    } else {
        eprintln!("Failed to convert message to json!!!!");
    }
    Ok(())
}

async fn parse_message(msg: &String) -> Option<Message> {
    let msg_json: Value = serde_json::from_str(msg.as_str()).ok()?;
    let res = update_list_database(&msg_json).await;

    if let Err(e) = res {
        eprintln!("Failed to update list in database! Error: {:#?}", e);
    }

    Some(Message::Text("".into()))
}

#[get("/todo-list")]
fn echo_stream(ws: ws::WebSocket, _key: auth::ApiKey) -> ws::Stream!['static] {
    let ws = ws.config(ws::Config {
        ..Default::default()
    });

    ws::Stream! { ws =>
        println!("New client!");
        if let Some(update_messages) = create_update_client_msg().await {
            println!("Sending client {} todo lists...", update_messages.len());
            for msg in update_messages {
                yield msg;
            }
        }

        for await message in ws {
            if let Ok(ws::Message::Text(ref msg)) = message {
                if let Some(res_msg) = parse_message(msg).await {
                    yield res_msg;
                }
                else
                {
                    println!("Shits fucked");
                }
            }
        }
    }
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    initialize().await;

    let _rocket = rocket::build()
        .mount("/", routes![echo_stream])
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
