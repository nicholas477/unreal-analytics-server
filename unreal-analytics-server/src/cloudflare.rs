use rocket::http::Status;

use rocket::request::Outcome;
use rocket::request::{self, FromRequest, Request};

use std::net::IpAddr;

// Country code to name
fn cc2n(code: &str) -> Option<&str> {
    use dia_i18n::iso_3166_1::{ALPHA2_CODES, ALPHA3_CODES, NUMERIC_CODES};

    match code.len() {
        2 => match code.chars().all(|c| match c {
            'a'..='z' | 'A'..='Z' => true,
            _ => false,
        }) {
            true => match ALPHA2_CODES
                .iter()
                .find(|c| c.code().eq_ignore_ascii_case(&code))
            {
                Some(code) => Some(code.country_name()),
                None => return None,
            },
            false => return None,
        },
        3 => match code.chars().all(|c| match c {
            'a'..='z' | 'A'..='Z' => true,
            _ => false,
        }) {
            true => match ALPHA3_CODES
                .iter()
                .find(|c| c.code().eq_ignore_ascii_case(&code))
            {
                Some(code) => Some(code.country_name()),
                None => return None,
            },
            false => match code.chars().all(|c| c >= '0' && c <= '9') {
                true => match NUMERIC_CODES.iter().find(|c| c.code() == code) {
                    Some(code) => Some(code.country_name()),
                    None => return None,
                },
                false => return None,
            },
        },
        _ => None,
    }
}

#[derive(Debug)]
pub struct CloudflareInfo {
    pub ip: IpAddr,
    pub country: String,
}

impl CloudflareInfo {
    pub fn get_country_name(&self) -> String {
        match self.country.as_str() {
            "XX" => "No Data",
            "T1" => "Tor",
            cn => cc2n(cn).unwrap_or("Unknown"),
        }
        .to_string()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CloudflareInfo {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let server_error = request::Outcome::Error((Status::InternalServerError, ()));

        let ip = match request
            .headers()
            .get_one("CF-Connecting-IP")
            .map(|ip| ip.parse::<IpAddr>())
        {
            Some(Ok(ip)) => ip,
            _ => return server_error,
        };

        let country = match request.headers().get_one("CF-IPCountry") {
            Some(country) => country.to_string(),
            _ => "unknown".to_string(),
        };

        return Outcome::Success(CloudflareInfo { ip, country });
    }
}
