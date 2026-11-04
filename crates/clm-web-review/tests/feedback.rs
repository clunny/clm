//! Comments and change requests on a spec branch's diff
//! (`web-review-feedback-as-ledger-records`): `commented` and `changes_requested` records
//! in the requester's web ledger, and the review's `comments` and `change_requests`.

mod common;

use clm_web_platform::platform::Auth;
use common::*;
use serde_json::{Value, json};

const GIT: &str = "https://api.github.com/repos/o/a/git";

/// `o/a` whose `main` (`111`) has node `m` with requirement `old`, and `spec/x`, which
/// adds node `n` with requirement `r`, removes `m/old` and adds change `x`. The signed-in
/// person is `clunny`, whose permission on the repo is `permission`.
fn setup(permission: &str) -> (Fake, String) {
    let mut base = clm_repo();
    base.insert(
        "specs/m/SPEC.md".into(),
        node("", &["old"], "active").into_bytes(),
    );
    let mut head = base.clone();
    head.insert(
        "specs/m/SPEC.md".into(),
        node("", &[], "active").into_bytes(),
    );
    head.insert(
        "specs/n/SPEC.md".into(),
        node("", &["r"], "active").into_bytes(),
    );
    head.insert(
        "changes/x/change.md".into(),
        b"---\nstatus: proposed\n---\n\n## Intent\nAdd n.\n".to_vec(),
    );
    let mut repo = FakeRepo::new("111", base);
    repo.commits.insert("222".into(), head);
    repo.branches.insert("spec/x".into(), "222".into());
    repo.permissions.insert("clunny".into(), permission.into());
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

fn fingerprint(fake: &Fake) -> String {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    fingerprint_of(
        &repo.commits["111"],
        &repo.commits[&repo.branches["spec/x"]],
    )
}

fn send(fake: &Fake, cookie: &str, route: &str, body: &Value) -> clm_web_platform::http::Response {
    fake.handle(post(
        &format!("/api/repos/o/a/branches/spec/x/{route}"),
        cookie,
        body.to_string().as_bytes(),
    ))
}

fn review(fake: &Fake, cookie: &str) -> Value {
    let res = fake.handle(get("/api/repos/o/a/branches/spec/x/review", cookie));
    assert_eq!(res.status, 200);
    body_json(&res)
}

/// The records on `clm/ledger/clunny/web`, or none.
fn records(fake: &Fake) -> Vec<Value> {
    let repos = fake.gh.repos.borrow();
    let repo = &repos["o/a"];
    let Some(tip) = repo.branches.get("clm/ledger/clunny/web") else {
        return Vec::new();
    };
    std::str::from_utf8(&repo.commits[tip]["records.jsonl"])
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn writes(fake: &Fake) -> Vec<(String, String)> {
    fake.gh
        .writes()
        .iter()
        .map(|w| (w.method.to_string(), w.url.clone()))
        .collect()
}

#[test]
fn comment_on_a_requirement() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("read");
    let f = fingerprint(&fake);
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": f, "target": "n/r", "text": " Name the rule "}),
    );
    assert_eq!(res.status, 200, "{res:?}");
    let id = body_json(&res)["id"].clone();
    assert_eq!(
        writes(&fake),
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
    let last = records(&fake).pop().unwrap();
    assert_eq!(last["kind"], "commented");
    assert_eq!(last["id"], id);
    assert_eq!(last["branch"], "spec/x");
    assert_eq!(last["fingerprint"], f.as_str());
    assert_eq!(last["target"], "n/r");
    assert_eq!(last["text"], "Name the rule");
    let comments = &review(&fake, &cookie)["comments"];
    assert_eq!(comments.as_array().unwrap().len(), 1, "{comments}");
    assert_eq!(comments[0]["id"], id);
    assert_eq!(comments[0]["writer"], "clunny/web");
    assert_eq!(comments[0]["target"], "n/r");
    assert_eq!(comments[0]["text"], "Name the rule");
    assert_eq!(comments[0]["fingerprint"], f.as_str());
    assert_eq!(comments[0]["current"], true);
}

#[test]
fn unknown_target() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("read");
    let f = fingerprint(&fake);
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": f, "target": "n/missing", "text": "Hm"}),
    );
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "target"}));
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": f, "target": null, "text": "  "}),
    );
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "text"}));
    assert!(fake.gh.writes().is_empty());
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": f, "target": "m/old", "text": "Keep it"}),
    );
    assert_eq!(res.status, 200, "a removed requirement the review lists");
    assert_eq!(records(&fake).pop().unwrap()["target"], "m/old");
    // A requirement the snapshot's graph holds, and the change as a whole.
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": "older", "target": "a/p", "text": "And this"}),
    );
    assert_eq!(res.status, 200, "no fingerprint comparison for a comment");
    let res = send(
        &fake,
        &cookie,
        "comments",
        &json!({"fingerprint": f, "target": null, "text": "Overall fine"}),
    );
    assert_eq!(res.status, 200);
    let comments = &review(&fake, &cookie)["comments"];
    let current: Vec<bool> = comments
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["current"].as_bool().unwrap())
        .collect();
    assert_eq!(current, [true, false, true], "oldest first");
    assert_eq!(comments[2]["target"], Value::Null);
}

