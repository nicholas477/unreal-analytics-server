use serde::Deserialize;
use std::fs;
use std::process::exit;

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Secrets {
    keys: Keys,
}

#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct Keys {
    todolist_auth_key: String,
    cactus_auth_key: String,
    discord_webhook: String,
    gitlab_token: String,
}

pub fn read_secrets() -> Secrets {
    let filename = "config/Secrets.toml";

    let contents = match fs::read_to_string(filename) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Could not read secrets file `{}`", filename);
            exit(1);
        }
    };

    match toml::from_str(&contents) {
        Ok(d) => d,
        Err(_) => {
            eprintln!("Unable to load data from `{}`", filename);
            exit(1);
        }
    }
}
