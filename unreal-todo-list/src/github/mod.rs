pub mod token;

use chrono;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};
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
    let github_config = crate::get_server_state()
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
    let user_agent = crate::get_server_state()
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

    let token = crate::get_server_state()
        .github_state
        .try_read()
        .ok()?
        .access_token
        .clone();

    let method = method.unwrap_or(http::method::Method::GET);
    let url = format!("https://api.github.com{}", endpoint);
    let user_agent = crate::get_server_state()
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

pub fn get_list_github_id(list: &Document) -> Option<i64> {
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

#[derive(Debug, Serialize, Deserialize)]
struct GithubIssue {
    title: String,
    body: String,
}

//'{"title":"Found a bug","body":"I'\''m having a problem with this.","assignees":["octocat"],"milestone":1,"labels":["bug"]}'
pub async fn create_issue(list: &Document) -> Option<i64> {
    // Don't create a list if this one already has an ID
    if let Some(_list_id) = get_list_github_id(list) {
        println!("Issue already exists");
        return None;
    }

    let list_name = list.get_str("ListName").ok()?;

    let new_issue = GithubIssue {
        title: list_name.to_string(),
        body: "".into(),
    };

    let new_issue_res = send_github_request(
        format!("/repos/{}/issues", get_github_repo()?.repo),
        Some(http::method::Method::POST),
        Some(serde_json::to_string(&new_issue).ok()?),
    )
    .await?;

    println!("new issue res: {:#?}", new_issue_res);

    Some(new_issue_res.get("number")?.as_i64()?)
}

pub async fn create_issues() -> Option<()> {
    let mut lists = crate::get_server_state().db.get_todo_lists().await.ok()?;

    for mut list in lists {
        if let Some(new_issue_id) = create_issue(&list).await {
            // Insert the new value into the old list
            list.get_document_mut("SerializedList")
                .ok()?
                .get_document_mut("GithubIssueID")
                .ok()?
                .insert("__Value", new_issue_id)?;

            // Update with the new ID
            crate::get_server_state()
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

#[post("/todo-list/webhook", data = "<data>")]
pub fn hook(data: Json<Value>) -> Result<Json<String>, Status> {
    //println!("Received post request! {:#?}", data);
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

pub async fn start() -> tokio::task::JoinHandle<()> {
    let handle = tokio::spawn(async move {
        let _rocket = rocket::build().mount("/", routes![hook]).launch().await;
    });

    handle
}
