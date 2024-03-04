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
fn key_is_authorized(state: &crate::ServerState, key: &str) -> bool {
    key == state.secrets.keys.todolist_auth_key
}

fn load_state_check_key(
    _request: &Request<'_>,
    header_api_key: &str,
) -> request::Outcome<ApiKey, ApiKeyError> {
    let state = crate::get_server_state();

    if key_is_authorized(&state, header_api_key) {
        Outcome::Success(ApiKey(header_api_key.to_string()))
    } else {
        request::Outcome::Error((Status::Unauthorized, ApiKeyError::Invalid))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ApiKeyError;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(header_api_key) = request.headers().get_one("Authorization") {
            load_state_check_key(request, header_api_key)
        } else {
            request::Outcome::Error((Status::Unauthorized, ApiKeyError::Missing))
        }
    }
}
