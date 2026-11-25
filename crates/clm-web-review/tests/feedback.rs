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
    // req:web.worker.review/review-feedback@6f6d24a4
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
    // req:web.worker.review/review-feedback@6f6d24a4
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

/// The 422 body a parser refuses `body` with, or none when it accepts it.
fn refusal<T>(parsed: Result<T, clm_web_platform::http::Response>) -> Option<Value> {
    let res = parsed.err()?;
    assert_eq!(res.status, 422);
    Some(body_json(&res))
}

#[test]
fn refused_bodies_and_branches() {
    // req:web.worker.review/review-feedback@6f6d24a4
    // req:web.worker.review/request-changes@e3dfe01b
    use clm_web_review::feedback::{parse_comment, parse_request};
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
        let parsed = parse_comment(body.to_string().as_bytes());
        assert_eq!(refusal(parsed), Some(json!({"error": field})), "{body}");
    }
    for (body, field) in [
        (json!({"text": "x"}), "fingerprint"),
        (json!({"fingerprint": "f", "text": ""}), "text"),
    ] {
        let parsed = parse_request(body.to_string().as_bytes());
        assert_eq!(refusal(parsed), Some(json!({"error": field})), "{body}");
    }
    let (fake, cookie) = setup("write");
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
    // req:web.worker.review/request-changes@e3dfe01b
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
    assert_eq!(body_json(&res), json!({"id": last["id"]}));
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
    // req:web.worker.review/request-changes@e3dfe01b
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
    // req:web.worker.review/request-changes@e3dfe01b
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
    // req:web.worker.review/request-changes@e3dfe01b
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
    // req:web.worker.review/review-feedback@6f6d24a4
    // req:web.worker.review/request-changes@e3dfe01b
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
    // req:web.worker.review/review-feedback@6f6d24a4
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
            // An approval that names the request clears it, whatever either clock said.
            json!({"id": "a1", "writer": "alice/1a2b3c4d", "at": "2027-01-15T06:50:00.000Z",
                   "kind": "approved", "fingerprint": f, "branch": "spec/x", "change": "x",
                   "clears": ["r1"]}),
            request("r2", "bob/web", "2027-01-15T07:05:00.000Z"),
            // A later approval by the requester's person that doesn't name the request
            // leaves it held.
            request("r3", "carol/web", "2027-01-15T07:20:00.000Z"),
            json!({"id": "a2", "writer": "carol/9f", "at": "2027-01-15T07:25:00.000Z",
                   "kind": "approved", "fingerprint": f, "branch": "spec/x", "change": "x",
                   "clears": []}),
            // An approval written before approvals named their requests clears by time.
            request("r4", "erin/web", "2027-01-15T07:30:00.000Z"),
            json!({"id": "a3", "writer": "erin/7e", "at": "2027-01-15T07:40:00.000Z",
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
        [
            ("r1", true, false),
            ("r2", true, true),
            ("r3", true, true),
            ("r4", true, false)
        ]
    );
}

/// `docs/allowed_signers` added to `main`'s tip.
fn sign_main(repo: &mut FakeRepo) {
    let main = repo.commits.get_mut("111").unwrap();
    main.insert(
        "docs/allowed_signers".into(),
        b"x ssh-ed25519 AAAA\n".to_vec(),
    );
}

