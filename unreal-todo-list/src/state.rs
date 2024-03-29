//use futures_lite::{future::block_on, stream::StreamExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

// Example serialized list: {
// "ListName": {
//     "__Type": "TextProperty",
//     "__Value": {
//       "Flags": 16,
//       "HistoryType": 0,
//       "Namespace": "",
//       "Key": "",
//       "SourceString": "March 20th Task List"
//     }
//   },
//   "Tasks": {
//     "__Type": "ArrayProperty",
//     "__InnerType": "StructProperty",
//     "__InnerStructName": "TodoTask",
//     "__Value": [
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "Finish the level"
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 0,
//             "HistoryType": -1,
//             "bHasCultureInvariantString": false
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "Done"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": []
//         }
//       },
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "Remove the ability to equip a knife, just make it \"press E to kill\""
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": -1,
//             "bHasCultureInvariantString": false
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "Done"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": []
//         }
//       },
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "Make the NPC give both the knife and the gun"
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": -1,
//             "bHasCultureInvariantString": false
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "Done"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": [
//             {
//               "PackageName": {
//                 "__Type": "NameProperty",
//                 "__Value": "/Game/Cactus/Maps/ModularLevels/Blueprints/Dialogue/DLG_KnifeOrGun"
//               },
//               "AssetName": {
//                 "__Type": "NameProperty",
//                 "__Value": "DLG_KnifeOrGun"
//               }
//             }
//           ]
//         }
//       },
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "Fix the crash with the weapon customization menu"
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 0,
//             "HistoryType": -1,
//             "bHasCultureInvariantString": false
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "Done"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": []
//         }
//       },
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "(Nick) Fix the collision on the shopping carts"
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 0,
//             "HistoryType": -1,
//             "bHasCultureInvariantString": false
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "None"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": []
//         }
//       },
//       {
//         "TaskName": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "(Mac) Add a sound for when the door bar is pryed open"
//           }
//         },
//         "TaskDescription": {
//           "__Type": "TextProperty",
//           "__Value": {
//             "Flags": 16,
//             "HistoryType": 0,
//             "Namespace": "",
//             "Key": "",
//             "SourceString": "In the dialogue, you can pry open the door barricade with a knife, but there is no sound for when that happens. Add in a sound for the door barricade being opened."
//           }
//         },
//         "TaskStatus": {
//           "__Type": "NameProperty",
//           "__Value": "Todo"
//         },
//         "LinkedAssets": {
//           "__Type": "ArrayProperty",
//           "__InnerType": "StructProperty",
//           "__InnerStructName": "TopLevelAssetPath",
//           "__Value": [
//             {
//               "PackageName": {
//                 "__Type": "NameProperty",
//                 "__Value": "/Game/Cactus/Blueprints/Core/Interactables/Doors/CrashDoor/DLG_DoorBarricade"
//               },
//               "AssetName": {
//                 "__Type": "NameProperty",
//                 "__Value": "DLG_DoorBarricade"
//               }
//             },
//             {
//               "PackageName": {
//                 "__Type": "NameProperty",
//                 "__Value": "/Game/Cactus/Blueprints/Core/Interactables/Doors/CrashDoor/BP_DoorBarricade"
//               },
//               "AssetName": {
//                 "__Type": "NameProperty",
//                 "__Value": "BP_DoorBarricade"
//               }
//             }
//           ]
//         }
//       }
//     ]
//   },
//   "bIsNetworkedTodoList": {
//     "__Type": "BoolProperty",
//     "__Value": 1
//   },
//   "NetworkedTodoListID": {
//     "__Type": "IntProperty",
//     "__Value": 8
//   },
//   "GithubIssueID": {
//     "__Type": "IntProperty",
//     "__Value": 2147483647
//   }
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoList {
    pub list_id: i64,
    pub list_name: String,
    pub github_issue_id: i64,
    pub list: serde_json::Value,
    pub deleted: bool,
}

impl TodoList {
    pub fn get_task_array(&self) -> Option<&Vec<serde_json::Value>> {
        self.list
            .as_object()?
            .get("Tasks")?
            .get("__Value")?
            .as_array()
    }
}

