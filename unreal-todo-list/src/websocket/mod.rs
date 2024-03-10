pub mod auth;

use futures::SinkExt;
use serde_json::json;

use std::sync::Arc;

use std::{collections::HashMap, net::SocketAddr, sync::Mutex};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use tokio_tungstenite::accept_hdr_async;

use crate::state::get_server_state;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

impl crate::state::TodoList {
    pub fn to_websocket_message(&self) -> serde_json::Value {
        json!({
            "ListID": self.list_id,
            "ListName": self.list_name,
            "GithubIssueID": self.github_issue_id,
            "SerializedList": self.list,
            "Deleted": self.deleted,
        })
    }

    pub fn from_websocket_message(msg_json: &serde_json::Value) -> Option<Self> {
        let list_id = msg_json.get("ListID")?.as_i64()?;
        let list_name = msg_json.get("ListName")?.as_str()?;
        let github_issue_id = (|| msg_json.get("GithubIssueID")?.as_i64())()
            .unwrap_or(crate::state::TodoList::default().github_issue_id);

        let list = msg_json.get("SerializedList")?;

        Some(crate::state::TodoList {
            list_id: list_id,
            list_name: list_name.into(),
            github_issue_id: github_issue_id,
            list: list.clone(),
            deleted: crate::state::TodoList::default().deleted,
        })
    }
}

// Creates a list of messages to send to connecting clients to update them on existing todo lists
async fn create_update_client_msg() -> Option<Vec<Message>> {
    let todo_lists = crate::state::get_server_state()
        .db
        .get_todo_lists(false)
        .await
        .ok()?;
    let mut messages: Vec<Message> = Vec::new();
    messages.reserve(todo_lists.len());

    for bson_list in todo_lists {
        let mut json_list = serde_json::to_value(&bson_list).unwrap();
        json_list.as_object_mut()?.insert(
            "MessageType".to_string(),
            serde_json::Value::String("TodoListUpdate".to_string()),
        );

        messages.push(Message::Text(
            serde_json::to_string_pretty(&json_list).ok()?,
        ));
    }

    Some(messages)
}

async fn parse_message(_peer_map: &PeerMap, msg: &Message, _addr: &SocketAddr) -> Option<()> {
    if let Ok(msg_str) = msg.to_text() {
        let msg_json: serde_json::Value = serde_json::from_str(msg_str).ok()?;
        println!("Websocket: New websocket message");

        let message_type = msg_json.get("MessageType")?.as_str()?;
        match message_type {
            "TodoListUpdate" => {
                let event = crate::state::ServerEvent::TodoListUpdate {
                    list: crate::state::TodoList::from_websocket_message(&msg_json)?,
                };

                if let Err(e) = crate::state::broadcast_server_event(event).await {
                    eprintln!(
                        "Websocket: Failed to broadcast server event for websocket TodoListUpdate message!"
                    );

                    eprintln!("Websocket: Error: {}", e.to_string());
                };
            }
            "TodoListDelete" => {
                let todo_list_id = msg_json.get("ListID")?.as_i64()?;

                let event = crate::state::ServerEvent::TodoListDelete { id: todo_list_id };

                if let Err(e) = crate::state::broadcast_server_event(event).await {
                    eprintln!(
                        "Websocket: Failed to broadcast server event for websocket TodoListDelete message!"
                    );

                    eprintln!("Websocket: Error: {}", e.to_string());
                };
            }
            "NewTodoList" => {
                let todo_list_id = {
                    let todo_lists = get_server_state().db.get_todo_lists(true).await.ok()?;
                    let mut new_id = 0;
                    for todo_list in todo_lists {
                        if let Some(todo_list) = crate::state::TodoList::from_bson(&todo_list) {
                            new_id = std::cmp::max(new_id, todo_list.list_id + 1);
                        } else {
                            eprintln!(
                                "Websocket: Failed to convert todo list from document to todo list type!"
                            );
                        }
                    }
                    new_id
                };

                println!(
                    "Websocket: Creating new todo list with id: {}",
                    todo_list_id
                );

                let mut new_json = msg_json.clone();
                {
                    new_json
                        .as_object_mut()?
                        .insert("ListID".into(), json!(todo_list_id));

                    let serialized_list_object = new_json
                        .as_object_mut()?
                        .get_mut("SerializedList")?
                        .as_object_mut()?;

                    let one: i64 = 1;
                    serialized_list_object
                        .get_mut("bIsNetworkedTodoList")?
                        .as_object_mut()?
                        .insert("__Value".into(), json!(one));

                    serialized_list_object
                        .get_mut("NetworkedTodoListID")?
                        .as_object_mut()?
                        .insert("__Value".into(), json!(todo_list_id));
                }

                let event = crate::state::ServerEvent::TodoListUpdate {
                    list: match crate::state::TodoList::from_websocket_message(&new_json) {
                        Some(list) => list,
                        None => {
                            eprintln!("Websocket: Failed to convert new todo list json to todo list type!");
                            eprintln!("Websocket: Json: {:#?}", new_json);
                            return None;
                        }
                    },
                };

                if let Err(e) = crate::state::broadcast_server_event(event).await {
                    eprintln!(
                        "Websocket: Failed to broadcast server event for websocket NewTodoList message!"
                    );

                    eprintln!("Websocket: Error: {}", e.to_string());
                };
            }
            str => {
                eprintln!("Websocket: Unknown command type: {}", str);
                return None;
            }
        }
    }

    Some(())
}

