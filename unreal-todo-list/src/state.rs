//use futures_lite::{future::block_on, stream::StreamExt};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoList {
    pub list_id: i64,
    pub list_name: String,
    pub list: serde_json::Value,
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

pub async fn broadcast_server_event(
    event: ServerEvent,
) -> Result<Option<ServerEvent>, async_broadcast::SendError<ServerEvent>> {
    let (tx, _) = get_event_channel();
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
