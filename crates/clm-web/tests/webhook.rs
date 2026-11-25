//! The webhook: verified, then each event goes to the `Replica` object of the repo it names.
//! The Worker computes no view and stores nothing.

mod common;

use clm_web_platform::http::Request;
use common::*;
use serde_json::json;

fn with_delivery(mut req: Request, delivery: &str) -> Request {
    req.headers
        .insert("x-github-delivery".into(), delivery.into());
    req
}

fn setup() -> Fake {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake
}

#[test]
fn push_to_main() {
    // req:web.worker/webhook-refresh@656e517a
    let fake = setup();
    let calls = fake.gh.calls.borrow().len();
    let res = fake.handle(with_delivery(
        webhook("push", &push_body("o/a", "111", "222"), WEBHOOK_SECRET),
        "d1",
    ));
    assert_eq!(fake.gh.calls.borrow().len(), calls, "no view computed");
    assert_eq!(res.status, 200, "{res:?}");
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1);
    assert_eq!(
        (asked[0].method.as_str(), asked[0].path.as_str()),
        ("POST", "/event")
    );
    let sent = asked[0].body.as_ref().unwrap();
    assert_eq!(sent["event"], "push");
    assert_eq!(sent["delivery"], "d1");
    assert_eq!(sent["installation"], 9);
    assert_eq!(sent["body"]["after"], "222");
    assert!(fake.stored.borrow().is_empty(), "nothing stored");
}

#[test]
fn every_event_that_names_repos_is_passed_on() {
    // req:web.worker/webhook-refresh@656e517a
    let fake = setup();
    let repo = |name: &str| json!({"full_name": name});
    let events = [
        (
            "pull_request",
            json!({"action": "opened", "repository": repo("o/a"), "installation": {"id": 9}}),
            vec!["o/a"],
        ),
        (
            "installation",
            json!({"action": "deleted", "repositories": [repo("o/a"), repo("o/b")], "installation": {"id": 9}}),
            vec!["o/a", "o/b"],
        ),
        (
            "installation_repositories",
            json!({
                "action": "removed", "repositories_added": [repo("o/c")],
                "repositories_removed": [repo("o/a"), repo("../x")], "installation": {"id": 9},
            }),
            vec!["o/a", "o/c"],
        ),
    ];
    for (event, body, names) in events {
        fake.replicas.calls.borrow_mut().clear();
        let res = fake.handle(with_delivery(webhook(event, &body, WEBHOOK_SECRET), "d2"));
        assert_eq!(res.status, 200, "{event}: {res:?}");
        let mut got: Vec<String> = fake
            .replicas
            .calls
            .borrow()
            .iter()
            .map(|c| c.name.clone())
            .collect();
        got.sort();
        assert_eq!(got, names, "{event}");
        let calls = fake.replicas.calls.borrow();
        assert!(
            calls
                .iter()
                .all(|c| c.body.as_ref().unwrap()["event"] == event)
        );
        assert!(
            calls
                .iter()
                .all(|c| c.body.as_ref().unwrap()["delivery"] == "d2")
        );
    }
    assert!(fake.stored.borrow().is_empty());
}

#[test]
fn a_failed_post_does_not_change_the_answer() {
    // req:web.worker/webhook-refresh@656e517a
    let fake = setup();
    fake.replicas.bound.set(false);
    let res = fake.handle(webhook(
        "push",
        &push_body("o/a", "111", "222"),
        WEBHOOK_SECRET,
    ));
    assert_eq!(res.status, 200);
    assert!(!fake.logs.borrow().is_empty(), "logged");
}

#[test]
fn forged_webhook() {
    // req:web.worker/webhook-refresh@656e517a
    let fake = setup();
    let calls = fake.gh.calls.borrow().len();
    let res = fake.handle(webhook("push", &push_body("o/a", "111", "222"), "wrong"));
    assert_eq!(res.status, 401);
    assert!(fake.replicas.calls.borrow().is_empty());
    assert!(fake.stored.borrow().is_empty());
    assert_eq!(fake.gh.calls.borrow().len(), calls);

    // A missing signature is no better.
    let mut req = webhook("push", &push_body("o/a", "111", "222"), WEBHOOK_SECRET);
    req.headers.remove("x-hub-signature-256");
    assert_eq!(fake.handle(req).status, 401);
    assert!(fake.replicas.calls.borrow().is_empty());
}

#[test]
fn other_events_reach_no_object() {
    // req:web.worker/webhook-refresh@656e517a
    let fake = setup();
    let res = fake.handle(webhook("ping", &json!({"zen": "z"}), WEBHOOK_SECRET));
    assert_eq!(res.status, 204);
    assert!(fake.replicas.calls.borrow().is_empty());
}

/// A webhook signed with `secret` whose body is `bytes`, JSON or not.
fn raw_webhook(event: &str, bytes: &[u8], secret: &str) -> Request {
    let mut req = webhook(event, &json!({}), secret);
    req.body = bytes.to_vec();
    req.headers
        .insert("x-hub-signature-256".into(), signature(secret, bytes));
    req
}

/// The objects the Worker posted to, in order.
fn reached(fake: &Fake) -> Vec<String> {
    let calls = fake.replicas.calls.borrow();
    calls.iter().map(|c| c.name.clone()).collect()
}

#[test]
fn what_the_webhook_answers() {
    // req:web.worker/webhook-refresh@656e517a
    let mut fake = setup();
    let repo = |name: &str| json!({"full_name": name});

    let ping = fake.handle(raw_webhook("ping", b"not json", WEBHOOK_SECRET));
    assert_eq!(ping.status, 204, "a ping");
    assert!(reached(&fake).is_empty(), "the ping reached no object");

    fake.replicas.bound.set(false);
    let push = fake.handle(webhook(
        "push",
        &push_body("o/a", "111", "222"),
        WEBHOOK_SECRET,
    ));
    assert_eq!(push.status, 200, "a push the object fails");
    fake.replicas.bound.set(true);

    let added = json!({
        "action": "added",
        "repositories_added": [repo("o/b"), repo("o/b"), repo("../x")],
        "repositories_removed": [],
        "installation": {"id": 9},
    });
    let res = fake.handle(webhook("installation_repositories", &added, WEBHOOK_SECRET));
    assert_eq!(res.status, 200);
    assert_eq!(reached(&fake), ["o/b"], "o/b's object once, and no other");

    for event in ["pull_request", "installation"] {
        let res = fake.handle(raw_webhook(event, b"not json", WEBHOOK_SECRET));
        assert_eq!(res.status, 400, "{event} with a body that isn't JSON");
    }

    fake.secrets.remove("GITHUB_WEBHOOK_SECRET");
    let unset = fake.handle(webhook(
        "push",
        &push_body("o/a", "111", "222"),
        WEBHOOK_SECRET,
    ));
    assert_eq!(unset.status, 401, "no GITHUB_WEBHOOK_SECRET");

    assert_eq!(
        reached(&fake),
        ["o/b"],
        "nothing reached an object but the installation_repositories event"
    );
}
