pub mod auth;

use futures::SinkExt;

use std::sync::Arc;

use std::{collections::HashMap, net::SocketAddr, sync::Mutex};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use tokio_tungstenite::accept_hdr_async;

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

// Creates a list of messages to send to connecting clients to update them on existing todo lists
async fn create_update_client_msg() -> Option<Vec<Message>> {
    let todo_lists = crate::get_server_state().db.get_todo_lists().await.ok()?;
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

// Update the list database with a new or updated list
async fn update_list_database(msg_json: &serde_json::Value) -> mongodb::error::Result<()> {
    if let Some(document) = crate::database::convert_list_to_bson(msg_json) {
        crate::get_server_state()
            .db
            .update_todo_list(&document)
            .await?;
    } else {
        eprintln!("Failed to convert message to json!!!!");
    }
    Ok(())
}

async fn parse_message(peer_map: &PeerMap, msg: &Message, addr: &SocketAddr) -> Option<()> {
    if let Ok(msg_str) = msg.to_text() {
        let msg_json: serde_json::Value = serde_json::from_str(msg_str).ok()?;

        let message_type = msg_json.get("MessageType")?.as_str()?;
        match message_type {
            "TodoListUpdate" => {
                let res = update_list_database(&msg_json).await;

                if let Err(e) = res {
                    eprintln!("Failed to update list in database! Error: {:#?}", e);
                }

                println!("Sending message back to other clients.");

                broadcast_message(&peer_map, &msg, &addr).unwrap();
            }
            "TodoListDelete" => {
                let todo_list_id = msg_json.get("ListID")?.as_i64()?;

                let res = crate::get_server_state()
                    .db
                    .delete_todo_list(&todo_list_id)
                    .await;

                if let Err(e) = res {
                    eprintln!("Error deleting todo list! Err: {}", e.to_string());
                    return None;
                }

                let todo_list_name = msg_json.get("ListName").map(|obj| obj.as_str()).flatten();
                println!("Deleted todo list: {:?} ({})", todo_list_name, todo_list_id);

                broadcast_message(&peer_map, &msg, &addr).unwrap();
            }
            str => {
                eprintln!("Unknown command type: {}", str);
                return None;
            }
        }
    }

    Some(())
}

pub fn broadcast_message(
    peer_map: &PeerMap,
    msg: &Message,
    addr: &SocketAddr,
) -> Result<(), futures_channel::mpsc::TrySendError<Message>> {
    let peers = peer_map.lock().unwrap();

    // We want to broadcast the message to everyone except ourselves.
    let broadcast_recipients = peers
        .iter()
        .filter(|(peer_addr, _)| peer_addr != &addr)
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
                    "Error during the websocket handshake occurred: {}",
                    e.to_string()
                );
                return;
            }
        }
    };
    println!("WebSocket connection established: {}", addr);

    // Insert the write part of this peer to the peer map.
    let (tx, rx) = unbounded();
    peer_map.lock().unwrap().insert(addr, tx);

    let (mut outgoing, incoming) = ws_stream.split();

    if let Some(update_messages) = create_update_client_msg().await {
        println!("Sending client {} todo lists...", update_messages.len());
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

    println!("{} disconnected", &addr);
    peer_map.lock().unwrap().remove(&addr);
}

pub async fn start() -> tokio::task::JoinHandle<()> {
    let (addr, port) = {
        let config = crate::get_server_state().read_config().unwrap();
        (config.websocket.address, config.websocket.port)
    };

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(format!("{}:{}", addr, port)).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Websocket listening on: {}:{}", addr, port);

    // Let's spawn the handling of each connection in a separate task.
    tokio::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tokio::spawn(handle_connection(state.clone(), stream, addr));
        }
    })
}
