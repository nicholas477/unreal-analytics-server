use chrono::{self, DateTime};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iat: i64, // Issued at (as UTC timestamp) (To protect against clock drift, we recommend that you set this 60 seconds in the past)
    exp: i64, // Expiration time (as UTC timestamp)
    iss: String, // Issuer (Github App ID)
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: String,
}

impl AccessToken {
    pub fn get_expiration_datetime(&self) -> Option<chrono::DateTime<chrono::Utc>> {
        let date: chrono::DateTime<chrono::Utc> = self.expires_at.parse().ok()?;
        Some(date)
    }

    pub fn get_expiration_timestamp(&self) -> Option<i64> {
        let date = self.get_expiration_datetime()?;
        Some(date.timestamp())
    }
}

pub fn create_token(config: &crate::config::GithubConfig) -> Option<String> {
    let now = chrono::offset::Utc::now().timestamp();
    let claims = Claims {
        iat: now - 60,
        exp: now + (10 * 60),
        iss: config.app_id.to_string(),
    };

    let key = match std::fs::read(&config.app_key_file) {
        Ok(key) => key,
        Err(e) => {
            eprintln!(
                "Failed to read key file \"{}\" for github bot!",
                config.app_key_file
            );
            eprintln!("Error: {}", e.to_string());
            std::process::exit(-1);
        }
    };

    let encoded_key = match EncodingKey::from_rsa_pem(&key) {
        Ok(key) => key,
        Err(e) => {
            eprintln!("Failed to encode key for github bot!");
            eprintln!("Error: {}", e.to_string());
            std::process::exit(-1);
        }
    };

    let token = encode(&Header::new(Algorithm::RS256), &claims, &encoded_key).ok()?;

    Some(token)
}

pub async fn request_access_token() -> Option<AccessToken> {
    let github_config = crate::state::get_server_state()
        .config
        .try_read()
        .ok()?
        .github
        .clone();

    let installation_request = super::send_github_request_with_token(
        format!("/repos/{}/installation", github_config.repo),
        None,
        create_token(&github_config)?,
    )
    .await?;

    let installation_id = installation_request.get("id")?.as_i64()?;

    let access_token_request = super::send_github_request_with_token(
        format!("/app/installations/{}/access_tokens", installation_id),
        Some(http::method::Method::POST),
        create_token(&github_config)?,
    )
    .await?;

    let access_token = access_token_request.get("token")?.as_str()?;
    let expiration = access_token_request.get("expires_at")?.as_str()?;

    Some(AccessToken {
        token: access_token.to_string(),
        expires_at: expiration.to_string(),
    })
}

/// Returns true if the access token is going to expire soon
pub fn should_refresh_access_token() -> Option<bool> {
    let expiration_timestamp = crate::state::get_server_state()
        .github_state
        .try_read()
        .ok()?
        .access_token
        .get_expiration_timestamp()?;

    // If the token expires in 30 seconds or less
    Some((expiration_timestamp - chrono::offset::Utc::now().timestamp()) <= 30)
}

pub async fn refresh_access_token() -> Option<()> {
    println!("Refreshing access token");
    let access_token = request_access_token().await?;

    crate::state::get_server_state()
        .github_state
        .try_write()
        .ok()?
        .access_token = access_token;

    Some(())
}
