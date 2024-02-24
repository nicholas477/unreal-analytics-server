use crate::cloudflare;

use rocket::{get, http::Status, serde::json::Json};

#[get("/")]
pub fn index(info: cloudflare::CloudflareInfo) -> Result<Json<String>, Status> {
    print!("User: {:?}", info);
    Ok(Json(String::from("Hello from rust and mongoDB")))
}
