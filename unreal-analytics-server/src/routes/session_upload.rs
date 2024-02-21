use crate::database;

use futures::executor::block_on;
use serenity::builder::ExecuteWebhook;
use serenity::{http::Http, model::webhook::Webhook};

use rocket::{
    get,
    http::Status,
    serde::json::{Json, Value},
    State,
};
async fn send_discord_session_info(
    state: &State<crate::ServerState>,
    session: &Json<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http = Http::new("");
    let url = &state.secrets.keys.discord_webhook;

    let webhook = Webhook::from_url(&http, url).await?;
    let builder = ExecuteWebhook::new()
        .content("hello there")
        .username("Webhook test");

    webhook.execute(&http, true, builder).await?;

    Ok(())
}

#[post("/", data = "<session>")]
pub fn upload_session(
    state: &State<crate::ServerState>,
    session: Json<Value>,
) -> Result<String, Status> {
    println!("Session data: {:#?}", session);

    state.db.add_session(&session.to_owned());

    block_on(send_discord_session_info(&state, &session));

    Ok("".to_string())
}
