use futures::executor::block_on;
use futures::StreamExt;
use mongodb::options::FindOneAndUpdateOptions;

use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};

use crate::config;

pub fn convert_list_to_bson(list: &crate::state::TodoList) -> Option<Document> {
    let document = mongodb::bson::to_document(&mongodb::bson::to_bson(&list.list).ok()?).ok()?;

    Some(
        doc!["ListID": list.list_id, "ListName": list.list_name.clone(), "SerializedList": document],
    )
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

        let list_name = list
            .get_str("ListName")
            .or(Err(mongodb::error::Error::custom("JSON Error")))?;

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
            if let Some(document) = convert_list_to_bson(&list) {
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
    tokio::spawn(async move {
        let mut rx = {
            let (_, rx) = crate::state::get_event_channel();
            rx.clone()
        };

        while let Ok(event) = rx.recv().await {
            tokio::spawn(handle_event(event));
        }
    })
}
