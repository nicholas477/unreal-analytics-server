pub mod issue;
pub mod token;

use mongodb::bson::{doc, Document};
use rocket::serde::json::{Json, Value};
use rocket::{http::Status, post};

use rocket::routes;

use self::token::refresh_access_token;

impl crate::state::TodoList {
    pub async fn get_github_id(&self) -> Option<i64> {
        if self.github_issue_id > -1 {
            return Some(self.github_issue_id);
        }

        let res = crate::state::get_server_state()
            .db
            .get_todo_list_github_id(&self.list_id)
            .await;

        match res {
            Ok(maybe_list_id) => {
                if maybe_list_id? >= 0 {
                    return maybe_list_id;
                }
            }
            Err(e) => {
                eprintln!("Github: Failed to read github id from database!");
                eprintln!("Github: Error: {}", e.to_string());
            }
        }

        return None;
    }

    pub fn set_github_id(&mut self, id: i64) -> Option<()> {
        self.github_issue_id = id;

        self.list
            .get_mut("GithubIssueID")?
            .as_object_mut()?
            .insert("__Value".into(), id.into());

        Some(())
    }
}

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
    method: Option<reqwest::Method>, // Defaults to GET
    token: String,
) -> Option<serde_json::Value> {
    let method = method.unwrap_or(reqwest::Method::GET);
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
    method: Option<reqwest::Method>, // Defaults to GET
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

    let method = method.unwrap_or(reqwest::Method::GET);
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

pub async fn create_issues() -> Option<()> {
    let lists = crate::state::get_server_state()
        .db
        .get_todo_lists(true)
        .await
        .ok()
        .unwrap();

    for list in lists {
        let todolist = crate::state::TodoList::from_bson(&list).unwrap();

        issue::update_or_create_issue(&todolist).await.unwrap();
    }

    Some(())
}

pub async fn initialize() -> Option<()> {
    if crate::state::get_server_state()
        .read_config()?
        .github
        .enabled
        == false
    {
        println!("Github: integration DISABLED");
        return Some(());
    }

    token::refresh_access_token().await?;
    println!("Github: Got Github access token");

    println!("Github: Creating/updating github issues...");
    create_issues().await.unwrap();

    Some(())
}

// Event handler for server events
async fn handle_event(event: crate::state::ServerEvent) -> Option<()> {
    use crate::state::ServerEvent;
    return match event {
        ServerEvent::TodoListUpdate { list } => {
            issue::update_or_create_issue(&list).await?;
            Some(())
        }

        // For deletion, query the list from the database and set the issue to closed
        ServerEvent::TodoListDelete { id } => {
            let mut list = crate::state::get_server_state()
                .db
                .get_todo_list(&id, true)
                .await
                .ok()??;

            list.deleted = true;

            issue::update_or_create_issue(&list).await?;
            Some(())
        }
    };
}

#[post("/todo-list/webhook", data = "<data>")]
pub fn hook(data: Json<Value>) -> Result<Json<String>, Status> {
    //println!("Received post request! {:#?}", data);
    Ok(Json(String::from("Hello from rust and mongoDB")))
}

pub async fn start() -> tokio::task::JoinHandle<()> {
    if crate::state::get_server_state()
        .read_config()
        .unwrap()
        .github
        .enabled
        == false
    {
        println!("Github: integration DISABLED");
        return tokio::spawn(async move {});
    }

    let server_handle = tokio::spawn(async move {
        let _rocket = rocket::build().mount("/", routes![hook]).launch().await;
    });

    let _event_handle = crate::state::add_server_event_handler(handle_event).await;

    server_handle
}
