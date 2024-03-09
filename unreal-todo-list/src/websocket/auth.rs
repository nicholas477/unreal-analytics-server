use tokio_tungstenite::tungstenite::{
    handshake::server::{Request, Response},
    http,
};

/*
Request headers: {
    "host": "analytics.cactus.vg",
    "accept-encoding": "gzip",
    "authorization": "f7965ead-a6ea-43a8-bf30-d2ab5b22e533",
    "cache-control": "no-cache",
    "cdn-loop": "cloudflare",
    "cf-connecting-ip": "2600:1700:8df7:29f:7021:2ea0:da74:5336",
    "cf-ipcountry": "US",
    "cf-ray": "85fb5c6dc9816783-ATL",
    "cf-visitor": "{\"scheme\":\"https\"}",
    "cf-warp-tag-id": "f93f0d56-52f7-46fc-b912-0c3885f8d656",
    "connection": "Upgrade",
    "origin": "http://analytics.cactus.vg",
    "pragma": "no-cache",
    "sec-websocket-key": "NxaSCFMctTc+Nw5y+/JGzA==",
    "sec-websocket-version": "13",
    "upgrade": "websocket",
    "x-forwarded-for": "2600:1700:8df7:29f:7021:2ea0:da74:5336",
    "x-forwarded-proto": "https",
}
 */
fn get_connection_info(req: &Request) -> String {
    let mut out_string = String::new();

    if let Some(ip) = req.headers().get("cf-connecting-ip") {
        out_string += &format!("\n\tip: {}", ip.to_str().unwrap_or(""));
    }

    if let Some(country) = req.headers().get("cf-ipcountry") {
        out_string += &format!("\n\tcountry: {}", country.to_str().unwrap_or(""));
    }

    out_string
}

// Authorizes a websocket connection or drops the connection if unauthorized
pub fn authorize(
    req: &Request,
    response: Response,
) -> Result<http::Response<()>, http::Response<Option<String>>> {
    let mk_err = || {
        Response::builder()
            .status(tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED)
            .body(None)
            .unwrap()
    };

    let auth = req.headers().get("Authorization").ok_or(mk_err())?;

    let ref auth_key = crate::get_server_state().secrets.keys.todolist_auth_key;
    if auth == auth_key {
        println!(
            "Authorized socket connection for: {}",
            get_connection_info(req)
        );
        Ok(response)
    } else {
        println!("Unauthorized socket connection, dropping!");
        Err(mk_err())
    }
}
