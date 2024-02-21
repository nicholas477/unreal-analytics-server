pub mod database;
pub mod routes;
pub mod secrets;

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
    secrets: secrets::Secrets,
}

#[get("/")]
fn index() -> Result<Json<String>, Status> {
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

#[launch]
fn rocket() -> _ {
    let keys = secrets::read_secrets();

    let db = database::connect_to_db();
    db.print_info();

    let state = ServerState {
        db: db,
        secrets: keys,
    };

    rocket::build()
        .manage(state)
        .mount("/", routes![index, routes::session_upload::upload_session])
}
