use rocket::serde::json::Value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GithubIssue {
    title: String,
    body: String,
    state: String, // opened or closed
}

// Converts a single task into a display string on a github issue
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

    let task_description = (|| {
        let description = task
            .get("TaskDescription")?
            .as_object()?
            .get("__Value")?
            .as_object()?
            .get("SourceString")?
            .as_str()?;

        Some(description)
    })();

    let mut task_string = String::new();

    task_string += "- [";
    task_string += if task_checked { "x" } else { " " };
    task_string += "] ";
    task_string += task_title;

    if let Some(description_str) = task_description {
        task_string += "\n\t";
        task_string += description_str.replace("\n", "\n\t").as_str();
    }

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
            state: if self.deleted {
                "closed".into()
            } else {
                "open".into()
            },
        }
    }
}

//'{"title":"Found a bug","body":"I'\''m having a problem with this.","assignees":["octocat"],"milestone":1,"labels":["bug"]}'

// Returns the todo list with the new github id
pub async fn create_issue(list: &crate::state::TodoList) -> Option<crate::state::TodoList> {
    // Don't create a list if this one already has an ID
    if let Some(list_id) = list.get_github_id().await {
        println!("Github: Issue {} already exists", list_id);
        return None;
    }

    let new_issue = list.to_github_issue();

    let new_issue_res = super::send_github_request(
        format!("/repos/{}/issues", super::get_github_repo()?.repo),
        Some(reqwest::Method::POST),
        Some(serde_json::to_string(&new_issue).ok()?),
    )
    .await?;

    let new_issue_id = new_issue_res.get("number")?.as_i64()?;

    let mut new_list = list.clone();
    new_list.set_github_id(new_issue_id);

    println!(
        "Github: Created new github issue: {} for list id: {} ({})",
        new_issue_id, new_list.list_name, new_list.list_id,
    );

    // Update with the new ID
    let event = crate::state::ServerEvent::TodoListUpdate {
        list: new_list.clone(),
    };

    if let Err(e) = crate::state::broadcast_server_event(event).await {
        eprintln!("Github: Failed to broadcast server event for TodoListUpdate message!");
        eprintln!("Github: Error: {}", e.to_string());
    } else {
        println!("Github: Broadcasted server event!");
    }

    Some(new_list)
}

pub async fn update_or_create_issue(list: &crate::state::TodoList) -> Option<i64> {
    // Don't create a list if this one already has an ID
    if let Some(list_github_id) = list.get_github_id().await {
        let issue = list.to_github_issue();

        let update_issue_res = super::send_github_request(
            format!(
                "/repos/{}/issues/{}",
                super::get_github_repo()?.repo,
                list_github_id
            ),
            Some(reqwest::Method::POST),
            Some(serde_json::to_string(&issue).ok()?),
        )
        .await?;

        println!(
            "Github: Updated issue: {} ({})",
            list.list_name, list_github_id
        );

        update_issue_res.get("number")?.as_i64()
    } else {
        let new_list = create_issue(list).await?;

        println!("update_or_create_issue: returning github id");
        return new_list.get_github_id().await;
    }
}
