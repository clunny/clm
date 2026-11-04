//! Push subscriptions (`push-subscriptions-in-ledger`): the VAPID public key, and a repo's
//! Notify me switch as `subscribed` and `unsubscribed` records in the user's web ledger.

mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web_platform::http::Request;
use common::*;
use serde_json::{Value, json};

const ENDPOINT: &str = "https://fcm.googleapis.com/fcm/send/1";

fn setup() -> (Fake, String) {
    let mut fake = Fake::default();
    fake.secrets
        .insert("VAPID_PUBLIC_KEY".into(), "BPublicKey".into());
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        (
            "application/json".into(),
            br#"["clunny", "alice", "bob"]"#.to_vec(),
        ),
    );
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh
        .add_repo("o/private", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    let cookie = fake.cookie_as("u", "alice");
    (fake, cookie)
}

fn p256dh() -> String {
    let mut point = vec![4u8];
    point.extend([1u8; 64]);
    URL_SAFE_NO_PAD.encode(point)
}

fn auth(len: usize) -> String {
    URL_SAFE_NO_PAD.encode(vec![9u8; len])
}

fn subscription(endpoint: &str, auth_len: usize) -> Vec<u8> {
    json!({"endpoint": endpoint, "keys": {"p256dh": p256dh(), "auth": auth(auth_len)}})
        .to_string()
        .into_bytes()
}

fn delete(path: &str, cookie: &str, body: &[u8]) -> Request {
    let mut req = post(path, cookie, body);
    req.method = "DELETE".into();
    req
}

/// The records of `o/a`'s `clm/ledger/<writer>`, at its tip.
fn records(fake: &Fake, writer: &str) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let Some(tip) = repo.branches.get(&format!("clm/ledger/{writer}")) else {
        return Vec::new();
    };
    let text = std::str::from_utf8(&repo.commits[tip]["records.jsonl"]).unwrap();
    text.lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn subscribed(fake: &Fake, cookie: &str, endpoint: &str) -> bool {
    let path = format!(
        "/api/repos/o/a/push?endpoint={}",
        clm_web_platform::http::percent_encode(endpoint)
    );
    let res = fake.handle(get(&path, cookie));
    assert_eq!(res.status, 200);
    body_json(&res)["subscribed"].as_bool().unwrap()
}

#[test]
fn key() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    let res = fake.handle(get("/api/push/key", &cookie));
    assert_eq!(res.status, 200);
    assert_eq!(body_json(&res), json!({"key": "BPublicKey"}));
    assert_eq!(fake.handle(get("/api/push/key", "")).status, 401);
}

#[test]
fn no_key() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let fake = Fake::default();
    let cookie = fake.cookie("u");
    assert_eq!(fake.handle(get("/api/push/key", &cookie)).status, 404);
}

#[test]
fn subscribe_once() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    assert!(!subscribed(&fake, &cookie, ENDPOINT));
    for _ in 0..2 {
        let res = fake.handle(post(
            "/api/repos/o/a/push",
            &cookie,
            &subscription(ENDPOINT, 16),
        ));
        assert_eq!(res.status, 200);
    }
    let records = records(&fake, "alice/web");
    assert_eq!(records.len(), 1, "{records:?}");
    assert_eq!(records[0]["kind"], "subscribed");
    assert_eq!(records[0]["writer"], "alice/web");
    assert_eq!(records[0]["endpoint"], ENDPOINT);
    assert_eq!(records[0]["p256dh"], p256dh());
    assert_eq!(records[0]["auth"], auth(16));
    // The writes were the first post's four; the second wrote nothing.
    assert_eq!(fake.gh.writes().len(), 4);
    // A notice's refresh reads the fold anew.
    let fresh = format!(
        "/api/repos/o/a/push?endpoint={}&fresh=d1",
        clm_web_platform::http::percent_encode(ENDPOINT)
    );
    assert_eq!(
        body_json(&fake.handle(get(&fresh, &cookie)))["subscribed"],
        true
    );
}

#[test]
fn off_right_after_on() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    let res = fake.handle(post(
        "/api/repos/o/a/push",
        &cookie,
        &subscription(ENDPOINT, 16),
    ));
    assert_eq!(res.status, 200);
    let s1 = records(&fake, "alice/web")[0]["id"].clone();
    let body = json!({"endpoint": ENDPOINT}).to_string();
    let res = fake.handle(delete("/api/repos/o/a/push", &cookie, body.as_bytes()));
    assert_eq!(res.status, 200);
    let records = records(&fake, "alice/web");
    assert_eq!(records.len(), 2);
    assert_eq!(records[1]["kind"], "unsubscribed");
    assert_eq!(records[1]["subscription"], s1);
    // Nothing left to end: no write.
    let writes = fake.gh.writes().len();
    let res = fake.handle(delete("/api/repos/o/a/push", &cookie, body.as_bytes()));
    assert_eq!(res.status, 200);
    assert_eq!(fake.gh.writes().len(), writes);
}