#[test]
fn refused_bodies_and_branches() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("write");
    for (body, field) in [
        (json!({"target": null, "text": "x"}), "fingerprint"),
        (
            json!({"fingerprint": "", "target": null, "text": "x"}),
            "fingerprint",
        ),
        (
            json!({"fingerprint": "f", "target": 3, "text": "x"}),
            "target",
        ),
        (json!({"fingerprint": "f", "target": null}), "text"),
        (
            json!({"fingerprint": "f", "target": null, "text": "x".repeat(4_001)}),
            "text",
        ),
    ] {
        let calls = fake.gh.calls.borrow().len();
        let res = send(&fake, &cookie, "comments", &body);
        assert_eq!(res.status, 422, "{body}");
        assert_eq!(body_json(&res), json!({"error": field}), "{body}");
        assert_eq!(fake.gh.calls.borrow().len(), calls, "{body}: asked GitHub");
    }
    for body in [
        json!({"text": "x"}),
        json!({"fingerprint": "f", "text": ""}),
    ] {
        let res = send(&fake, &cookie, "request-changes", &body);
        assert_eq!(res.status, 422, "{body}");
    }
    for branch in ["impl/a", "spec/nope"] {
        for route in ["comments", "request-changes"] {
            let body = json!({"fingerprint": "f", "target": null, "text": "x"}).to_string();
            let res = fake.handle(post(
                &format!("/api/repos/o/a/branches/{branch}/{route}"),
                &cookie,
                body.as_bytes(),
            ));
            assert_eq!(res.status, 422, "{branch} {route}");
            assert_eq!(body_json(&res), json!({"error": "branch"}));
        }
    }
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn changes_requested() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("write");
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let main = repos
            .get_mut("o/a")
            .unwrap()
            .commits
            .get_mut("111")
            .unwrap();
        main.insert(
            "docs/allowed_signers".into(),
            b"x ssh-ed25519 AAAA\n".to_vec(),
        );
    }
    let f = fingerprint(&fake);
    let res = send(
        &fake,
        &cookie,
        "request-changes",
        &json!({"fingerprint": f, "text": "Split it"}),
    );
    assert_eq!(res.status, 200, "a signed repository accepts it");
    let last = records(&fake).pop().unwrap();
    assert_eq!(last["kind"], "changes_requested");
    assert_eq!(last["branch"], "spec/x");
    assert_eq!(last["fingerprint"], f.as_str());
    assert_eq!(last["text"], "Split it");
    let review = review(&fake, &cookie);
    let requests = &review["change_requests"];
    assert_eq!(requests.as_array().unwrap().len(), 1);
    assert_eq!(requests[0]["id"], last["id"]);
    assert_eq!(requests[0]["current"], true);
    assert_eq!(requests[0]["held"], true);
    assert_eq!(review["comments"], json!([]));
}

