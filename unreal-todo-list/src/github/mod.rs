pub mod issue;
pub mod token;

use mongodb::bson::{doc, Document};
use rocket::serde::json::{Json, Value};
use rocket::{http::Status, post};
use serde::{Deserialize, Serialize};

use rocket::routes;

use self::token::refresh_access_token;

#[derive(Debug, Default)]
pub struct GithubState {
    pub access_token: token::AccessToken,
}

pub fn get_github_repo() -> Option<crate::config::GithubConfig> {
    let github_config = crate::state::get_server_state()
        .config
        .try_read()
        .ok()?
        .github
        .clone();

    Some(github_config)
}

/// # Arguments
///
/// * `method` - http method, defaults to GET
///
pub async fn send_github_request_with_token(
    endpoint: String,
    method: Option<http::method::Method>, // Defaults to GET
    token: String,
) -> Option<serde_json::Value> {
    let method = method.unwrap_or(http::method::Method::GET);
    let url = format!("https://api.github.com{}", endpoint);
    let user_agent = crate::state::get_server_state()
        .config
        .try_read()
        .ok()?
        .github
        .app_name
        .clone()
        .unwrap_or("Curl".into());

    let client = reqwest::Client::new();
    let builder = client
        .request(method, url)
        .header("User-Agent", user_agent)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", token))
        .header("X-GitHub-Api-Version", "2022-11-28");

    let res = builder.send().await;

    res.ok()?.json::<serde_json::Value>().await.ok()
}

pub async fn send_github_request(
    endpoint: String,
    method: Option<http::method::Method>, // Defaults to GET
    body: Option<String>,
) -> Option<serde_json::Value> {
    if token::should_refresh_access_token()? {
        refresh_access_token().await?;
    }

    let token = crate::state::get_server_state()
        .github_state
        .try_read()
        .ok()?
        .access_token
        .clone();

    let method = method.unwrap_or(http::method::Method::GET);
    let url = format!("https://api.github.com{}", endpoint);
    let user_agent = crate::state::get_server_state()
        .config
        .try_read()
        .ok()?
        .github
        .app_name
        .clone()
        .unwrap_or("Curl".into());

    let client = reqwest::Client::new();
    let builder = client
        .request(method, url)
        .header("User-Agent", user_agent)
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", format!("Bearer {}", token.token))
        .header("X-GitHub-Api-Version", "2022-11-28")
        .body(body.unwrap_or("".into()));

    let res = builder.send().await;

    res.ok()?.json::<serde_json::Value>().await.ok()
}

pub fn get_db_list_github_id(list: &Document) -> Option<i64> {
    let list_id = list
        .get_document("SerializedList")
        .ok()?
        .get_document("GithubIssueID")
        .ok()?
        .get_i64("__Value")
        .ok()?;

    if list_id >= 0 {
        return Some(list_id);
    } else {
        return None;
    }
}

pub fn get_list_github_id(list: &crate::state::TodoList) -> Option<i64> {
    let list_id = list
        .list
        .get("SerializedList")?
        .as_object()?
        .get("GithubIssueID")?
        .as_object()?
        .get("__Value")?
        .as_i64()?;

    if list_id >= 0 {
        return Some(list_id);
    } else {
        return None;
    }
}

pub async fn create_issues() -> Option<()> {
    let lists = crate::state::get_server_state()
        .db
        .get_todo_lists()
        .await
        .ok()?;

    for mut list in lists {
        let todolist = crate::state::TodoList::from_bson(&list)?;
        if let Some(new_issue_id) = issue::create_issue(&todolist).await {
            // Insert the new value into the old list
            list.get_document_mut("SerializedList")
                .ok()?
                .get_document_mut("GithubIssueID")
                .ok()?
                .insert("__Value", new_issue_id)?;

            // Update with the new ID
            crate::state::get_server_state()
                .db
                .update_todo_list(&list)
                .await
                .ok()?;
        }
    }

    Some(())
}

pub async fn initialize() -> Option<()> {
    token::refresh_access_token().await?;
    println!("Got Github access token");

    println!("Creating github issues...");
    create_issues().await;

    Some(())
}

// Event handler for server events
async fn handle_event(event: crate::state::ServerEvent) -> Option<()> {
    use crate::state::ServerEvent;
    return match event {
        ServerEvent::TodoListUpdate { list } => {
            //if ()
            None
        }
        ServerEvent::TodoListDelete { id } => None,
    };
}

#[post("/todo-list/webhook", data = "<data>")]
pub fn hook(data: Json<Value>) -> Result<Json<String>, Status> {
    //println!("Received post request! {:#?}", data);
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

pub async fn start() -> tokio::task::JoinHandle<()> {
    let server_handle = tokio::spawn(async move {
        let _rocket = rocket::build().mount("/", routes![hook]).launch().await;
    });

    let _event_handle = crate::state::add_server_event_handler(handle_event).await;

    server_handle
}
