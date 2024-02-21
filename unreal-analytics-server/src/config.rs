use serde::Deserialize;
use std::fs;
use std::process::exit;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub mongodb_connection_string: String,
}

#[derive(Deserialize, Debug)]
pub struct Secrets {
    pub keys: Keys,
}

#[derive(Deserialize, Debug)]
pub struct Keys {
    pub todolist_auth_key: String,
    pub cactus_auth_key: String,
    pub discord_webhook: String,
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
        Err(_) => {
            eprintln!("Unable to load data from `{}`", filename);
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
        Err(_) => {
            eprintln!("Unable to load data from `{}`", filename);
            exit(1);
        }
    }
}
