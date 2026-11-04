mod common;

use clm_web_live::{Answer, Request, fetch, on_close, on_message};
use common::{Fake, block_on};

fn request(method: &str, path: &str, websocket: bool, body: &str) -> Request {
    Request {
        method: method.into(),
        path: path.into(),
        websocket,
        body: body.into(),
    }
}

#[test]
fn two_pages() {
    // req:web.live/hub@688e0569
    let host = Fake::default();
    let s1 = host.connect();
    let s2 = host.connect();
    host.browser_closes(s1);
    on_close(&host, &s1);
    let notice = r#"{"delivery": "d1"}"#;
    let answer = block_on(fetch(&host, request("POST", "/notify", false, notice)));
    assert_eq!(answer, Answer::Status(204));
    assert_eq!(host.received(s2), [notice]);
    assert!(host.received(s1).is_empty());
}

#[test]
fn notify_with_no_sockets() {
    // req:web.live/hub@688e0569
    let host = Fake::default();
    let answer = block_on(fetch(&host, request("POST", "/notify", false, "{}")));
    assert_eq!(answer, Answer::Status(204));
}

#[test]
fn an_upgrade_is_accepted() {
    // req:web.live/hub@688e0569
    let host = Fake::default();
    let answer = block_on(fetch(
        &host,
        request("GET", "/api/repos/o/a/live", true, ""),
    ));
    assert_eq!(answer, Answer::Upgrade(0));
    assert!(host.is_open(0));
    // The accepted socket now hears notices.
    block_on(fetch(&host, request("POST", "/notify", false, "n")));
    assert_eq!(host.received(0), ["n"]);
}

#[test]
fn not_a_socket() {
    // req:web.live/hub@688e0569
    let host = Fake::default();
    let s = host.connect();
    let answer = block_on(fetch(
        &host,
        request("GET", "/api/repos/o/a/live", false, ""),
    ));
    assert_eq!(answer, Answer::Status(426));
    on_message(&host, &s, "hello");
    assert!(host.is_open(s));
    // The socket errors: the object closes it.
    on_close(&host, &s);
    assert!(host.received(s).is_empty());
    assert!(!host.is_open(s));
}

#[test]
fn other_requests_are_not_found() {
    // req:web.live/hub@688e0569
    let host = Fake::default();
    let s = host.connect();
    for (method, path) in [("POST", "/other"), ("PUT", "/notify"), ("DELETE", "/")] {
        let answer = block_on(fetch(&host, request(method, path, false, "x")));
        assert_eq!(answer, Answer::Status(404), "{method} {path}");
    }
    assert!(host.received(s).is_empty());
}
