use serde::{Deserialize, Serialize};
use std::fs;
use std::process::exit;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GithubConfig {
    pub enabled: bool,
    pub app_id: u64,
    pub app_key_file: String,
    pub app_name: Option<String>,
    pub repo: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PortBindConfig {
    pub address: String,
    pub port: u16,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub mongodb_connection_string: String,
    pub websocket: PortBindConfig,
    pub github: GithubConfig,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Secrets {
    pub keys: Keys,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Keys {
    pub todolist_auth_key: String,
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
