use rocket::http::Status;

use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};
use rocket::State;

pub struct ApiKey(pub String);

#[derive(Debug)]
pub enum ApiKeyError {
    Missing,
    Invalid,
}

// Returns true if `key` is a valid API key string.
fn key_is_authorized(state: &State<crate::ServerState>, key: &str) -> bool {
    key == state.secrets.keys.cactus_auth_key
}

async fn load_state_check_key(
    request: &Request<'_>,
    header_api_key: &str,
) -> request::Outcome<ApiKey, ApiKeyError> {
    let state = request.guard::<&State<crate::ServerState>>().await;

    match state {
        rocket::outcome::Outcome::Success(state) => {
            if key_is_authorized(state, header_api_key) {
                Outcome::Success(ApiKey(header_api_key.to_string()))
            } else {
                request::Outcome::Error((Status::Unauthorized, ApiKeyError::Invalid))
            }
        }
        rocket::outcome::Outcome::Error(e) => {
            eprintln!("Error trying to read server state in FromRequest!");
            request::Outcome::Error((Status::InternalServerError, ApiKeyError::Invalid))
        }
        rocket::outcome::Outcome::Forward(e) => rocket::outcome::Outcome::Forward(e),
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ApiKeyError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(header_api_key) = request.headers().get_one("X-Api-Key") {
            load_state_check_key(request, header_api_key).await
        } else {
            request::Outcome::Error((Status::Unauthorized, ApiKeyError::Missing))
        }
    }
}