#[test]
fn held_in_a_signed_repository() {
    // req:web.worker.review/review-feedback@6f6d24a4
    // In a signed repository only an approval whose line carries a non-empty `sig` clears a
    // request: the unsigned one leaves `r` held, the signed one clears it.
    for (sig, held) in [
        (None, true),
        (Some(""), true),
        (Some("-----SIG-----"), false),
    ] {
        let (fake, cookie) = setup("write");
        set(&fake, sign_main);
        let f = fingerprint(&fake);
        {
            let mut repos = fake.gh.repos.borrow_mut();
            let repo = repos.get_mut("o/a").unwrap();
            let mut approval = json!({"id": "a1", "writer": "alice/1a2b3c4d",
                "at": "2027-01-15T07:10:00.000Z", "kind": "approved", "fingerprint": f,
                "branch": "spec/x", "change": "x", "clears": ["r"]});
            if let Some(sig) = sig {
                approval["sig"] = json!(sig);
            }
            let request = json!({"id": "r", "writer": "alice/web",
                "at": "2027-01-15T07:00:00.000Z", "kind": "changes_requested",
                "branch": "spec/x", "fingerprint": f, "text": "Split it"});
            ledger(repo, "alice/web", "LA", &[request]);
            ledger(repo, "alice/1a2b3c4d", "LB", &[approval]);
        }
        let review = review(&fake, &cookie);
        assert_eq!(review["signed"], true);
        let r = &review["change_requests"][0];
        assert_eq!((&r["id"], &r["current"]), (&json!("r"), &json!(true)));
        assert_eq!(r["held"], held, "sig {sig:?}");
    }
    // The same unsigned approval clears it in a repository that isn't signed.
    let (fake, cookie) = setup("write");
    let f = fingerprint(&fake);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let records = [
            json!({"id": "r", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
                   "kind": "changes_requested", "branch": "spec/x", "fingerprint": f,
                   "text": "Split it"}),
            json!({"id": "a1", "writer": "alice/1a2b3c4d", "at": "2027-01-15T07:10:00.000Z",
                   "kind": "approved", "fingerprint": f, "branch": "spec/x", "change": "x",
                   "clears": ["r"]}),
        ];
        ledger(repo, "alice/web", "LA", &records);
    }
    assert_eq!(review(&fake, &cookie)["change_requests"][0]["held"], false);
}

/// An access check for the signed-in user `u` on `o/a`, as the router makes it.
fn access(fake: &Fake) -> clm_web_platform::access::Access {
    block_on(clm_web_platform::access::check(fake, "u", "o", "a")).unwrap()
}

/// A comment, checked by `parse_comment`.
fn comment(body: &Value) -> clm_web_review::feedback::Comment {
    clm_web_review::feedback::parse_comment(body.to_string().as_bytes())
        .unwrap_or_else(|_| panic!("{body}"))
}

#[test]
fn the_target_cant_be_checked() {
    // req:web.worker.review/review-feedback@6f6d24a4
    // A snapshot that can't be read, and one that lacks the target while GitHub fails the
    // fetch of the branch's spec files: 502 and no write. A comment with no target needs
    // neither.
    use clm_web_review::feedback;
    let (fake, _) = setup("read");
    let f = fingerprint(&fake);
    let access = access(&fake);
    let on = |target: Value| comment(&json!({"fingerprint": f, "target": target, "text": "Hm"}));
    let unread = async { None };
    let res = block_on(feedback::comment(
        &fake,
        &access,
        "clunny",
        "spec/x",
        on(json!("n/r")),
        unread,
    ));
    assert_eq!(res.status, 502, "no snapshot");
    fake.gh.graphql_null.borrow_mut().insert("Blobs".into());
    let lacking = async { Some(std::collections::BTreeSet::new()) };
    let res = block_on(feedback::comment(
        &fake,
        &access,
        "clunny",
        "spec/x",
        on(json!("n/r")),
        lacking,
    ));
    assert_eq!(res.status, 502, "no comparison");
    assert!(fake.gh.writes().is_empty());
    let unread = async { None };
    let res = block_on(feedback::comment(
        &fake,
        &access,
        "clunny",
        "spec/x",
        on(Value::Null),
        unread,
    ));
    assert_eq!(res.status, 200, "no target, nothing to check");
}

