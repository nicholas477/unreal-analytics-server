pub mod config;
pub mod database;
pub mod github;
pub mod websocket;

#[macro_use]
extern crate rocket;

use once_cell::sync::OnceCell;

use std::sync::RwLock;

use std::io::Error as IoError;

#[derive(Debug)]
pub struct ServerState {
    pub db: database::Database,
    pub default_config: config::Config,
    pub config: RwLock<config::Config>, // Config can be changed later
    pub secrets: config::Secrets,
    pub github_state: RwLock<github::GithubState>,
}

impl ServerState {
    // Acquires a read lock on config and returns a copy of it
    pub fn read_config(&self) -> Option<config::Config> {
        let lock = self.config.read().ok()?;

        Some(lock.clone())
    }
}

pub static SERVER_STATE: OnceCell<ServerState> = OnceCell::new();
pub fn get_server_state() -> &'static ServerState {
    return &SERVER_STATE.get().unwrap();
}

async fn initialize() {
    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);

    let state = ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
        github_state: RwLock::new(github::GithubState::default()),
    };

    SERVER_STATE.set(state).unwrap();

    // Github state has to initialize after the server state since it depends on writing to github_state and reading from config
    github::initialize().await.unwrap();
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    initialize().await;

    let websocket_handle = websocket::start().await;
    let github_handle = github::start().await;

    futures::join!(websocket_handle, github_handle);

    Ok(())
}
