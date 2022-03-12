use tungstenite::{connect, Message};
use url::Url;

pub mod json;
pub mod wsapi;
pub mod error;

use wsapi::WsApi;

pub fn wsconnect(host: &str, port: u16, token: &str) {

    let addr = format!("{}://{}:{}/api/websocket", "ws", host, port);

    let (mut socket, response) =
        connect(Url::parse(&addr).unwrap()).expect("Can't connect");

    println!("Connected to {}", &addr);
    println!("Response HTTP code: {}", response.status());
    println!("Response contains the following headers:");
    for (ref header, _value) in response.headers() {
        println!("* {}", header);
    }

    let msg = socket.read_message().expect("Error reading message");
    println!("Received: {}", msg);

    let msg = format!("{{\"type\": \"auth\", \"access_token\": \"{}\"}}", token);
    println!("Sending: {}", msg);
    socket.write_message(Message::Text(msg)).unwrap();

    let msg = socket.read_message().expect("Error reading message");
    println!("Received: {}", msg);

    socket.close(None).expect("Error closing socket");
}
