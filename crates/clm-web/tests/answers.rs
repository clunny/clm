//! An answer to a backlog `question` from the web app (`web-answers-as-ledger-records`): an
//! `answered` record in the requester's web ledger, through the repo's `Replica` object
//! (`writes-to-object`), and the open answers the implementation response lists.

mod common;

use clm_web_platform::http::Response;
use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

const GIT: &str = "https://api.github.com/repos/o/a/git";

/// `o/a` with a `question` item `q` offering `yes` and `no`, and alice signed in. With
/// `real`, a real `Replica` object is built from it; without, the views are scripted.
fn setup_as(real: bool) -> (Fake, String) {
    let mut files = clm_repo();
    files.insert(
        "backlog/q.md".into(),
        b"---\ntitle: \"Q\"\nkind: question\ncomponents: []\nchoices: [yes, no]\n---\n\nWhich?\n"
            .to_vec(),
    );
    let fake = Fake::default();
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), br#"["alice", "bob"]"#.to_vec()),
    );
    fake.gh.add_repo("o/a", FakeRepo::new("111", files));
    fake.gh.grant("u", "o/a");
    if real {
        fake.real_object("o/a");
    } else {
        fake.set_view("o/a", "implementation", json!({"sha": "111"}));
    }
    let cookie = fake.cookie_as("u", "alice");
    (fake, cookie)
}

fn answer(fake: &Fake, cookie: &str, item: &str, body: &Value) -> Response {
    fake.handle(post(
        &format!("/api/repos/o/a/backlog/{item}/answer"),
        cookie,
        body.to_string().as_bytes(),
    ))
}

/// The records of `o/a`'s `clm/ledger/<writer>`, at its tip.
fn records(fake: &Fake, writer: &str) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let Some(tip) = repo.branches.get(&format!("clm/ledger/{writer}")) else {
        return Vec::new();
    };
    std::str::from_utf8(&repo.commits[tip]["records.jsonl"])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

#[test]
fn an_answer_is_appended() {
    // req:web.worker/answers@7d19919b
    let (fake, cookie) = setup_as(true);
    let res = answer(&fake, &cookie, "q", &json!({"answer": "yes"}));
    assert_eq!(res.status, 200, "{res:?}");
    let id = body_json(&res)["id"].clone();
    let writes: Vec<(String, String)> = fake
        .gh
        .writes()
        .iter()
        .map(|w| (w.method.to_string(), w.url.clone()))
        .collect();
    assert_eq!(
        writes,
        [
            ("POST".to_string(), format!("{GIT}/blobs")),
            ("POST".into(), format!("{GIT}/trees")),
            ("POST".into(), format!("{GIT}/commits")),
            ("POST".into(), format!("{GIT}/refs")),
        ]
    );
    assert!(
        fake.gh
            .writes()
            .iter()
            .all(|w| w.auth == Auth::User("u".into()))
    );
    let records = records(&fake, "alice/web");
    let last = records.last().unwrap();
    assert_eq!(last["id"], id);
    assert_eq!(last["kind"], "answered");
    assert_eq!(last["writer"], "alice/web");
    assert_eq!(last["item"], "q");
    assert_eq!(last["answer"], "yes");
}

#[test]
fn refused() {
    // req:web.worker/answers@7d19919b
    let (fake, cookie) = setup_as(true);
    let res = answer(&fake, &cookie, "q", &json!({"answer": "maybe"}));
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "answer"}));
    let res = answer(&fake, &cookie, "nope", &json!({"answer": "yes"}));
    assert_eq!(res.status, 404);
    assert_eq!(body_json(&res), json!({"error": "unknown"}));
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn a_body_that_is_no_answer_is_refused_before_asking_anyone() {
    // req:web.worker/answers@7d19919b
    let (fake, cookie) = setup_as(true);
    for body in [json!({}), json!({"answer": 1}), json!({"answer": null})] {
        let (calls, asked) = (
            fake.gh.calls.borrow().len(),
            fake.replicas.asked("o/a").len(),
        );
        let res = answer(&fake, &cookie, "q", &body);
        assert_eq!(res.status, 422, "{body}");
        assert_eq!(body_json(&res), json!({"error": "answer"}), "{body}");
        assert_eq!(fake.gh.calls.borrow().len(), calls, "{body}: asked GitHub");
        assert_eq!(
            fake.replicas.asked("o/a").len(),
            asked,
            "{body}: asked the object"
        );
    }
    let res = fake.handle(post(
        "/api/repos/o/a/backlog/q/answer",
        &cookie,
        b"not json",
    ));
    assert_eq!(res.status, 422);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn the_item_is_percent_decoded() {
    // req:web.worker/answers@7d19919b
    let (fake, cookie) = setup_as(false);
    fake.set_write(200, json!({"id": "a1"}));
    let res = answer(&fake, &cookie, "my%20item", &json!({"answer": "yes"}));
    assert_eq!(res.status, 200);
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1);
    let post = asked[0].body.clone().unwrap();
    assert_eq!(post["kind"], "answer");
    assert_eq!(post["item"], "my item");
    assert_eq!(post["answer"], "yes");
}

#[test]
fn open_answers() {
    // req:web.worker/answers@7d19919b
    let (fake, cookie) = setup_as(false);
    let listed = [
        json!({"id": "a1", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z", "item": "q", "answer": "yes"}),
        json!({"id": "b1", "writer": "bob/web", "at": "2027-01-15T07:10:00.000Z", "item": "q", "answer": "no"}),
    ];
    fake.set_view(
        "o/a",
        "implementation",
        json!({"sha": "111", "answers": listed}),
    );
    let view = body_json(&fake.handle(get("/api/repos/o/a/implementation", &cookie)));
    let got: Vec<(&str, bool)> = view["answers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["id"].as_str().unwrap(), p["mine"].as_bool().unwrap()))
        .collect();
    assert_eq!(got, [("a1", true), ("b1", false)], "the view's order");
    for (i, mine) in [true, false].into_iter().enumerate() {
        let mut expected = listed[i].clone();
        expected["mine"] = json!(mine);
        assert_eq!(view["answers"][i], expected, "the other fields as given");
    }
}
