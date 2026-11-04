//! `GET /api/repos/{o}/{r}/live`: 403 from any other origin, then the session and access
//! checks every API route makes, then the request forwarded to the repo's `Live` object,
//! whose 101 the user gets.

mod common;

use common::*;

/// A request from the app's own page, as a browser sends it.
fn same_origin(path: &str, cookie: &str) -> clm_web_platform::http::Request {
    let mut req = get(path, cookie);
    req.headers.insert("origin".into(), ORIGIN.into());
    req
}

fn upgrade(path: &str, cookie: &str) -> clm_web_platform::http::Request {
    let mut req = same_origin(path, cookie);
    req.headers.insert("upgrade".into(), "websocket".into());
    req
}

fn setup() -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh
        .add_repo("o/private", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

#[test]
fn forwarded() {
    // req:web.worker/live-socket@80387388
    let (fake, cookie) = setup();
    let res = fake.handle(upgrade("/api/repos/o/a/live", &cookie));
    assert_eq!(res.status, 101);
    assert!(res.upgrade.is_some(), "the object's socket");
    let calls = fake.live.calls.borrow();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].name, "o/a");
    assert_eq!(calls[0].method, "GET");
    assert!(calls[0].websocket);
}

#[test]
fn another_origin() {
    // req:web.worker/live-socket@80387388
    let (fake, cookie) = setup();
    let mut req = upgrade("/api/repos/o/a/live", &cookie);
    req.headers
        .insert("origin".into(), "https://evil.example".into());
    assert_eq!(fake.handle(req).status, 403);
    let mut req = upgrade("/api/repos/o/a/live", &cookie);
    req.headers.remove("origin");
    assert_eq!(fake.handle(req).status, 403);
    assert!(fake.live.calls.borrow().is_empty(), "nothing forwarded");
}

#[test]
fn origin_before_any_other_check() {
    // req:web.worker/live-socket@80387388
    // No session, no access, no upgrade: each still gets 403, not 401, 404 or 426.
    let (fake, cookie) = setup();
    let mut req = upgrade("/api/repos/o/a/live", "");
    req.headers
        .insert("origin".into(), "https://evil.example".into());
    assert_eq!(fake.handle(req).status, 403);
    let mut req = upgrade("/api/repos/o/private/live", &cookie);
    req.headers.remove("origin");
    assert_eq!(fake.handle(req).status, 403);
    assert_eq!(fake.handle(get("/api/repos/o/a/live", &cookie)).status, 403);
    assert!(fake.live.calls.borrow().is_empty());
}

#[test]
fn no_access() {
    // req:web.worker/live-socket@80387388
    let (fake, cookie) = setup();
    let res = fake.handle(upgrade("/api/repos/o/private/live", &cookie));
    assert_eq!(res.status, 404);
    assert!(fake.live.calls.borrow().is_empty(), "nothing forwarded");
}

#[test]
fn no_session() {
    // req:web.worker/live-socket@80387388
    let (fake, _) = setup();
    let res = fake.handle(upgrade("/api/repos/o/a/live", ""));
    assert_eq!(res.status, 401);
    let res = fake.handle(upgrade("/api/repos/o/a/live", "__Host-clm=tampered"));
    assert_eq!(res.status, 401);
    assert!(fake.live.calls.borrow().is_empty());
}

#[test]
fn without_the_upgrade() {
    // req:web.worker/live-socket@80387388
    let (fake, cookie) = setup();
    let res = fake.handle(same_origin("/api/repos/o/a/live", &cookie));
    assert_eq!(res.status, 426);
    assert!(fake.live.calls.borrow().is_empty());
}

#[test]
fn without_the_binding() {
    // req:web.worker/live-socket@80387388
    let (fake, cookie) = setup();
    fake.live.bound.set(false);
    let res = fake.handle(upgrade("/api/repos/o/a/live", &cookie));
    assert_eq!(res.status, 503);
}

#[test]
fn renewed_session_keeps_its_cookie() {
    // req:web.worker/live-socket@80387388
    // A session near its expiry is renewed on the 101, as on every API route.
    let (fake, cookie) = setup();
    fake.gh.grant("u-refresh-renewed", "o/a");
    fake.now.set(NOW + 8 * 3600 - 60);
    let res = fake.handle(upgrade("/api/repos/o/a/live", &cookie));
    assert_eq!(res.status, 101);
    assert!(res.header("set-cookie").is_some(), "{res:?}");
}
