mod common;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use clm_web_live::{Answer, MAX_MESSAGES, Request, Vapid, fetch, web_push};
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
            upgrade: None,
            body: body.as_bytes().to_vec(),
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
    // req:web.live/push-send@849824f0
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
    // req:web.live/push-send@849824f0
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
    let sent = host.sent.borrow();
    assert_eq!(sent.len(), 2);
    assert_eq!(jwt(&sent[0].headers), jwt(&sent[1].headers));
}

#[test]
fn each_post_is_the_messages_web_push_request() {
    // req:web.live/push-send@849824f0
    // Two origins: each request is `web_push`'s for its own subscription and the payload
    // serialised as JSON, signed for its own endpoint's origin, at the host's clock and
    // with the host's randomness.
    const C: &str = "https://push.example/s/1";
    let host = Fake {
        vapid: Some(vapid()),
        ..Fake::default()
    };
    let messages = [message(A), message(C)];
    let body = json!({ "messages": messages }).to_string();
    assert_eq!(
        send(&host, &body),
        Answer::Json(200, json!({"gone": []}).to_string())
    );
    let expected: Vec<_> = messages
        .iter()
        .map(|m| {
            let subscription = serde_json::from_value(m["subscription"].clone()).unwrap();
            let payload = m["payload"].to_string();
            web_push(
                &subscription,
                payload.as_bytes(),
                &vapid(),
                1_800_000_000,
                [5; 48],
            )
            .unwrap()
        })
        .collect();
    assert_eq!(*host.sent.borrow(), expected);
    assert_ne!(
        jwt(&expected[0].headers),
        jwt(&expected[1].headers),
        "one JWT per origin"
    );
}

#[test]
fn a_bad_subscription_is_skipped() {
    // req:web.live/push-send@849824f0
    // `auth` of 3 bytes, a `p256dh` that isn't a point, an `http://` endpoint, and a payload
    // whose JSON is over 3993 bytes.
    let cases = [
        ("/subscription/auth", json!("AAAA")),
        ("/subscription/p256dh", json!(b64(&[4; 65]))),
        ("/subscription/endpoint", json!("http://push.example/a")),
        ("/payload", json!("a".repeat(4000))),
    ];
    for (case, value) in cases {
        let host = Fake {
            vapid: Some(vapid()),
            ..Fake::default()
        };
        let mut bad = message(A);
        *bad.pointer_mut(case).unwrap() = value;
        let body = json!({"messages": [message(B), bad]}).to_string();
        assert_eq!(
            send(&host, &body),
            Answer::Json(200, json!({"gone": []}).to_string()),
            "{case}"
        );
        let sent = host.sent.borrow();
        assert_eq!(
            sent.iter().map(|r| r.url.as_str()).collect::<Vec<_>>(),
            [B],
            "{case}"
        );
    }
}

#[test]
fn too_many() {
    // req:web.live/push-send@849824f0
    let host = Fake {
        vapid: Some(vapid()),
        ..Fake::default()
    };
    assert_eq!(MAX_MESSAGES, 10);
    let body = json!({"messages": vec![message(A); MAX_MESSAGES + 1]}).to_string();
    assert_eq!(send(&host, &body), Answer::Status(422));
    assert!(host.sent.borrow().is_empty());
    let most = json!({"messages": vec![message(A); MAX_MESSAGES]}).to_string();
    assert!(matches!(send(&host, &most), Answer::Json(200, _)));
    assert_eq!(host.sent.borrow().len(), MAX_MESSAGES);
}

#[test]
fn a_body_that_does_not_parse() {
    // req:web.live/push-send@849824f0
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
    // req:web.live/push-send@849824f0
    let two = json!({"messages": [message(A), message(B)]}).to_string();
    let host = Fake::default();
    assert_eq!(send(&host, &two), Answer::Status(503));
    let mut broken = vapid();
    broken.private_key = String::new();
    let mut zero = vapid();
    zero.private_key = b64(&[0; 32]);
    for vapid in [broken, zero] {
        let host = Fake {
            vapid: Some(vapid.clone()),
            ..Fake::default()
        };
        assert_eq!(send(&host, &two), Answer::Status(503), "{vapid:?}");
        assert!(host.sent.borrow().is_empty());
    }
    // Too many is refused whatever the secrets.
    let eleven = json!({"messages": vec![message(A); 11]}).to_string();
    assert_eq!(send(&host, &eleven), Answer::Status(422));
    assert!(host.sent.borrow().is_empty());
}

#[test]
fn without_randomness() {
    // req:web.live/push-send@849824f0
    let host = Fake {
        vapid: Some(vapid()),
        no_random: true,
        ..Fake::default()
    };
    let body = json!({"messages": [message(A), message(B)]}).to_string();
    assert_eq!(send(&host, &body), Answer::Status(503));
    assert!(host.sent.borrow().is_empty());
}

#[test]
fn not_gone() {
    // req:web.live/push-send@849824f0
    const C: &str = "https://fcm.googleapis.com/fcm/send/c";
    const D: &str = "https://fcm.googleapis.com/fcm/send/d";
    let host = Fake {
        vapid: Some(vapid()),
        statuses: [(B.to_owned(), 429), (C.to_owned(), 500)].into(),
        failing: [D.to_owned()].into(),
        ..Fake::default()
    };
    let body = json!({"messages": [message(A), message(B), message(C), message(D)]}).to_string();
    assert_eq!(
        send(&host, &body),
        Answer::Json(200, json!({"gone": []}).to_string())
    );
    let sent = host.sent.borrow();
    assert_eq!(
        sent.iter().map(|r| r.url.as_str()).collect::<Vec<_>>(),
        [A, B, C, D]
    );
}
