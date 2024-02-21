use crate::database;

use rocket::{
    get,
    http::Status,
    serde::json::{Json, Value},
    State,
};

#[post("/", data = "<session>")]
pub fn upload_session(
    state: &State<crate::ServerState>,
    session: Json<Value>,
) -> Result<String, Status> {
    println!("Session data: {:#?}", session);

    state.db.add_session(&session.into_inner());

    Ok("".to_string())
}
