pub mod auth;
pub mod cloudflare;
pub mod commands;
pub mod config;
pub mod database;
pub mod discord_bot;
pub mod routes;
pub mod utils;

use once_cell::sync::OnceCell;
use std::sync::Arc;
use std::sync::RwLock;

#[macro_use]
extern crate rocket;

use rocket::routes;

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
    database::fixup_database(&db).await.unwrap();

    let state = Arc::new(ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
    });

    SERVER_STATE.set(state.clone()).unwrap();
}

#[rocket::main]
async fn main() -> Result<(), rocket::Error> {
    initialize().await;

    discord_bot::initialize();

    println!("Running http server...");
    let _rocket = rocket::build()
        .mount(
            "/",
            routes![routes::index::index, routes::session_upload::upload_session],
        )
        .ignite()
        .await?
        .launch()
        .await?;

    Ok(())
}
