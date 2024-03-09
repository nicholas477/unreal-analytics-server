use futures::executor::block_on;
use futures::StreamExt;
use mongodb::options::FindOneAndUpdateOptions;

use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};

use crate::config;

impl crate::state::TodoList {
    pub fn to_bson(&self) -> Option<Document> {
        let document =
            mongodb::bson::to_document(&mongodb::bson::to_bson(&self.list).ok()?).ok()?;

        Some(
            doc!["ListID": self.list_id, "ListName": self.list_name.clone(), "SerializedList": document],
        )
    }

    pub fn from_bson(list: &Document) -> Option<Self> {
        let list_name = list.get_str("ListName").ok()?;
        let list_id = list.get_i64("ListID").ok()?;
        let list = list.get_document("SerializedList").ok()?;

        Some(crate::state::TodoList {
            list_id: list_id,
            list_name: list_name.into(),
            list: serde_json::to_value(list).ok()?,
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
        println!("Printing database names...");
        let db_names = block_on(self.client.list_database_names(None, None));
        for db_name in db_names.unwrap() {
            println!("{}", db_name);
        }

        println!("Printing collection names in {}", self.database.name());
        for collection_name in block_on(self.database.list_collection_names(None)).unwrap() {
            println!("{}", collection_name);
        }
    }

    pub async fn update_todo_list(&self, list: &Document) -> mongodb::error::Result<()> {
        let collection = self.database.collection::<Document>("todolists");

        let list_id = list
            .get_i64("ListID")
            .or(Err(mongodb::error::Error::custom("JSON Error")))?;

        // let list_name = list
        //     .get_str("ListName")
        //     .or(Err(mongodb::error::Error::custom("JSON Error")))?;

        let filter = doc! {
            "ListID": list_id
        };

        let update = doc! {
            "$set": list
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

        collection.delete_one(filter, None).await?;

        Ok(())
    }

    pub async fn get_todo_lists(&self) -> mongodb::error::Result<Vec<Document>> {
        let collection = self.database.collection::<Document>("todolists");

        let todo_lists = collection.find(None, None).await?;

        let vec = todo_lists.collect::<Vec<_>>().await;
        vec.into_iter().collect()
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
            if let Some(document) = list.to_bson() {
                let res = crate::state::get_server_state()
                    .db
                    .update_todo_list(&document)
                    .await;

                match res.clone() {
                    Ok(_) => println!("Updated todo list in database"),
                    Err(e) => {
                        eprintln!("Failed to update todolist in database!");
                        eprintln!("Error: {}", e.to_string());
                    }
                }

                res.ok()
            } else {
                eprintln!("Database event listener failed to convert message to json!!!!");
                None
            }
        }
        ServerEvent::TodoListDelete { id } => {
            let res = crate::state::get_server_state()
                .db
                .delete_todo_list(&id)
                .await;

            match res.clone() {
                Ok(_) => println!("Deleted todo list in database: {}", id),
                Err(e) => {
                    eprintln!("Failed to delete todolist in database!");
                    eprintln!("Error: {}", e.to_string());
                }
            }

            res.ok()
        }
    };
}

pub async fn start_event_listener() -> tokio::task::JoinHandle<()> {
    crate::state::add_server_event_handler(handle_event).await
}
