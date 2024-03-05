pub mod auth;
pub mod config;
pub mod database;

use futures::SinkExt;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::mpsc::TrySendError;
use std::sync::Arc;
use std::sync::RwLock;
use tokio_tungstenite::tungstenite::handshake::server::ErrorResponse;
use tungstenite::http::StatusCode;

#[derive(Debug)]
pub struct ServerState {
    pub db: database::Database,
    pub default_config: config::Config,
    pub config: RwLock<config::Config>, // Config can be changed later
    pub secrets: config::Secrets,
}

impl ServerState {
    // Acquires a read lock on config and returns a copy of it
    pub fn read_config(&self) -> Option<config::Config> {
        let lock = self.config.read().ok()?;

        Some(lock.clone())
    }
}

pub static SERVER_STATE: OnceCell<Arc<ServerState>> = OnceCell::new();
pub fn get_server_state() -> Arc<ServerState> {
    SERVER_STATE.get().unwrap().clone()
}

async fn initialize() {
    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);

    let state = Arc::new(ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
    });

    SERVER_STATE.set(state.clone()).unwrap();
}

// Creates a list of messages to send to connecting clients to update them on existing todo lists
async fn create_update_client_msg() -> Option<Vec<Message>> {
    let todo_lists = get_server_state().db.get_todo_lists().await.ok()?;
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
    if let Some(document) = database::convert_list_to_bson(msg_json) {
        get_server_state().db.update_todo_list(&document).await?;
    } else {
        eprintln!("Failed to convert message to json!!!!");
    }
    Ok(())
}

use std::{collections::HashMap, io::Error as IoError, net::SocketAddr, sync::Mutex};

use futures_channel::mpsc::{unbounded, UnboundedSender};
use futures_util::{future, pin_mut, stream::TryStreamExt, StreamExt};

use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::protocol::Message;

use tokio_tungstenite::{
    accept_hdr_async,
    tungstenite::{
        connect,
        handshake::server::{Request, Response},
    },
};

type Tx = UnboundedSender<Message>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

async fn parse_message(peer_map: &PeerMap, msg: &Message, addr: &SocketAddr) -> Option<()> {
    if let Ok(msg_str) = msg.to_text() {
        let msg_json: serde_json::Value = serde_json::from_str(msg_str).ok()?;
        let res = update_list_database(&msg_json).await;

        if let Err(e) = res {
            eprintln!("Failed to update list in database! Error: {:#?}", e);
        }

        println!("Sending message back to other clients.");

        broadcast_message(&peer_map, &msg, &addr).unwrap();
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
    let mut ws_stream = {
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

#[tokio::main]
async fn main() -> Result<(), IoError> {
    initialize().await;

    let (addr, port) = {
        let config = get_server_state().read_config().unwrap();
        (config.address, config.port)
    };

    let state = PeerMap::new(Mutex::new(HashMap::new()));

    // Create the event loop and TCP listener we'll accept connections on.
    let try_socket = TcpListener::bind(format!("{}:{}", addr, port)).await;
    let listener = try_socket.expect("Failed to bind");
    println!("Listening on: {}:{}", addr, port);

    // Let's spawn the handling of each connection in a separate task.
    while let Ok((stream, addr)) = listener.accept().await {
        tokio::spawn(handle_connection(state.clone(), stream, addr));
    }

    Ok(())
}
