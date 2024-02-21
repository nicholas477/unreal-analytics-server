use chrono::format::ParseError;
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta};
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

fn get_end_time(session: &Json<Value>) -> Option<NaiveDateTime> {
    let start_time_str = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("EndTime")?
        .as_str()?;

    NaiveDateTime::parse_from_str(&start_time_str, "%Y.%m.%d-%H.%M.%S").ok()
}

fn get_start_time(session: &Json<Value>) -> Option<NaiveDateTime> {
    let start_time_str = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("StartTime")?
        .as_str()?;

    NaiveDateTime::parse_from_str(&start_time_str, "%Y.%m.%d-%H.%M.%S").ok()
}

fn is_steam_session(session: &Json<Value>) -> Option<bool> {
    Some(
        session
            .as_object()?
            .get("BP_SessionAnalyicsCollector_C")?
            .get("PlayerControllerData")?
            .as_array()?
            .get(0)?
            .get("SteamAnalyticsData")
            .is_some(),
    )
}

// Returns the NetID from inside PlayerControllerData
fn get_net_id(session: &Json<Value>) -> Option<&str> {
    session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("PlayerControllerData")?
        .as_array()?
        .get(0)?
        .get("NetID")?
        .as_str()
}

// Inserts the IP into the BP_SessionAnalyicsCollector_C object
fn insert_ip_into_session_collector(
    addr: &std::net::SocketAddr,
    session: &Json<Value>,
) -> Result<Json<Value>, Box<dyn std::error::Error>> {
    let mut res = session.clone();

    let session_obj = res
        .as_object_mut()
        .ok_or("Failed to find json object in request")?;

    let session_collector_obj = session_obj
        .get_mut("BP_SessionAnalyicsCollector_C")
        .ok_or("Failed to find BP_SessionAnalyicsCollector_C in json object")?
        .as_object_mut()
        .ok_or("Failed to convert BP_SessionAnalyicsCollector_C to json object")?;

    session_collector_obj.insert("ip".to_string(), Value::String(addr.ip().to_string()));

    Ok(res)
}

fn session_duration_to_string(session_duration: &TimeDelta) -> String {
    format!(
        "{:0>2}:{:0>2}:{:0>2}",
        session_duration.num_hours() % 24,
        session_duration.num_minutes() % 60,
        session_duration.num_seconds() % 60
    )
}

async fn send_discord_session_info(
    url: &String,
    mut session: Json<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http = Http::new("");

    // Look into the json and pick out some interesting data
    let net_id = get_net_id(&session).ok_or("Unable to find Net ID in session data")?;
    let is_steam_session =
        is_steam_session(&session).ok_or("Unable to find SteamAnalyticsData in session data")?;

    let start_time = get_start_time(&session).ok_or("Unable to find StartTime in session data")?;
    let end_time = get_end_time(&session).ok_or("Unable to find EndTime in session data")?;
    let session_str = serde_json::ser::to_string_pretty(session.as_object().unwrap())?;

    let session_duration = session_duration_to_string(&(end_time - start_time));

    let mut content_str = String::new();

    if is_steam_session {
        content_str = format!(
            "http://steamcommunity.com/profiles/{} played a game for {}",
            net_id, session_duration
        );
    } else {
        content_str = format!("{} played a game for {}", net_id, session_duration);
    }

    let webhook = Webhook::from_url(&http, &url).await?;
    let file = CreateAttachment::bytes(session_str, "AnalyticsSession.json");
    let builder = ExecuteWebhook::new().content(content_str).add_file(file);

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
    // Modify the session data, add the IP
    let modified_session =
        insert_ip_into_session_collector(&addr, &session).map_err(|_| Status { code: 400 })?;

    // Throw it into the database
    state.db.add_session(&modified_session);

    // Spawn the discord task async so that we don't have to wait before returning a http response
    let url = state.secrets.keys.discord_webhook.to_string();
    task::spawn(async move {
        send_discord_session_info(&url, modified_session)
            .await
            .unwrap_or(());
    });

    println!("Returning result");
    Ok("".to_string())
}
