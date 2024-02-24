use serde::{Deserialize, Serialize};
use std::fs;
use std::process::exit;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DiscordConfig {
    pub send_messages: bool,
    pub notify_editor_sessions: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub mongodb_connection_string: String,
    pub discord_config: DiscordConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Secrets {
    pub keys: Keys,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Keys {
    pub todolist_auth_key: String,
    pub cactus_auth_key: String,
    pub discord_webhook: String,
    pub discord_token: String,
    pub gitlab_token: String,
}

pub fn read_config() -> Config {
    let filename = "config/App.toml";

    let contents = fs::read_to_string(filename).unwrap_or_else(|_err| {
        eprintln!("Could not read config file! {}", filename);
        exit(1);
    });

    match toml::from_str(&contents) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Unable to load data from `{}`", filename);
            eprintln!("Error: {}", e.to_string());
            exit(1);
        }
    }
}

pub fn read_secrets() -> Secrets {
    let filename = "config/Secrets.toml";

    let contents = fs::read_to_string(filename).unwrap_or_else(|_err| {
        eprintln!("Could not read secrets file! {}", filename);
        exit(1);
    });

    match toml::from_str(&contents) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("Unable to load data from `{}`", filename);
            eprintln!("Error: {}", e.to_string());
            exit(1);
        }
    }
}
