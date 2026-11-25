//! The guard every `/api/*` request passes: the app's own `Origin` for anything but a
//! `GET`, then a session, then a route (web.worker/api-guard).

mod common;

use clm_web_platform::http::Request;
use common::*;
use serde_json::json;

fn setup() -> (Fake, String) {
    let fake = Fake::default();
    let mut repo = FakeRepo::new("111", clm_repo());
    repo.branches.insert("spec/x".into(), "111".into());
    fake.gh.add_repo("o/a", repo);
    fake.set_view("o/a", "snapshot", json!({"nodes": []}));
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

/// A post of `body` to `path` from `origin`, or with no `Origin` header.
fn from(origin: Option<&str>, path: &str, cookie: &str, body: &[u8]) -> Request {
    let mut req = post(path, cookie, body);
    match origin {
        Some(o) => req.headers.insert("origin".into(), o.into()),
        None => req.headers.remove("origin"),
    };
    req
}

#[test]
fn another_origins_write() {
    // req:web.worker/api-guard@b1c7f110
    let (fake, cookie) = setup();
    let refs = || fake.gh.repos.borrow()["o/a"].branches.clone();
    let before = refs();
    let evil = Some("https://evil.example");
    let approve = json!({"fingerprint": "f"}).to_string();
    let proposal = json!({"title": "Export CSV", "body": "", "nodes": []}).to_string();
    let subscription = json!({
        "endpoint": "https://fcm.googleapis.com/fcm/send/1",
        "keys": {"p256dh": "BAAA", "auth": "AAAA"},
    })
    .to_string();
    for (origin, path, body) in [
        (evil, "/api/repos/o/a/branches/spec/x/approve", &approve),
        (None, "/api/repos/o/a/branches/spec/x/approve", &approve),
        (evil, "/api/repos/o/a/proposals", &proposal),
        (evil, "/api/repos/o/a/push", &subscription),
    ] {
        let res = fake.handle(from(origin, path, &cookie, body.as_bytes()));
        assert_eq!(res.status, 403, "{origin:?} {path}");
    }
    assert!(
        fake.gh.calls.borrow().is_empty(),
        "GitHub was asked nothing"
    );
    assert!(
        fake.replicas.calls.borrow().is_empty(),
        "the object was asked nothing"
    );
    assert_eq!(refs(), before, "nothing is written");
}

#[test]
fn a_path_no_route_answers() {
    // req:web.worker/api-guard@b1c7f110
    let (fake, cookie) = setup();
    let refs = || fake.gh.repos.borrow()["o/a"].branches.clone();
    let before = refs();
    for req in [
        get("/api/nope", &cookie),
        get("/api/repos/o/a/nope", &cookie),
        post("/api/repos/o/a/snapshot", &cookie, b""),
    ] {
        let what = format!("{} {}", req.method, req.path);
        assert_eq!(fake.handle(req).status, 404, "{what}");
    }
    assert_eq!(fake.handle(get("/api/nope", "")).status, 401);
    assert!(fake.gh.writes().is_empty(), "nothing is written");
    assert!(
        fake.replicas
            .calls
            .borrow()
            .iter()
            .all(|c| c.method == "GET"),
        "nothing is posted to the object"
    );
    assert_eq!(refs(), before);
}