#[test]
fn an_odd_body() {
    // req:web.worker.review/review-feedback@6f6d24a4
    use clm_web_review::feedback::parse_comment;
    let long = "é".repeat(4_000);
    let refused = |body: &[u8]| refusal(parse_comment(body));
    assert_eq!(refused(b"not json"), Some(json!({"error": "fingerprint"})));
    assert_eq!(
        refused(br#"{"target": 3}"#),
        Some(json!({"error": "fingerprint"}))
    );
    assert_eq!(
        refused(br#"{"fingerprint": "f", "target": 3, "text": ""}"#),
        Some(json!({"error": "target"}))
    );
    // The two it accepts: no target reads as null, and 4,000 characters is not too many.
    let no_target = json!({"fingerprint": "f", "text": "x"});
    let long_text = json!({"fingerprint": "f", "text": long});
    let (fake, cookie) = setup("read");
    for body in [&no_target, &long_text] {
        assert_eq!(refused(body.to_string().as_bytes()), None, "{body}");
        assert_eq!(send(&fake, &cookie, "comments", body).status, 200, "{body}");
    }
    let records = records(&fake);
    assert_eq!(records[0]["target"], Value::Null);
    assert!(records[0].as_object().unwrap().contains_key("target"));
    assert_eq!(records[1]["text"], long.as_str());
}

/// `setup(permission)` with a web ledger for `clunny` at `L1`.
fn with_ledger(permission: &str) -> (Fake, String) {
    let (fake, cookie) = setup(permission);
    let old = json!({"id": "0", "writer": "clunny/web", "at": "2026-10-26T10:00:00.000Z",
                     "kind": "commented", "branch": "spec/y", "fingerprint": "f0",
                     "target": null, "text": "old"});
    set(&fake, |r| ledger(r, "clunny/web", "L1", &[old]));
    (fake, cookie)
}

fn ledger_tip(fake: &Fake) -> String {
    fake.gh.repos.borrow()["o/a"].branches["clm/ledger/clunny/web"].clone()
}

fn set(fake: &Fake, f: impl FnOnce(&mut FakeRepo)) {
    f(fake.gh.repos.borrow_mut().get_mut("o/a").unwrap())
}

#[test]
fn a_comment_github_fails() {
    // req:web.worker.review/review-feedback@6f6d24a4
    let (fake, cookie) = with_ledger("read");
    let body = json!({"fingerprint": "f", "target": null, "text": "Hm"});
    set(&fake, |r| {
        r.ref_fails.insert("spec/x".into());
    });
    let res = send(&fake, &cookie, "comments", &body);
    assert_eq!(res.status, 502, "the branch's ref");
    assert!(fake.gh.writes().is_empty());
    set(&fake, |r| {
        r.ref_fails.clear();
        r.refused_refs.insert("clm/ledger/clunny/web".into());
    });
    let res = send(&fake, &cookie, "comments", &body);
    assert_eq!(res.status, 502, "the ledger's ref");
    assert_eq!(ledger_tip(&fake), "L1", "the ledger's ref is where it was");
}

#[test]
fn what_a_refused_request_says() {
    // req:web.worker.review/request-changes@e3dfe01b
    let request = |fake: &Fake, cookie: &str, branch: &str, f: &str| {
        let body = json!({"fingerprint": f, "text": "Split it"}).to_string();
        fake.handle(post(
            &format!("/api/repos/o/a/branches/{branch}/request-changes"),
            cookie,
            body.as_bytes(),
        ))
    };
    let (fake, cookie) = with_ledger("write");
    let f = fingerprint(&fake);
    let res = request(&fake, &cookie, "spec/nope", &f);
    assert_eq!(
        (res.status, body_json(&res)),
        (422, json!({"error": "branch"}))
    );
    set(&fake, |r| {
        r.truncated_trees.insert("222".into());
    });
    let res = request(&fake, &cookie, "spec/x", &f);
    assert_eq!(
        (res.status, body_json(&res)),
        (422, json!({"error": "fingerprint"}))
    );
    set(&fake, |r| {
        r.truncated_trees.clear();
        r.fail_paths.push("installation".into());
    });
    let res = request(&fake, &cookie, "spec/x", &f);
    assert_eq!(res.status, 502, "no installation");
    set(&fake, |r| {
        r.fail_paths.clear();
        r.ref_fails.insert("spec/x".into());
    });
    let res = request(&fake, &cookie, "spec/x", &f);
    assert_eq!(res.status, 502, "the branch's ref");
    assert!(fake.gh.writes().is_empty());
    set(&fake, |r| {
        r.ref_fails.clear();
        r.refused_refs.insert("clm/ledger/clunny/web".into());
    });
    let res = request(&fake, &cookie, "spec/x", &f);
    assert_eq!(res.status, 502, "the ledger's ref");
    assert_eq!(ledger_tip(&fake), "L1", "the ledger's ref is where it was");
}
