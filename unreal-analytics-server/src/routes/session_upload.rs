use rocket::tokio::task::spawn_blocking;
use serenity::builder::ExecuteWebhook;
use serenity::{builder::CreateAttachment, http::Http, model::webhook::Webhook};

use rocket::{
    get,
    http::Status,
    request::Request,
    serde::json::serde_json,
    serde::json::{Json, Value},
    State,
};

async fn send_discord_session_info(
    addr: std::net::SocketAddr,
    url: String,
    mut session: Json<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http = Http::new("");

    let session_obj = session.as_object_mut().unwrap();
    session_obj
        .get_mut("BP_SessionAnalyicsCollector_C")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("ip".to_string(), Value::String(addr.ip().to_string()));
    let session_str = serde_json::ser::to_string_pretty(session_obj)?;

    let webhook = Webhook::from_url(&http, &url).await?;
    let file = CreateAttachment::bytes(session_str, "AnalyticsSession.json");
    let builder = ExecuteWebhook::new()
        .content("New game incoming!")
        .add_file(file);

    println!("Sending discord message");
    webhook.execute(&http, true, builder).await?;
    println!("Discord message sent!");

    Ok(())
}

use tokio::task;

#[post("/", data = "<session>")]
pub async fn upload_session(
    addr: std::net::SocketAddr,
    state: &State<crate::ServerState>,
    session: Json<Value>,
) -> Result<String, Status> {
    println!("Session data: {:#?}", session);

    state.db.add_session(&session.to_owned());

    // Spawn the discord task async so that we don't have to wait before returning a http response
    let url = state.secrets.keys.discord_webhook.to_string();
    task::spawn(async move {
        send_discord_session_info(addr, url, session).await.unwrap();
    });

    println!("Returning result");
    Ok("".to_string())
}
