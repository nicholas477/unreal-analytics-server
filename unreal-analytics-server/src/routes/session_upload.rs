use crate::utils;
use chrono::{NaiveDateTime, TimeDelta};
use serenity::builder::ExecuteWebhook;
use serenity::{builder::CreateAttachment, http::Http, model::webhook::Webhook};

use rocket::{
    http::Status,
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

    NaiveDateTime::parse_from_str(start_time_str, "%Y.%m.%d-%H.%M.%S").ok()
}

fn get_start_time(session: &Json<Value>) -> Option<NaiveDateTime> {
    let start_time_str = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("StartTime")?
        .as_str()?;

    NaiveDateTime::parse_from_str(start_time_str, "%Y.%m.%d-%H.%M.%S").ok()
}

fn get_feedback_comments(session: &Json<Value>) -> Option<Vec<&str>> {
    let feedback_array: &Vec<Value> = session
        .as_object()?
        .get("BP_CactusGameFeedbackCollector_C")?
        .get("FeedbackComments")?
        .as_array()?;

    Some(
        feedback_array
            .iter()
            .map(|val| val.as_str())
            .flatten()
            .collect::<Vec<&str>>(),
    )
}

fn is_steam_session(session: &Json<Value>) -> Option<bool> {
    Some(
        session
            .as_object()?
            .get("BP_SessionAnalyicsCollector_C")?
            .get("PlayerControllerData")?
            .as_array()?
            .first()?
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
        .first()?
        .get("NetID")?
        .as_str()
}

// Inserts the IP into the BP_SessionAnalyicsCollector_C object
fn insert_ip_into_session_collector(
    addr: &utils::CloudflareIP,
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

    session_collector_obj.insert("ip".to_string(), Value::String(addr.0.clone()));

    Ok(res)
}

fn session_duration_to_string(session_duration: &TimeDelta) -> String {
    format!(
        "{}:{:0>2}:{:0>2}",
        session_duration.num_hours(),
        session_duration.num_minutes() % 60,
        session_duration.num_seconds() % 60
    )
}

// Looks at the session data and picks out NetID, StartTime, EndTime, and feedback comments for the discord message
fn build_content_string(session: &Json<Value>) -> Result<String, Box<dyn std::error::Error>> {
    // Look into the json and pick out some interesting data
    let net_id = get_net_id(&session).ok_or("Unable to find Net ID in session data")?;
    let is_steam_session =
        is_steam_session(&session).ok_or("Unable to find SteamAnalyticsData in session data")?;

    let comments = get_feedback_comments(&session).unwrap_or(Vec::new());

    let start_time = get_start_time(&session).ok_or("Unable to find StartTime in session data")?;
    let end_time = get_end_time(&session).ok_or("Unable to find EndTime in session data")?;

    let session_duration = session_duration_to_string(&(end_time - start_time));

    // If it's a steam session, put their steam page in the message
    let mut content_str = if is_steam_session {
        format!(
            "https://steamcommunity.com/profiles/{} played a game for {}",
            net_id, session_duration
        )
    } else {
        format!("{} played a game for {}", net_id, session_duration)
    };

    // If they added comments, put that in the message
    if comments.len() > 0 {
        content_str += "\nFeedback comments:";
        for comment in comments {
            content_str += &format!("\n`{}`", comment);
        }
    }

    Ok(content_str)
}

async fn send_discord_session_info(
    url: &String,
    session: Json<Value>,
) -> Result<(), Box<dyn std::error::Error>> {
    let http = Http::new("");

    let content_str = build_content_string(&session)?;
    let session_str = serde_json::ser::to_string_pretty(session.as_object().unwrap())?;

    let webhook = Webhook::from_url(&http, url).await?;
    let file = CreateAttachment::bytes(session_str, "AnalyticsSession.json");
    let builder = ExecuteWebhook::new().content(content_str).add_file(file);

    println!("Sending discord message");
    webhook.execute(&http, true, builder).await?;
    println!("Discord message sent!");

    Ok(())
}

use tokio::task;

use crate::auth::ApiKey;

#[post("/", data = "<session>")]
pub async fn upload_session(
    _key: ApiKey,
    addr: utils::CloudflareIP,
    state: &State<crate::ServerState>,
    session: Json<Value>,
) -> Result<String, Status> {
    // Modify the session data, add the IP
    let modified_session =
        insert_ip_into_session_collector(&addr, &session).map_err(|_| Status { code: 400 })?;

    // Throw it into the database
    let db_res = state.db.add_session(&modified_session).await;

    match db_res {
        Ok(_) => {
            // Spawn the discord task async so that we don't have to wait before returning a http response
            let url = state.secrets.keys.discord_webhook.to_string();
            task::spawn(async move {
                send_discord_session_info(&url, modified_session)
                    .await
                    .unwrap_or(());
            });

            Ok("".to_string())
        }
        Err(e) => {
            eprintln!(
                "Failed to insert session into database! Error: {}",
                e.to_string()
            );

            Err(Status { code: 500 })
        }
    }
}
