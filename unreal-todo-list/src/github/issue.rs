use rocket::serde::json::{Json, Value};
use rocket::{http::Status, post};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubIssue {
    title: String,
    body: String,
}

fn task_to_string(task: &Value) -> String {
    let task_checked = (|| {
        let task_status = task
            .get("TaskStatus")?
            .as_object()?
            .get("__Value")?
            .as_str()?;
        return Some(task_status == "Done");
    })()
    .unwrap_or(false);

    let task_title = (|| {
        let title = task
            .get("TaskName")?
            .as_object()?
            .get("__Value")?
            .as_object()?
            .get("SourceString")?
            .as_str()?;
        return Some(title);
    })()
    .unwrap_or("".into());

    let mut task_string = String::new();

    task_string += "- [";
    task_string += if task_checked { "x" } else { " " };
    task_string += "] ";
    task_string += task_title;

    task_string
}

impl crate::state::TodoList {
    pub fn to_github_issue(&self) -> GithubIssue {
        let mut tasks_vec = Vec::<String>::new();

        if let Some(tasks) = self.get_task_array() {
            for task in tasks {
                tasks_vec.push(task_to_string(task));
            }
        }

        GithubIssue {
            title: self.list_name.clone(),
            body: tasks_vec.join("\n"),
        }
    }
}

//'{"title":"Found a bug","body":"I'\''m having a problem with this.","assignees":["octocat"],"milestone":1,"labels":["bug"]}'

// Returns the todo list with the new github id
pub async fn create_issue(list: &crate::state::TodoList) -> Option<crate::state::TodoList> {
    // Don't create a list if this one already has an ID
    if let Some(_list_id) = list.get_github_id() {
        println!("Issue already exists");
        return None;
    }

    let new_issue = list.to_github_issue();

    let new_issue_res = super::send_github_request(
        format!("/repos/{}/issues", super::get_github_repo()?.repo),
        Some(http::method::Method::POST),
        Some(serde_json::to_string(&new_issue).ok()?),
    )
    .await?;

    let new_issue_id = new_issue_res.get("number")?.as_i64()?;
    let mut new_list = list.clone();
    new_list.set_github_id(new_issue_id);

    // // Update with the new ID
    crate::state::get_server_state()
        .db
        .update_todo_list(&new_list)
        .await
        .ok()?;

    Some(new_list)
}

pub async fn update_or_create_issue(list: &crate::state::TodoList) -> Option<i64> {
    // Don't create a list if this one already has an ID
    if let Some(list_github_id) = list.get_github_id() {
        let issue = list.to_github_issue();

        let update_issue_res = super::send_github_request(
            format!(
                "/repos/{}/issues/{}",
                super::get_github_repo()?.repo,
                list_github_id
            ),
            Some(http::method::Method::POST),
            Some(serde_json::to_string(&issue).ok()?),
        )
        .await?;

        println!("Updated issue: {} ({})", list.list_name, list_github_id);

        update_issue_res.get("number")?.as_i64()
    } else {
        let new_list = create_issue(list).await?;
        return new_list.get_github_id();
    }
}
