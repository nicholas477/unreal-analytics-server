use tokio_tungstenite::tungstenite::{
    handshake::server::{Request, Response},
    http,
};

// Authorizes a websocket connection or drops the connection if unauthorized
pub fn authorize(
    req: &Request,
    mut response: Response,
) -> Result<http::Response<()>, http::Response<Option<String>>> {
    // println!("Received a new ws handshake");
    // println!("The request's path is: {}", req.uri().path());
    // println!("The request's headers are:");
    // for (ref header, _value) in req.headers() {
    //     println!("* {}: {:?}", header, _value);
    // }

    let mk_err = || {
        Response::builder()
            .status(tokio_tungstenite::tungstenite::http::StatusCode::UNAUTHORIZED)
            .body(None)
            .unwrap()
    };

    let auth = req.headers().get("Authorization").ok_or(mk_err())?;

    let ref auth_key = crate::get_server_state().secrets.keys.todolist_auth_key;
    if auth == auth_key {
        println!("Authorized socket connection");
        Ok(response)
    } else {
        println!("Unauthorized socket connection, dropping!");
        Err(mk_err())
    }
}
