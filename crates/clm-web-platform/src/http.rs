//! The Worker's own request and response types, so the core needs no `workers-rs`.

use std::any::Any;
use std::collections::BTreeMap;
use std::rc::Rc;

use serde::Serialize;

#[derive(Debug, Clone, Default)]
pub struct Request {
    pub method: String,
    /// The URL path, percent-encoded as received.
    pub path: String,
    /// The query string without `?`.
    pub query: String,
    /// Header names in lower case.
    pub headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
    /// The request URL's scheme and host, `https://clm-app.dev`.
    pub origin: String,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    /// The value of cookie `name`.
    pub fn cookie(&self, name: &str) -> Option<&str> {
        self.header("cookie")?
            .split(';')
            .filter_map(|c| c.trim().split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v)
    }

    /// The first value of query parameter `name`, percent-decoded.
    pub fn param(&self, name: &str) -> Option<String> {
        self.query
            .split('&')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| percent_decode(v))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
    /// A 101's socket, which only the platform makes and reads (`live-socket`).
    pub upgrade: Option<Upgrade>,
}

/// The platform's socket for a 101, opaque to the core: `worker.rs` fills it with the
/// `Live` object's WebSocket.
#[derive(Clone)]
pub struct Upgrade(Rc<dyn Any>);

impl Upgrade {
    pub fn new<T: Any>(socket: T) -> Upgrade {
        Upgrade(Rc::new(socket))
    }

    /// The socket, when it is a `T`.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.downcast_ref()
    }
}

impl std::fmt::Debug for Upgrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Upgrade")
    }
}

impl PartialEq for Upgrade {
    fn eq(&self, other: &Upgrade) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for Upgrade {}

impl Response {
    pub fn empty(status: u16) -> Response {
        Response {
            status,
            headers: Vec::new(),
            body: Vec::new(),
            upgrade: None,
        }
    }

    pub fn json<T: Serialize>(value: &T) -> Response {
        Response {
            status: 200,
            headers: vec![("content-type".into(), "application/json".into())],
            body: serde_json::to_vec(value).unwrap_or_default(),
            upgrade: None,
        }
    }

    pub fn html(status: u16, body: &str) -> Response {
        Response {
            status,
            headers: vec![("content-type".into(), "text/html; charset=utf-8".into())],
            body: body.as_bytes().to_vec(),
            upgrade: None,
        }
    }

    pub fn redirect(to: &str) -> Response {
        Response {
            status: 302,
            headers: vec![("location".into(), to.into())],
            body: Vec::new(),
            upgrade: None,
        }
    }

    pub fn with_header(mut self, name: &str, value: String) -> Response {
        self.headers.push((name.into(), value));
        self
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

pub fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

pub fn percent_decode(s: &str) -> String {
    let hex = |b: u8| (b as char).to_digit(16).map(|d| d as u8);
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let pair = (bytes[i] == b'%')
            .then(|| Some((hex(*bytes.get(i + 1)?)?, hex(*bytes.get(i + 2)?)?)))
            .flatten();
        match (pair, bytes[i]) {
            (Some((hi, lo)), _) => {
                out.push(hi << 4 | lo);
                i += 3;
                continue;
            }
            (None, b'+') => out.push(b' '),
            (None, b) => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
