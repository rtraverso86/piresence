use std::env;
use tungstenite::{connect, Message};
use url::Url;

fn main() {
    let host = env::var("HA_HOST").expect("environment variable missing: HA_HOST");
    let port : u16 = env::var("HA_PORT")
        .unwrap_or("8123".into())
        //.expect("environment variable missing: HA_PORT")
        .parse()
        .expect("environment variable error: HA_PORT only accepts 16 bit unsigned integers");
    let token = env::var("HA_TOKEN")
        .unwrap_or("ABCDEFG".into());

    env_logger::init();
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
