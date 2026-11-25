//! A local HTTP/1.1 server answering each request from a handler and keeping what it was sent.
//!
//! The handler gets each [`Request`] and returns the status and JSON body; [`Server::requests`] lists what came,
//! so a test asserts on the calls made and the headers sent.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

/// The largest request head read, bytes
pub const HEAD_SIZE_BYTES_MAX: usize = 64 << 10;
/// The largest request body read, bytes
pub const BODY_SIZE_BYTES_MAX: usize = 16 << 20;

/// One request as received
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    /// `/params/by_mpn`
    pub path: String,
    /// The `Authorization` header, as sent
    pub authorization: Option<String>,
    pub body: Vec<u8>,
}

impl Request {
    /// The body as text
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

/// An answer: status code and a JSON body
pub type Answer = (u16, String);

type Handler = dyn Fn(&Request) -> Answer + Send + Sync;

/// A running server; it stops with the test process
pub struct Server {
    url: String,
    requests: Arc<Mutex<Vec<Request>>>,
}

impl Server {
    /// Listens on a free port of 127.0.0.1 and answers from `handler` on its own thread
    ///
    /// # Panics
    ///
    /// When no port can be bound.
    pub fn start(handler: impl Fn(&Request) -> Answer + Send + Sync + 'static) -> Server {
        let listener = TcpListener::bind("127.0.0.1:0").expect("a free port on 127.0.0.1");
        let url = format!(
            "http://{}",
            listener
                .local_addr()
                .expect("a bound listener has an address")
        );
        let requests = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&requests);
        let handler: Box<Handler> = Box::new(handler);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                serve(stream, handler.as_ref(), &kept);
            }
        });
        Server { url, requests }
    }

    /// `http://127.0.0.1:PORT`
    pub fn url(&self) -> &str {
        &self.url
    }

    /// Every request so far, in order
    ///
    /// # Panics
    ///
    /// When the server thread panicked while holding the list.
    pub fn requests(&self) -> Vec<Request> {
        self.requests.lock().expect("no panic while held").clone()
    }
}

/// Reads one request, answers it and closes; a request that cannot be read is answered 400
fn serve(stream: TcpStream, handler: &Handler, kept: &Mutex<Vec<Request>>) {
    let mut reader = BufReader::new(stream.try_clone().expect("a socket clones"));
    let answer = match read_request(&mut reader) {
        Some(req) => {
            let answer = handler(&req);
            kept.lock().expect("no panic while held").push(req);
            answer
        }
        None => (400, "{\"error\":\"bad request\"}".to_owned()),
    };
    let (status, body) = answer;
    let head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        if status < 400 { "OK" } else { "Error" },
        body.len()
    );
    let mut stream = stream;
    let sent = stream
        .write_all(head.as_bytes())
        .and_then(|()| stream.write_all(body.as_bytes()))
        .and_then(|()| stream.flush());
    debug_assert!(sent.is_ok(), "the client reads its answer");
}

fn read_request(reader: &mut BufReader<TcpStream>) -> Option<Request> {
    let mut head = String::new();
    let mut req = Request::default();
    let mut length = 0usize;
    loop {
        let mut line = String::new();
        let n = reader.read_line(&mut line).ok()?;
        head.push_str(&line);
        if n == 0 || head.len() > HEAD_SIZE_BYTES_MAX {
            return None;
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if req.method.is_empty() {
            let mut parts = line.split(' ');
            req.method = parts.next()?.to_owned();
            req.path = parts.next()?.to_owned();
        } else if let Some((name, value)) = line.split_once(':') {
            let value = value.trim();
            if name.eq_ignore_ascii_case("content-length") {
                length = value.parse().ok()?;
            } else if name.eq_ignore_ascii_case("authorization") {
                req.authorization = Some(value.to_owned());
            }
        }
    }
    if length > BODY_SIZE_BYTES_MAX {
        return None;
    }
    req.body = vec![0; length];
    reader.read_exact(&mut req.body).ok()?;
    Some(req)
}
