use rocket::serde::json::{Json, Value};
use rocket::{http::Status, post};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GithubIssue {
    title: String,
    body: String,
}

//'{"title":"Found a bug","body":"I'\''m having a problem with this.","assignees":["octocat"],"milestone":1,"labels":["bug"]}'
pub async fn create_issue(list: &crate::state::TodoList) -> Option<i64> {
    // Don't create a list if this one already has an ID
    if let Some(_list_id) = super::get_list_github_id(list) {
        println!("Issue already exists");
        return None;
    }

    let new_issue = GithubIssue {
        title: list.list_name.clone(),
        body: "".into(),
    };

    let new_issue_res = super::send_github_request(
        format!("/repos/{}/issues", super::get_github_repo()?.repo),
        Some(http::method::Method::POST),
        Some(serde_json::to_string(&new_issue).ok()?),
    )
    .await?;

    println!("new issue res: {:#?}", new_issue_res);

    // Insert the new value into the old list
    // list.get_document_mut("SerializedList")
    //     .ok()?
    //     .get_document_mut("GithubIssueID")
    //     .ok()?
    //     .insert("__Value", new_issue_id)?;

    // // Update with the new ID
    // crate::state::get_server_state()
    //     .db
    //     .update_todo_list(&list)
    //     .await
    //     .ok()?;

    Some(new_issue_res.get("number")?.as_i64()?)
}

pub async fn update_issue(list: &crate::state::TodoList) -> Option<i64> {
    // Don't create a list if this one already has an ID
    if let Some(_list_id) = super::get_list_github_id(list) {
        println!("Issue already exists");
        return None;
    }

    let new_issue = GithubIssue {
        title: list.list_name.clone(),
        body: "".into(),
    };

    let new_issue_res = super::send_github_request(
        format!("/repos/{}/issues", super::get_github_repo()?.repo),
        Some(http::method::Method::POST),
        Some(serde_json::to_string(&new_issue).ok()?),
    )
    .await?;

    println!("new issue res: {:#?}", new_issue_res);

    Some(new_issue_res.get("number")?.as_i64()?)
}
