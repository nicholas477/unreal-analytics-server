use std::sync::Arc;

use crate::{cloudflare, get_server_state};
use chrono::{NaiveDateTime, TimeDelta};
use serenity::builder::ExecuteWebhook;
use serenity::{builder::CreateAttachment, http::Http, model::webhook::Webhook};

use rocket::{
    http::Status,
    post,
    serde::json::serde_json,
    serde::json::{Json, Value},
    State,
};

fn parse_end_time(session: &Json<Value>) -> Option<NaiveDateTime> {
    let start_time_str = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("EndTime")?
        .as_str()?;

    NaiveDateTime::parse_from_str(start_time_str, "%Y.%m.%d-%H.%M.%S").ok()
}

fn parse_start_time(session: &Json<Value>) -> Option<NaiveDateTime> {
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

fn is_editor_session(session: &Json<Value>) -> Option<bool> {
    session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("IsPlayInEditorSession")?
        .as_bool()
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

// Returns the country code and name
fn get_country_data(session: &Json<Value>) -> Option<(&str, &str)> {
    let country_code = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("CountryCode")?
        .as_str()?;

    let country = session
        .as_object()?
        .get("BP_SessionAnalyicsCollector_C")?
        .get("CountryName")?
        .as_str()?;

    Some((country_code, country))
}

// Inserts the IP into the BP_SessionAnalyicsCollector_C object
fn insert_cloudflare_info_into_session_collector(
    cloudflare_info: &cloudflare::CloudflareInfo,
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

    session_collector_obj.insert(
        "ip".to_string(),
        Value::String(cloudflare_info.ip.to_string()),
    );

    session_collector_obj.insert(
        "CountryCode".to_string(),
        Value::String(cloudflare_info.country.clone()),
    );

    session_collector_obj.insert(
        "CountryName".to_string(),
        Value::String(cloudflare_info.get_country_name().to_string()),
    );

    Ok(res)
}

// Looks at the session data and picks out NetID, StartTime, EndTime, and feedback comments for the discord message
fn build_content_string(session: &Json<Value>) -> Result<String, Box<dyn std::error::Error>> {
    // Look into the json and pick out some interesting data
    let net_id = get_net_id(&session).ok_or("Unable to find Net ID in session data")?;
    let is_steam_session =
        is_steam_session(&session).ok_or("Unable to find SteamAnalyticsData in session data")?;

    let comments = get_feedback_comments(&session).unwrap_or(Vec::new());

    let start_time =
        parse_start_time(&session).ok_or("Unable to find StartTime in session data")?;
    let end_time = parse_end_time(&session).ok_or("Unable to find EndTime in session data")?;

    let session_duration = crate::utils::session_duration_to_string(&(end_time - start_time));

    let (country_code, country_name) =
        get_country_data(&session).ok_or("Unable to find country data")?;
    let country_string = format!("({}/{})", country_code, country_name);

    let game_name_type_string = if is_editor_session(&session).unwrap_or(false) {
        "PIE game"
    } else {
        "game"
    };

    // If it's a steam session, put their steam page in the message
    let mut content_str = if is_steam_session {
        format!(
            "https://steamcommunity.com/profiles/{}\n{}\nPlayed a {} for {}",
            net_id, country_string, game_name_type_string, session_duration
        )
    } else {
        format!(
            "{}\n{}\nPlayed a {} for {}",
            net_id, country_string, game_name_type_string, session_duration
        )
    };

    // If they added comments, put that in the message
    if comments.len() > 0 {
        content_str += "\n\nFeedback comments:";
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

use crate::auth::ApiKey;

pub fn try_spawn_discord_message_task(session: Json<Value>) {
    let state = get_server_state();
    let config = match state.read_config() {
        Some(config) => config,
        None => {
            eprintln!("Failed to acquire server state lock!");
            return;
        }
    };

    if !config.discord_config.send_messages {
        return;
    }

    // Don't send editor session messages if it's an editor sesh
    if is_editor_session(&session).unwrap_or(false) && !config.discord_config.notify_editor_sessions
    {
        return;
    }

    let url = state.secrets.keys.discord_webhook.to_string();
    tokio::task::spawn(async move {
        send_discord_session_info(&url, session).await.unwrap_or(());
    });
}

#[post("/", data = "<session>")]
pub async fn upload_session(
    _key: ApiKey,
    cloudflare_info: cloudflare::CloudflareInfo,
    session: Json<Value>,
) -> Result<String, Status> {
    // Modify the session data, add the IP
    let modified_session =
        insert_cloudflare_info_into_session_collector(&cloudflare_info, &session)
            .map_err(|_| Status { code: 400 })?;

    let state = crate::get_server_state();

    // Throw it into the database
    let db_res = state.db.add_session(&modified_session).await;

    match db_res {
        Ok(_) => {
            // Spawn the discord task async so that we don't have to wait before returning a http response
            try_spawn_discord_message_task(modified_session);

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
