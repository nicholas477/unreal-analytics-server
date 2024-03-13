use futures::executor::block_on;
use futures::StreamExt;
use mongodb::options::FindOneAndUpdateOptions;

use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};

use crate::state::TodoList;
use crate::{config, state};

impl crate::state::TodoList {
    pub fn to_bson(&self) -> Option<Document> {
        let document =
            mongodb::bson::to_document(&mongodb::bson::to_bson(&self.list).ok()?).ok()?;

        Some(
            doc!["ListID": self.list_id, "ListName": self.list_name.clone(), "GithubIssueID": self.github_issue_id, "SerializedList": document, "Deleted": self.deleted],
        )
    }

    pub fn from_bson(list: &Document) -> Option<Self> {
        let list_name = list.get_str("ListName").ok()?;

        let mut list_id = list.get_i64("ListID").ok();
        if list_id.is_none() {
            list_id = Some(list.get_i32("ListID").ok()?.into());
        }

        let mut github_issue_id = list.get_i64("GithubIssueID").ok();
        if github_issue_id.is_none() {
            github_issue_id = list.get_i32("GithubIssueID").ok().map(|val| val.into());
        }
        if github_issue_id.is_none() {
            github_issue_id = Some(TodoList::default().github_issue_id);
        }

        let mut deleted = list.get_bool("Deleted").ok();
        if deleted.is_none() {
            deleted = Some(TodoList::default().deleted);
        }

        let list = list.get_document("SerializedList").ok()?;

        Some(crate::state::TodoList {
            list_id: list_id?,
            list_name: list_name.into(),
            list: serde_json::to_value(list).ok()?,
            github_issue_id: github_issue_id?,
            deleted: deleted?,
        })
    }
}

#[derive(Debug)]
pub struct Database {
    pub client: mongodb::Client,
    pub database: mongodb::Database,
}

impl Database {
    pub fn print_info(&self) {
        println!("Database: Printing database names...");
        let db_names = block_on(self.client.list_database_names(None, None));
        for db_name in db_names.unwrap() {
            println!("{}", db_name);
        }

        println!(
            "Database: Printing collection names in {}",
            self.database.name()
        );
        for collection_name in block_on(self.database.list_collection_names(None)).unwrap() {
            println!("{}", collection_name);
        }
    }

    pub async fn update_todo_list(
        &self,
        list: &crate::state::TodoList,
    ) -> mongodb::error::Result<()> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = doc! {
            "ListID": list.list_id
        };

        let update = doc! {
            "$set": list.to_bson()
        };

        let options = FindOneAndUpdateOptions::builder()
            .upsert(Some(true))
            .build();

        collection
            .find_one_and_update(filter, update, Some(options))
            .await?;

        Ok(())
    }

    pub async fn delete_todo_list(&self, list_id: &i64) -> mongodb::error::Result<()> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = doc! {
            "ListID": list_id
        };

        let update = doc! {
            "$set": doc!{"Deleted": true}
        };

        collection.find_one_and_update(filter, update, None).await?;

        Ok(())
    }

    pub async fn get_todo_lists(
        &self,
        include_deleted_lists: bool,
    ) -> mongodb::error::Result<Vec<Document>> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = if include_deleted_lists {
            None
        } else {
            Some(doc! {
                "Deleted": doc!{ "$exists": false }
            })
        };

        let todo_lists = collection.find(filter, None).await?;

        let vec = todo_lists.collect::<Vec<_>>().await;
        vec.into_iter().collect()
    }

    pub async fn get_todo_list(
        &self,
        list_id: &i64,
        include_deleted_lists: bool,
    ) -> mongodb::error::Result<Option<crate::state::TodoList>> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = if include_deleted_lists {
            Some(doc! {"ListID": list_id,})
        } else {
            Some(doc! {
                "ListID": list_id,
                "Deleted": doc!{ "$in": [null, false] }
            })
        };

        let list = collection.find_one(filter, None).await?;

        if let Some(list) = list {
            return Ok(state::TodoList::from_bson(&list));
        }

        Ok(None)
    }

    pub async fn get_todo_list_github_id(
        &self,
        list_id: &i64,
    ) -> mongodb::error::Result<Option<i64>> {
        let collection = self.database.collection::<Document>("todolists");

        let filter = doc! {
            "ListID": list_id
        };

        let list = collection.find_one(filter, None).await?;

        if let Some(list) = list {
            if let Some(list) = state::TodoList::from_bson(&list) {
                return Ok(Some(list.github_issue_id));
            }
        }

        Ok(None)
    }
}

pub fn connect_to_db(config: &config::Config) -> Database {
    let client_options = ClientOptions::parse(&config.mongodb_connection_string).unwrap();
    let client = Client::with_options(client_options).unwrap();
    let db = client.database("local");

    Database {
        client,
        database: db,
    }
}

// Event handler for server events
async fn handle_event(event: crate::state::ServerEvent) -> Option<()> {
    use crate::state::ServerEvent;
    return match event {
        ServerEvent::TodoListUpdate { list } => {
            let res = crate::state::get_server_state()
                .db
                .update_todo_list(&list)
                .await;

            match res.clone() {
                Ok(_) => println!("Database: Updated todo list in database"),
                Err(e) => {
                    eprintln!("Database: Failed to update todolist in database!");
                    eprintln!("Database: Error: {}", e.to_string());
                }
            }

            res.ok()
        }
        ServerEvent::TodoListDelete { id } => {
            let res = crate::state::get_server_state()
                .db
                .delete_todo_list(&id)
                .await;

            match res.clone() {
                Ok(_) => println!("Database: Deleted todo list in database: {}", id),
                Err(e) => {
                    eprintln!("Database: Failed to delete todolist in database!");
                    eprintln!("Database: Error: {}", e.to_string());
                }
            }

            res.ok()
        }
    };
}

pub async fn start_event_listener() -> tokio::task::JoinHandle<()> {
    crate::state::add_server_event_handler(handle_event).await
}
