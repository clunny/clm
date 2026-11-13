//! The proposal, withdrawal, answer and decide routes pass their writes to the repo's `Replica`
//! object in one post and answer what it answers (`writes-to-object`). The object is
//! scripted here; the routes' own scenarios run against a real one.

mod common;

use clm_web_platform::http::{Request, Response};
use common::*;
use serde_json::{Value, json};

/// `o/a` and `o/private`, `alice` signed in with token `u` who can read only `o/a`, and the
/// invited list holding her.
fn setup() -> (Fake, String) {
    let fake = Fake::default();
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), br#"["alice"]"#.to_vec()),
    );
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh
        .add_repo("o/private", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    let cookie = fake.cookie_as("u", "alice");
    (fake, cookie)
}

fn proposal() -> Value {
    json!({"title": " Export CSV ", "body": "", "nodes": ["a", "a"]})
}

fn propose(fake: &Fake, cookie: &str, repo: &str, body: &Value) -> Response {
    fake.handle(post(
        &format!("/api/repos/o/{repo}/proposals"),
        cookie,
        body.to_string().as_bytes(),
    ))
}

fn decide(fake: &Fake, cookie: &str, path: &str, body: &Value) -> Response {
    fake.handle(post(
        &format!("/api/repos/o/a/{path}/decide"),
        cookie,
        body.to_string().as_bytes(),
    ))
}

fn withdraw(fake: &Fake, cookie: &str, id: &str) -> Response {
    let mut req: Request = post(&format!("/api/repos/o/a/proposals/{id}"), cookie, b"");
    req.method = "DELETE".into();
    fake.handle(req)
}

/// Beyond the access check, GitHub was asked nothing since `from` calls.
fn only_the_access_check(fake: &Fake, from: usize) {
    let calls = fake.gh.calls.borrow();
    assert!(
        calls[from..]
            .iter()
            .all(|c| c.url.contains("/collaborators/") || c.url.ends_with("/repos/o/a")),
        "{:?}",
        &calls[from..]
    );
}

/// The one post the object was asked.
fn one_post(fake: &Fake) -> Value {
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1, "{asked:?}");
    assert_eq!(asked[0].method, "POST");
    assert_eq!(asked[0].path, "/write");
    asked[0].body.clone().unwrap()
}

#[test]
fn a_proposal_passed_on() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(409, json!({"error": "taken"}));
    let before = fake.gh.calls.borrow().len();
    let res = propose(&fake, &cookie, "a", &proposal());
    assert_eq!(res.status, 409);
    assert_eq!(body_json(&res), json!({"error": "taken"}));
    let post = one_post(&fake);
    assert_eq!(post["login"], "alice");
    assert_eq!(post["token"], "u");
    assert_eq!(post["kind"], "propose");
    assert_eq!(post["title"], "Export CSV");
    assert_eq!(post["body"], "");
    assert_eq!(post["nodes"], json!(["a"]));
    only_the_access_check(&fake, before);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn a_decision_passed_on() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(200, json!({"id": "d1"}));
    let body = json!({"version": "0123abcd", "value": "low", "decision": "cut"});
    let before = fake.gh.calls.borrow().len();
    let res = decide(&fake, &cookie, "architecture/my%20e", &body);
    only_the_access_check(&fake, before);
    assert_eq!(res.status, 200);
    assert_eq!(body_json(&res), json!({"id": "d1"}), "the body as given");
    let post = one_post(&fake);
    assert_eq!(post["kind"], "decide");
    assert_eq!(post["id"], "architecture/my e");
    assert_eq!(post["version"], "0123abcd");
    assert_eq!(post["value"], "low");
    assert_eq!(post["decision"], "cut");
    assert_eq!(post["login"], "alice");
    assert_eq!(post["token"], "u");
}

#[test]
fn a_capability_decision_names_its_layer() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(200, json!({}));
    let body = json!({"version": "0123abcd", "value": "high", "decision": "keep"});
    assert_eq!(decide(&fake, &cookie, "capabilities/x", &body).status, 200);
    assert_eq!(one_post(&fake)["id"], "capability/x");
}

#[test]
fn a_withdrawal_passed_on() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    *fake.replicas.write.borrow_mut() = Some((404, String::new()));
    let before = fake.gh.calls.borrow().len();
    let res = withdraw(&fake, &cookie, "p1");
    only_the_access_check(&fake, before);
    assert_eq!(res.status, 404);
    let post = one_post(&fake);
    assert_eq!(post["kind"], "withdraw");
    assert_eq!(post["id"], "p1");
}

#[test]
fn an_answer_passed_on() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(404, json!({"error": "unknown"}));
    let before = fake.gh.calls.borrow().len();
    let res = fake.handle(post(
        "/api/repos/o/a/backlog/my%20q/answer",
        &cookie,
        json!({"answer": "yes"}).to_string().as_bytes(),
    ));
    only_the_access_check(&fake, before);
    assert_eq!(res.status, 404);
    assert_eq!(body_json(&res), json!({"error": "unknown"}));
    let post = one_post(&fake);
    assert_eq!(post["kind"], "answer");
    assert_eq!(post["item"], "my q");
    assert_eq!(post["answer"], "yes");
    assert_eq!(post["login"], "alice");
    assert_eq!(post["token"], "u");
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn preparing() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(503, json!({"preparing": 0.4}));
    let res = propose(&fake, &cookie, "a", &proposal());
    assert_eq!(res.status, 503);
    assert_eq!(body_json(&res), json!({"preparing": 0.4}));
}

#[test]
fn object_down() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.replicas.bound.set(false);
    let valid = json!({"version": "0123abcd", "value": "low", "decision": "cut"});
    for res in [
        propose(&fake, &cookie, "a", &proposal()),
        decide(&fake, &cookie, "capabilities/x", &valid),
        withdraw(&fake, &cookie, "p1"),
        fake.handle(post(
            "/api/repos/o/a/backlog/q/answer",
            &cookie,
            br#"{"answer": "yes"}"#,
        )),
    ] {
        assert_eq!(res.status, 503);
        assert_eq!(body_json(&res), json!({"error": "unavailable"}));
    }
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn no_access() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(200, json!({}));
    let res = propose(&fake, &cookie, "private", &proposal());
    assert_eq!(res.status, 404);
    assert!(
        fake.replicas.calls.borrow().is_empty(),
        "the object is asked nothing"
    );
}

#[test]
fn a_failed_body_check_posts_nothing() {
    // req:web.worker/writes-to-object@a68570e6
    let (fake, cookie) = setup();
    fake.set_write(200, json!({}));
    let res = propose(
        &fake,
        &cookie,
        "a",
        &json!({"title": "", "body": "", "nodes": []}),
    );
    assert_eq!(res.status, 422);
    let res = decide(&fake, &cookie, "capabilities/x", &json!({}));
    assert_eq!(res.status, 422);
    assert!(fake.replicas.calls.borrow().is_empty());
}