#[test]
fn admins_can_hold_a_land() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("admin");
    let f = fingerprint(&fake);
    let res = send(
        &fake,
        &cookie,
        "request-changes",
        &json!({"fingerprint": f, "text": "Split it"}),
    );
    assert_eq!(res.status, 200);
}

#[test]
fn a_reader_cant_hold_a_land() {
    // req:web.worker.review/review-feedback@3d004f4e
    for permission in ["read", "none"] {
        let (fake, cookie) = setup(permission);
        let f = fingerprint(&fake);
        let res = send(
            &fake,
            &cookie,
            "request-changes",
            &json!({"fingerprint": f, "text": "Split it"}),
        );
        assert_eq!(res.status, 403, "{permission}");
        assert_eq!(body_json(&res), json!({"error": "permission"}));
        // The permission is checked before the fingerprint.
        let res = send(
            &fake,
            &cookie,
            "request-changes",
            &json!({"fingerprint": "older", "text": "Split it"}),
        );
        assert_eq!(res.status, 403, "{permission}: before 409");
        assert!(fake.gh.writes().is_empty());
    }
}

#[test]
fn moved() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("write");
    let res = send(
        &fake,
        &cookie,
        "request-changes",
        &json!({"fingerprint": "f", "text": "Split it"}),
    );
    assert_eq!(res.status, 409);
    assert_eq!(body_json(&res), json!({"error": "moved"}));
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn truncated_listing() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("write");
    let f = fingerprint(&fake);
    let at = "2027-01-15T07:00:00.000Z";
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.truncated_trees.insert("222".into());
        let records = [
            json!({"id": "c1", "writer": "alice/web", "at": at, "kind": "commented",
                   "branch": "spec/x", "fingerprint": f, "target": null, "text": "Hm"}),
            json!({"id": "r1", "writer": "alice/web", "at": at, "kind": "changes_requested",
                   "branch": "spec/x", "fingerprint": f, "text": "Split"}),
        ];
        ledger(repo, "alice/web", "LA", &records);
    }
    let res = send(
        &fake,
        &cookie,
        "request-changes",
        &json!({"fingerprint": f, "text": "Split it"}),
    );
    assert_eq!(res.status, 422);
    assert_eq!(body_json(&res), json!({"error": "fingerprint"}));
    let review = review(&fake, &cookie);
    assert_eq!(review["fingerprint"], Value::Null);
    assert_eq!(review["comments"][0]["current"], false);
    assert_eq!(review["change_requests"][0]["current"], false);
    assert_eq!(review["change_requests"][0]["held"], false);
}

#[test]
fn cleared() {
    // req:web.worker.review/review-feedback@3d004f4e
    let (fake, cookie) = setup("write");
    let f = fingerprint(&fake);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let request = |id: &str, writer: &str, at: &str| {
            json!({"id": id, "writer": writer, "at": at, "kind": "changes_requested",
                   "branch": "spec/x", "fingerprint": f, "text": "Split it"})
        };
        let records = [
            request("r1", "alice/web", "2027-01-15T07:00:00.000Z"),
            json!({"id": "a1", "writer": "alice/1a2b3c4d", "at": "2027-01-15T07:10:00.000Z",
                   "kind": "approved", "fingerprint": f, "branch": "spec/x", "change": "x"}),
            request("r2", "bob/web", "2027-01-15T07:05:00.000Z"),
            // An approval before the request doesn't clear it.
            request("r3", "carol/web", "2027-01-15T07:20:00.000Z"),
            json!({"id": "a2", "writer": "carol/9f", "at": "2027-01-15T07:15:00.000Z",
                   "kind": "approved", "fingerprint": f, "branch": "spec/x", "change": "x"}),
        ];
        ledger(repo, "alice/web", "LA", &records);
    }
    let requests = review(&fake, &cookie)["change_requests"].clone();
    let held: Vec<(&str, bool, bool)> = requests
        .as_array()
        .unwrap()
        .iter()
        .map(|r| {
            (
                r["id"].as_str().unwrap(),
                r["current"].as_bool().unwrap(),
                r["held"].as_bool().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        held,
        [("r1", true, false), ("r2", true, true), ("r3", true, true)]
    );
}
