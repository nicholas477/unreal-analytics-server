pub mod auth;
pub mod cloudflare;
pub mod config;
pub mod database;
pub mod routes;

use std::sync::RwLock;

#[macro_use]
extern crate rocket;
use rocket::{get, http::Status, serde::json::Json};

#[derive(Debug)]
pub struct ServerState {
    pub db: database::Database,
    pub default_config: config::Config,
    pub config: RwLock<config::Config>, // Config can be changed later
    pub secrets: config::Secrets,
}

#[get("/")]
fn index(info: cloudflare::CloudflareInfo) -> Result<Json<String>, Status> {
    print!("User: {:?}", info);
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

#[launch]
fn rocket() -> _ {
    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);
    db.print_info();

    let state = ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
    };

    rocket::build()
        .manage(state)
        .mount("/", routes![index, routes::session_upload::upload_session])
}