impl Default for TodoList {
    fn default() -> TodoList {
        TodoList {
            list_id: -1,
            list_name: "Todo List".into(),
            github_issue_id: -1,
            list: serde_json::Value::Null,
            deleted: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ServerEvent {
    TodoListUpdate { list: TodoList },
    TodoListDelete { id: i64 },
}

impl ServerEvent {
    pub fn get_event_enum_name(&self) -> String {
        match self {
            ServerEvent::TodoListUpdate { list: _ } => "TodoListUpdate".into(),
            ServerEvent::TodoListDelete { id: _ } => "TodoListDelete".into(),
        }
    }
}

#[derive(Debug)]
pub struct ServerState {
    pub db: crate::database::Database,
    pub default_config: crate::config::Config,
    pub config: RwLock<crate::config::Config>, // Config can be changed later
    pub secrets: crate::config::Secrets,
    pub github_state: RwLock<crate::github::GithubState>,
}

impl ServerState {
    // Acquires a read lock on config and returns a copy of it
    pub fn read_config(&self) -> Option<crate::config::Config> {
        let lock = self.config.read().ok()?;

        Some(lock.clone())
    }
}

pub static SERVER_STATE: OnceCell<ServerState> = OnceCell::new();
pub fn get_server_state() -> &'static ServerState {
    return &SERVER_STATE.get().unwrap();
}

pub type ServerEventChannel = (
    async_broadcast::Sender<ServerEvent>,
    async_broadcast::Receiver<ServerEvent>,
);
pub static SERVER_EVENT_CHANNEL: OnceCell<ServerEventChannel> = OnceCell::new();
pub fn get_event_channel() -> &'static ServerEventChannel {
    return &SERVER_EVENT_CHANNEL.get().unwrap();
}

pub fn initialize_server_event_channel() {
    let (mut tx, rx) = async_broadcast::broadcast::<ServerEvent>(1);
    tx.set_overflow(true);

    SERVER_EVENT_CHANNEL
        .set((tx, rx))
        .expect("Failed to set server event channel!");
}

fn fix_ids(list: &TodoList) -> Option<TodoList> {
    use serde_json::json;
    let mut list = list.clone();

    let one: i64 = 1;
    let list_object = list.list.as_object_mut()?;

    // Sets or creates a property on the list
    let mut set_or_insert_property =
        |property: &str, property_type: &str, value: serde_json::Value| -> Option<()> {
            if list_object.get(property).is_none() {
                list_object.insert(
                    property.into(),
                    json!({"__Type": property_type, "__Value": value}),
                );
            } else {
                list_object
                    .get_mut(property)?
                    .as_object_mut()?
                    .insert("__Value".into(), value);
            }

            Some(())
        };

    set_or_insert_property("bIsNetworkedTodoList", "BoolProperty", json!(one))?;
    set_or_insert_property("NetworkedTodoListID", "IntProperty", json!(list.list_id))?;
    set_or_insert_property("GithubIssueID", "IntProperty", json!(list.github_issue_id))?;

    Some(list)
}

async fn pretransform_event(event: ServerEvent) -> ServerEvent {
    let mut event = event.clone();
    match event {
        ServerEvent::TodoListUpdate { ref mut list } => {
            if let Some(github_issue_id) = list.get_github_id().await {
                list.github_issue_id = github_issue_id;
            }
            list.clone_from(&fix_ids(list).unwrap());
        }
        _ => (),
    }

    println!("Pretransformed event");

    return event;
}

pub async fn broadcast_server_event(
    event: ServerEvent,
) -> Result<Option<ServerEvent>, async_broadcast::SendError<ServerEvent>> {
    let event = pretransform_event(event).await;

    let (tx, _) = get_event_channel();
    println!("========================================");
    println!("Broadcasting server event: {}", event.get_event_enum_name());
    tx.broadcast(event.clone()).await
}

pub async fn add_server_event_handler<Future, Func: (Fn(ServerEvent) -> Future)>(
    f: Func,
) -> tokio::task::JoinHandle<()>
where
    Future: futures::Future + Send + 'static,
    Future::Output: Send + 'static,
    Func: Send + 'static,
{
    tokio::spawn(async move {
        let mut rx = {
            let (_, rx) = crate::state::get_event_channel();
            rx.clone()
        };

        while let Ok(event) = rx.recv().await {
            tokio::spawn(f(event));
        }
    })
}

pub fn initialize(state: ServerState) {
    SERVER_STATE
        .set(state)
        .expect("Failed to set server state!");
}
