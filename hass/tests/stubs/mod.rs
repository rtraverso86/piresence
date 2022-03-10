use std::{
    io,
    net::{
        TcpListener,
        TcpStream,
    },
    sync::{
        Arc,
        Barrier,
        mpsc::{
            self,
            Sender,
            TryRecvError,
        },
    },
    time::Duration,
    thread::{
        self,
        JoinHandle,
    },
};

use tungstenite::{
    accept_hdr,
    handshake::server::{Request, Response},
};

pub struct WsApiServer {
    host: String,
    port: u16,

    thread: JoinHandle<()>,
    tx: Sender<()>,
}

impl WsApiServer {
    pub fn new(port: u16) -> WsApiServer {
        let host = "127.0.0.1";
        let (tx, rx) = mpsc::channel();
        let sleep_interval = Duration::from_millis(10);
        let barrier = Arc::new(Barrier::new(2));

        let server_barrier = Arc::clone(&barrier);
        let server_thread = thread::spawn(move || {
            let listener = TcpListener::bind(format!("{}:{}", host, port)).unwrap();
            listener.set_nonblocking(true).expect("could not set non-blocking");
            server_barrier.wait();
            for stream in listener.incoming() {
                match stream {
                    Ok(s) => {
                        thread::spawn(move || {wsapi_server_handler(s); });
                    },
                    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                        match rx.try_recv() {
                            Ok(_) | Err(TryRecvError::Disconnected) => {
                                break;
                            }
                            Err(TryRecvError::Empty) => {}
                        }
                        thread::sleep(sleep_interval);
                        continue;
                    },
                    Err(e) => panic!("encountered IO error: {}", e),
                }
            }
        });

        barrier.wait();
        WsApiServer {
            host: String::from(host),
            port: port,
            thread: server_thread,
            tx: tx,
        }
    }

    pub fn stop(self) {
        self.tx.send(()).expect("could not send stop message");
        self.thread.join().expect("could not join thread");
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }


}

fn wsapi_server_handler(stream: TcpStream) {
    let callback = |req: &Request, mut response: Response| {
        println!("Received a new ws handshake");
        println!("The request's path is: {}", req.uri().path());
        println!("The request's headers are:");
        for (ref header, _value) in req.headers() {
            println!("* {}", header);
        }

        // Let's add an additional header to our response to the client.
        let headers = response.headers_mut();
        headers.append("MyCustomHeader", ":)".parse().unwrap());
        headers.append("SOME_TUNGSTENITE_HEADER", "header_value".parse().unwrap());

        Ok(response)
    };
    let mut websocket = accept_hdr(stream, callback).unwrap();

    loop {
        let msg = websocket.read_message().unwrap();
        println!("Message: {}", msg);
        if msg.is_binary() || msg.is_text() {
            websocket.write_message(msg).unwrap();
        }
    }
}