#[test]
fn off_when_the_ledger_cant_be_read() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    let res = fake.handle(post(
        "/api/repos/o/a/push",
        &cookie,
        &subscription(ENDPOINT, 16),
    ));
    assert_eq!(res.status, 200);
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .ref_fails
        .insert("clm/ledger/alice/web".into());
    let writes = fake.gh.writes().len();
    let body = json!({"endpoint": ENDPOINT}).to_string();
    let res = fake.handle(delete("/api/repos/o/a/push", &cookie, body.as_bytes()));
    assert_eq!(
        res.status, 502,
        "a failed read is never taken for no ledger"
    );
    assert_eq!(fake.gh.writes().len(), writes);
}

#[test]
fn not_a_subscription() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    for body in [
        subscription("https://example.com/s/1", 16),
        subscription(ENDPOINT, 15),
        subscription("http://fcm.googleapis.com/fcm/send/1", 16),
        subscription("https://x@fcm.googleapis.com/fcm/send/1", 16),
        subscription("https://fcm.googleapis.com:444/fcm/send/1", 16),
        subscription("https://evil.com?.push.apple.com/", 16),
        json!({"endpoint": ENDPOINT, "keys": {"p256dh": auth(65), "auth": auth(16)}})
            .to_string()
            .into_bytes(),
        b"not json".to_vec(),
    ] {
        let res = fake.handle(post("/api/repos/o/a/push", &cookie, &body));
        assert_eq!(res.status, 422, "{}", String::from_utf8_lossy(&body));
    }
    let res = fake.handle(delete("/api/repos/o/a/push", &cookie, b""));
    assert_eq!(res.status, 422);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn known_services() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    for endpoint in [
        "https://updates.push.services.mozilla.com/wpush/v2/x",
        "https://web.push.apple.com/QG",
        "https://wns2-par02p.notify.windows.com/w/?token=x",
    ] {
        let res = fake.handle(post(
            "/api/repos/o/a/push",
            &cookie,
            &subscription(endpoint, 16),
        ));
        assert_eq!(res.status, 200, "{endpoint}");
    }
    assert_eq!(records(&fake, "alice/web").len(), 3);
}

#[test]
fn no_access() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    let path = "/api/repos/o/private/push";
    let res = fake.handle(post(path, &cookie, &subscription(ENDPOINT, 16)));
    assert_eq!(res.status, 404);
    let res = fake.handle(get(&format!("{path}?endpoint=x"), &cookie));
    assert_eq!(res.status, 404);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn another_persons_subscription() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    fake.gh.grant("v", "o/a");
    let bob = fake.cookie_as("v", "bob");
    let res = fake.handle(post(
        "/api/repos/o/a/push",
        &bob,
        &subscription(ENDPOINT, 16),
    ));
    assert_eq!(res.status, 200);
    // Alice's delete ends nothing of Bob's, and her switch isn't on.
    let body = json!({"endpoint": ENDPOINT}).to_string();
    let res = fake.handle(delete("/api/repos/o/a/push", &cookie, body.as_bytes()));
    assert_eq!(res.status, 200);
    assert!(records(&fake, "alice/web").is_empty());
    assert!(!subscribed(&fake, &cookie, ENDPOINT));
    assert!(subscribed(&fake, &bob, ENDPOINT));
}

#[test]
fn ended_from_another_branch() {
    // req:web.worker/push-subscriptions@c5f1fe54
    let (fake, cookie) = setup();
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let s1 = json!({
            "id": "s1", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "subscribed", "endpoint": ENDPOINT, "p256dh": p256dh(), "auth": auth(16),
        });
        ledger(repo, "alice/web", "LA", &[s1]);
        let ended = json!({
            "id": "u1", "writer": "alice/web", "at": "2027-01-15T07:30:00.000Z",
            "kind": "unsubscribed", "subscription": "s1",
        });
        // And a `subscribed` record claiming alice in a branch that isn't hers.
        let forged = json!({
            "id": "s2", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
            "kind": "subscribed", "endpoint": "https://fcm.googleapis.com/fcm/send/2",
            "p256dh": p256dh(), "auth": auth(16),
        });
        ledger(repo, "mallory/web", "LM", &[ended, forged]);
    }
    assert!(subscribed(&fake, &cookie, ENDPOINT));
    assert!(!subscribed(
        &fake,
        &cookie,
        "https://fcm.googleapis.com/fcm/send/2"
    ));
}
