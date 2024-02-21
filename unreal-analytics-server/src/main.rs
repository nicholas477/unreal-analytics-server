pub mod config;
pub mod database;
pub mod routes;

#[macro_use]
extern crate rocket;
use rocket::{
    get,
    http::Status,
    serde::json::{Json, Value},
    State,
};

pub struct ServerState {
    db: database::Database,
    config: config::Config,
    secrets: config::Secrets,
}

#[get("/")]
fn index() -> Result<Json<String>, Status> {
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

#[launch]
fn rocket() -> _ {
    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);
    db.print_info();

    let state = ServerState {
        db: db,
        config: config,
        secrets: keys,
    };

    rocket::build()
        .manage(state)
        .mount("/", routes![index, routes::session_upload::upload_session])
}
