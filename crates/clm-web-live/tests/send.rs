mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web_live::{Answer, Request, Vapid, fetch};
use common::{Fake, block_on};
use p256::SecretKey;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use serde_json::{Value, json};

fn b64(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

fn vapid() -> Vapid {
    let secret = SecretKey::from_slice(&[3; 32]).unwrap();
    Vapid {
        public_key: b64(secret.public_key().to_encoded_point(false).as_bytes()),
        private_key: b64(&secret.to_bytes()),
    }
}

fn message(endpoint: &str) -> Value {
    let browser = SecretKey::from_slice(&[7; 32]).unwrap();
    json!({
        "subscription": {
            "endpoint": endpoint,
            "p256dh": b64(browser.public_key().to_encoded_point(false).as_bytes()),
            "auth": b64(&[9; 16]),
        },
        "payload": {"title": "t", "body": "b", "url": "/#/o/a/backlog"},
    })
}

fn send(host: &Fake, body: &str) -> Answer<usize> {
    block_on(fetch(
        host,
        Request {
            method: "POST".into(),
            path: "/send".into(),
            websocket: false,
            body: body.into(),
        },
    ))
}

const A: &str = "https://fcm.googleapis.com/fcm/send/a";
const B: &str = "https://fcm.googleapis.com/fcm/send/b";

fn jwt(headers: &[(String, String)]) -> String {
    let auth = &headers
        .iter()
        .find(|(n, _)| n == "Authorization")
        .unwrap()
        .1;
    let t = auth.strip_prefix("vapid t=").unwrap();
    t.split_once(", k=").unwrap().0.to_owned()
}

#[test]
fn one_gone() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        statuses: [(B.to_owned(), 410)].into(),
        ..Fake::default()
    };
    let body = json!({"messages": [message(A), message(B)]}).to_string();
    let answer = send(&host, &body);
    assert_eq!(answer, Answer::Json(200, json!({"gone": [B]}).to_string()));
    let sent = host.sent.borrow();
    assert_eq!(
        sent.iter().map(|r| r.url.as_str()).collect::<Vec<_>>(),
        [A, B]
    );
    assert_eq!(jwt(&sent[0].headers), jwt(&sent[1].headers));
}

#[test]
fn a_404_is_gone_too() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        statuses: [(A.to_owned(), 404), (B.to_owned(), 500)].into(),
        ..Fake::default()
    };
    let body = json!({"messages": [message(A), message(B)]}).to_string();
    assert_eq!(
        send(&host, &body),
        Answer::Json(200, json!({"gone": [A]}).to_string())
    );
}

#[test]
fn a_bad_subscription_is_skipped() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        ..Fake::default()
    };
    let mut bad = message(A);
    bad["subscription"]["auth"] = "AAAA".into();
    let body = json!({"messages": [bad, message(B)]}).to_string();
    assert_eq!(
        send(&host, &body),
        Answer::Json(200, json!({"gone": []}).to_string())
    );
    assert_eq!(host.sent.borrow().len(), 1);
    assert_eq!(host.sent.borrow()[0].url, B);
}

#[test]
fn too_many() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        ..Fake::default()
    };
    let body = json!({"messages": vec![message(A); 11]}).to_string();
    assert_eq!(send(&host, &body), Answer::Status(422));
    assert!(host.sent.borrow().is_empty());
    let ten = json!({"messages": vec![message(A); 10]}).to_string();
    assert!(matches!(send(&host, &ten), Answer::Json(200, _)));
    assert_eq!(host.sent.borrow().len(), 10);
}

#[test]
fn a_body_that_does_not_parse() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        ..Fake::default()
    };
    for body in [
        "",
        "{",
        r#"{"messages": 1}"#,
        r#"{"messages": [{"payload": 1}]}"#,
    ] {
        assert_eq!(send(&host, body), Answer::Status(422), "{body}");
    }
    assert!(host.sent.borrow().is_empty());
}

#[test]
fn without_vapid_secrets() {
    // req:web.live/push-send@50c3e1cb
    let body = json!({"messages": [message(A)]}).to_string();
    let host = Fake::default();
    assert_eq!(send(&host, &body), Answer::Status(503));
    let mut broken = vapid();
    broken.private_key = String::new();
    let host = Fake {
        vapid: Some(broken),
        ..Fake::default()
    };
    assert_eq!(send(&host, &body), Answer::Status(503));
    assert!(host.sent.borrow().is_empty());
}

#[test]
fn without_randomness() {
    // req:web.live/push-send@50c3e1cb
    let host = Fake {
        vapid: Some(vapid()),
        no_random: true,
        ..Fake::default()
    };
    let body = json!({"messages": [message(A), message(B)]}).to_string();
    assert_eq!(send(&host, &body), Answer::Status(503));
    assert!(host.sent.borrow().is_empty());
}
