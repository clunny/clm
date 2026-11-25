//! The Worker's own request and response types, so the core needs no `workers-rs`.

use std::any::Any;
use std::collections::BTreeMap;
use std::rc::Rc;

use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
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

    /// The first value of query parameter `name`, each `+` read as a space and then
    /// percent-decoded; none when the query holds no `name=` (`percent-coding`).
    pub fn param(&self, name: &str) -> Option<String> {
        self.query
            .split('&')
            .filter_map(|p| p.split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| percent_decode(&v.replace('+', " ")))
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

/// The bytes `percent_encode` writes as `%XX`: all but ASCII letters, digits, `-`, `_`, `.`
/// and `~`.
const ENCODED: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// `s` with every byte of its UTF-8 but ASCII letters, digits, `-`, `_`, `.` and `~`
/// written as `%` and two upper-case hex digits (`percent-coding`).
pub fn percent_encode(s: &str) -> String {
    utf8_percent_encode(s, ENCODED).to_string()
}

/// `s` with each `%` and two hex digits turned into that byte, any other `%` and every `+`
/// kept, and U+FFFD for bytes that aren't UTF-8 (`percent-coding`).
pub fn percent_decode(s: &str) -> String {
    percent_decode_str(s).decode_utf8_lossy().into_owned()
}
