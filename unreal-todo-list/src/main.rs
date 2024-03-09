pub mod config;
pub mod database;
pub mod github;
pub mod state;
pub mod websocket;

#[macro_use]
extern crate rocket;

use std::io::Error as IoError;
use std::sync::RwLock;

async fn initialize() {
    state::initialize_server_event_channel();

    let config = config::read_config();
    let keys = config::read_secrets();

    let db = database::connect_to_db(&config);
    let _db_event_handle = database::start_event_listener().await;

    let state = state::ServerState {
        db,
        default_config: config.clone(),
        config: RwLock::new(config),
        secrets: keys,
        github_state: RwLock::new(github::GithubState::default()),
    };

    state::initialize(state);

    // Github state has to initialize after the server state since it depends on writing to github_state and reading from config
    github::initialize().await.unwrap();
}

#[tokio::main]
async fn main() -> Result<(), IoError> {
    initialize().await;

    let websocket_handle = websocket::start().await;
    let github_handle = github::start().await;

    let _ = futures::join!(websocket_handle, github_handle);

    Ok(())
}