pub fn broadcast_message(
    peer_map: &PeerMap,
    msg: &Message,
    addr: Option<&SocketAddr>,
) -> Result<(), futures_channel::mpsc::TrySendError<Message>> {
    let peers = peer_map.lock().unwrap();

    // We want to broadcast the message to everyone except ourselves.
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| {
            if let Some(addr) = addr {
                return peer_addr != &addr;
            }
            return true;
        })
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(msg.clone())?;
    }

    Ok(())
}

pub fn ping(
    peer_map: &PeerMap,
    addr: &SocketAddr,
) -> Result<(), futures_channel::mpsc::TrySendError<Message>> {
    let peers = peer_map.lock().unwrap();

    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| peer_addr == &addr)
        .map(|(_, ws_sink)| ws_sink);

    for recp in broadcast_recipients {
        recp.unbounded_send(Message::Ping(Vec::new()))?;
    }

    Ok(())
}

async fn handle_connection(peer_map: PeerMap, raw_stream: TcpStream, addr: SocketAddr) {
    let ws_stream = {
        let res = accept_hdr_async(raw_stream, auth::authorize).await;

        match res {
            Ok(stream) => stream,
            Err(e) => {
                println!(
                    "Websocket: Error during the websocket handshake occurred: {}",
                    e.to_string()
                );
                return;
            }
        }
    };
    println!("Websocket: WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (mut outgoing, incoming) = ws_stream.split();

    if let Some(update_messages) = create_update_client_msg().await {
        println!(
            "Websocket: Sending client {} todo lists...",
            update_messages.len()
        );
        for msg in update_messages {
            outgoing.send(msg).await.unwrap();
        }
    }

    let handle_incoming = incoming.try_for_each(|msg| {
        if msg.is_text() {
            let _ = ping(&peer_map, &addr);

            futures::executor::block_on(parse_message(&peer_map, &msg, &addr));
        }
        future::ok(())
    });

    let handle_outgoing = rx.map(Ok).forward(outgoing);

    pin_mut!(handle_incoming, handle_outgoing);
    future::select(handle_incoming, handle_outgoing).await;

    println!("Websocket: {} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

// Event handler for server events
async fn handle_event(peer_map: PeerMap, event: crate::state::ServerEvent) -> Option<()> {
    use crate::state::ServerEvent;
    return match event {
        ServerEvent::TodoListUpdate { list } => {
            // Don't listen to events for deleted todo lists
            if list.deleted {
                return None;
            }

            println!("Websocket: Broadcasting todo list update to all peers");
            let mut msg_json = list.to_websocket_message();

            // Insert the message type
            msg_json
                .as_object_mut()?
                .insert("MessageType".into(), json!("TodoListUpdate"));

            let msg = serde_json::to_string(&msg_json).ok()?;
            let res = broadcast_message(&peer_map, &Message::text(msg), None);
            match res.clone() {
                Ok(_) => println!("Websocket: Broadcasted todo list update to websocket listeners"),
                Err(e) => {
                    eprintln!(
                        "Websocket: Failed to broadcast todo list update to websocket listeners!"
                    );
                    eprintln!("Websocket: Error: {}", e.to_string());
                }
            }

            res.ok()
        }
        ServerEvent::TodoListDelete { id } => {
            let msg_json = json!({
                "MessageType": "TodoListDelete",
                "ListID": id
            });

            let msg = serde_json::to_string(&msg_json).ok()?;
            let res = broadcast_message(&peer_map, &Message::text(msg), None);
            match res.clone() {
                Ok(_) => println!("Broadcasted todo list delete to websocket listeners"),
                Err(e) => {
                    eprintln!("Failed to broadcast todo list delete to websocket listeners!");
                    eprintln!("Error: {}", e.to_string());
                }
            }

            res.ok()
        }
    };
}

pub async fn start() -> tokio::task::JoinHandle<()> {
    let (addr, port) = {
        let config = crate::state::get_server_state().read_config().unwrap();
        (config.websocket.address, config.websocket.port)
    };

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    println!("Websocket: trying to bind to: {}:{}", addr, port);
    let try_socket = TcpListener::bind(format!("{}:{}", addr, port)).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Websocket: listening on: {}:{}", addr, port);

    // Let's spawn the handling of each connection in a separate task.
    tokio::spawn(async move {
        // Listen for server events so we can broadcast them back out to clients
        let listener_state = state.clone();
        crate::state::add_server_event_handler(move |event| {
            return handle_event(listener_state.clone(), event);
        })
        .await;

        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(state.clone(), stream, addr));
        }
    })
}
