use rocket::http::Status;

use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};



pub struct CloudflareIP(pub String);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CloudflareIP {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        if let Some(cloudflare_ip_string) = request.headers().get_one("CF-Connecting-IP") {
            Outcome::Success(CloudflareIP(cloudflare_ip_string.to_string()))
        } else {
            request::Outcome::Error((Status::InternalServerError, ()))
        }
    }
}
