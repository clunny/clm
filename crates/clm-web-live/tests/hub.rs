mod common;

use clm_web_live::{Answer, Request, fetch, on_close, start};
use common::{Fake, block_on};

fn request(method: &str, path: &str, upgrade: Option<&str>, body: &[u8]) -> Request {
    Request {
        method: method.into(),
        path: path.into(),
        upgrade: upgrade.map(Into::into),
        body: body.into(),
    }
}

const LIVE: &str = "/api/repos/o/a/live";

#[test]
fn two_pages() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    let s1 = block_on(fetch(&host, request("GET", LIVE, Some("websocket"), b"")));
    let s2 = block_on(fetch(&host, request("GET", LIVE, Some("WebSocket"), b"")));
    let (Answer::Upgrade(s1), Answer::Upgrade(s2)) = (s1, s2) else {
        panic!("each upgrade is answered 101");
    };
    host.browser_closes(s1);
    on_close(&host, &s1);
    let notice = r#"{"delivery": "d1"}"#;
    let answer = block_on(fetch(
        &host,
        request("POST", "/notify", None, notice.as_bytes()),
    ));
    assert_eq!(answer, Answer::Status(204));
    assert_eq!(host.received(s2), [notice]);
    assert!(host.received(s1).is_empty());
    // With no socket held, again.
    let empty = Fake::default();
    let answer = block_on(fetch(
        &empty,
        request("POST", "/notify", None, notice.as_bytes()),
    ));
    assert_eq!(answer, Answer::Status(204));
}

#[test]
fn notify_with_no_sockets() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    let answer = block_on(fetch(&host, request("POST", "/notify", None, b"{}")));
    assert_eq!(answer, Answer::Status(204));
}

#[test]
fn an_upgrade_is_accepted() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    let answer = block_on(fetch(&host, request("GET", LIVE, Some("websocket"), b"")));
    assert_eq!(answer, Answer::Upgrade(0));
    assert!(host.is_open(0));
    // The accepted socket now hears notices.
    block_on(fetch(&host, request("POST", "/notify", None, b"n")));
    assert_eq!(host.received(0), ["n"]);
}

#[test]
fn not_a_socket() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    let s = host.connect();
    for upgrade in [None, Some("h2c")] {
        let answer = block_on(fetch(&host, request("GET", LIVE, upgrade, b"")));
        assert_eq!(answer, Answer::Status(426), "{upgrade:?}");
    }
    host.page_sends(s, "hello");
    assert!(host.is_open(s));
    // The socket errors: the object closes it.
    on_close(&host, &s);
    assert!(host.received(s).is_empty());
    assert!(!host.is_open(s));
}

#[test]
fn other_requests_are_not_found() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    let s = host.connect();
    for (method, path) in [("POST", "/other"), ("PUT", "/notify"), ("DELETE", "/")] {
        let answer = block_on(fetch(&host, request(method, path, None, b"x")));
        assert_eq!(answer, Answer::Status(404), "{method} {path}");
    }
    assert!(host.received(s).is_empty());
}

#[test]
fn a_body_that_is_not_text() {
    // req:web.live/hub@58b3db78
    // No VAPID secrets: `/send` refuses the body before it looks for them.
    let host = Fake::default();
    let s = host.connect();
    for (path, status) in [("/notify", 422), ("/send", 422), ("/other", 404)] {
        let answer = block_on(fetch(&host, request("POST", path, None, &[0xff, 0xfe])));
        assert_eq!(answer, Answer::Status(status), "{path}");
    }
    assert!(host.received(s).is_empty());
    assert!(host.sent.borrow().is_empty());
}

#[test]
fn no_socket_pair() {
    // req:web.live/hub@58b3db78
    let host = Fake {
        no_pair: true,
        ..Fake::default()
    };
    let answer = block_on(fetch(&host, request("GET", LIVE, Some("websocket"), b"")));
    assert_eq!(answer, Answer::Status(500));
    assert!(host.sockets.borrow().is_empty());
}

#[test]
fn ping_is_answered_pong() {
    // req:web.live/hub@58b3db78
    let host = Fake::default();
    start(&host);
    assert_eq!(
        *host.auto_response.borrow(),
        Some(("ping".to_owned(), "pong".to_owned()))
    );
    let s1 = host.connect();
    let s2 = host.connect();
    host.page_sends(s1, "ping");
    host.page_sends(s1, "ping");
    assert_eq!(host.received(s1), ["pong", "pong"]);
    assert!(host.received(s2).is_empty());
    assert!(host.is_open(s1) && host.is_open(s2));
}
