//! What the router itself decides about a spec branch's routes, before and around
//! `clm-web-review`: the app's own origin on a write, the PR-numbered routes' removal, the
//! branch's name from the path, the bodies checked first, and a comment's targets.

mod common;

use common::*;
use serde_json::json;

/// `o/a` with the spec branch `spec/x`, whose head adds node `a`, and a pull request for it.
fn setup() -> (Fake, String) {
    let mut base = clm_repo();
    base.insert(
        "specs/m/SPEC.md".into(),
        node("", &["old"], "active").into_bytes(),
    );
    let mut head = base.clone();
    head.insert(
        "specs/a/SPEC.md".into(),
        node("", &["p", "x"], "active").into_bytes(),
    );
    let mut repo = FakeRepo::new("111", base);
    repo.commits.insert("spec".into(), head);
    repo.branches.insert("spec/x".into(), "spec".into());
    repo.pulls = vec![FakePull {
        number: 7,
        state: "open".into(),
        title: "Add x".into(),
        base_ref: "main".into(),
        base_sha: "111".into(),
        head_ref: "spec/x".into(),
        head_sha: "spec".into(),
        ..FakePull::default()
    }];
    let fake = Fake::default();
    fake.gh.add_repo("o/a", repo);
    // The default branch's graph, as the `Replica` object's snapshot view lists it.
    fake.set_view(
        "o/a",
        "snapshot",
        json!({"nodes": [{"id": "m", "requirements": [{"id": "m/old"}]}]}),
    );
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

#[test]
fn approve_needs_same_origin() {
    // req:web.worker/api-guard@b1c7f110
    let (fake, cookie) = setup();
    let url = "/api/repos/o/a/branches/spec/x/approve";
    let mut req = post(url, &cookie, b"");
    req.headers
        .insert("origin".into(), "https://evil.example".into());
    assert_eq!(fake.handle(req).status, 403);
    let mut req = post(url, &cookie, b"");
    req.headers.remove("origin");
    assert_eq!(fake.handle(req).status, 403);
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn old_pr_routes() {
    // req:web.worker/pr-routes-removed@da6b0097
    let (fake, cookie) = setup();
    let refs = || fake.gh.repos.borrow()["o/a"].branches.clone();
    let before = refs();
    for path in ["pulls", "pulls/7/review"] {
        let res = fake.handle(get(&format!("/api/repos/o/a/{path}"), &cookie));
        assert_eq!(res.status, 404, "{path}");
    }
    for body in [
        json!({"head": "spec"}).to_string(),
        json!({"fingerprint": "f"}).to_string(),
        String::new(),
    ] {
        let res = fake.handle(post(
            "/api/repos/o/a/pulls/7/approve",
            &cookie,
            body.as_bytes(),
        ));
        assert_eq!(res.status, 404, "{body}");
    }
    assert!(fake.gh.writes().is_empty());
    assert_eq!(refs(), before, "no ref of o/a changed");
    // The user can read o/a: the routes that replace these answer.
    for path in ["branches", "branches/spec/x/review"] {
        let res = fake.handle(get(&format!("/api/repos/o/a/{path}"), &cookie));
        assert_eq!(res.status, 200, "{path}");
    }
}

#[test]
fn encoded_branch_name() {
    // req:web.worker/branch-routes@d492ab12
    let (fake, cookie) = setup();
    let encoded = fake.handle(get("/api/repos/o/a/branches/spec%2Fx/review", &cookie));
    assert_eq!(encoded.status, 200, "{encoded:?}");
    assert_eq!(body_json(&encoded)["branch"], "spec/x");
    // The name split over segments is the same branch, with the same review.
    let plain = fake.handle(get("/api/repos/o/a/branches/spec/x/review", &cookie));
    assert_eq!(plain.status, 200, "{plain:?}");
    assert_eq!(body_json(&plain), body_json(&encoded));
}

#[test]
fn bodies_are_checked_before_any_request_to_github() {
    // req:web.worker/branch-routes@d492ab12
    let (fake, cookie) = setup();
    let calls = || fake.gh.calls.borrow().len();
    let fingerprint = Some(json!({"error": "fingerprint"}));
    let text = Some(json!({"error": "text"}));
    for (route, body, answer) in [
        ("approve", "", None),
        ("approve", "not json", None),
        ("approve", "{}", None),
        ("approve", r#"{"fingerprint": 3}"#, None),
        ("comments", "", fingerprint.clone()),
        (
            "comments",
            r#"{"fingerprint": "f", "target": null, "text": " "}"#,
            text.clone(),
        ),
        ("request-changes", r#"{"fingerprint": "f"}"#, text.clone()),
    ] {
        let before = calls();
        let url = format!("/api/repos/o/a/branches/spec/x/{route}");
        let res = fake.handle(post(&url, &cookie, body.as_bytes()));
        assert_eq!(res.status, 422, "{route} {body}");
        match &answer {
            None => assert!(res.body.is_empty(), "{route} {body}: {res:?}"),
            Some(answer) => assert_eq!(&body_json(&res), answer, "{route} {body}"),
        }
        assert_eq!(calls(), before, "{route} {body}: GitHub was asked");
    }
    assert!(fake.gh.writes().is_empty(), "nothing is written");
    assert!(
        fake.replicas.asked("o/a").is_empty(),
        "the object was asked"
    );
}

#[test]
fn approve_through_the_router() {
    // req:web.worker/branch-routes@d492ab12
    let (fake, cookie) = setup();
    let f = {
        let repos = fake.gh.repos.borrow();
        let repo = &repos["o/a"];
        fingerprint_of(&repo.commits["111"], &repo.commits["spec"])
    };
    let url = "/api/repos/o/a/branches/spec/x/approve";
    let body = json!({"fingerprint": f}).to_string();
    let res = fake.handle(post(url, &cookie, body.as_bytes()));
    assert_eq!(res.status, 200, "{res:?}");
    assert_eq!(fake.gh.writes().len(), 4, "blob, tree, commit, ref");
}

#[test]
fn comments_take_their_targets_from_the_snapshot() {
    // req:web.worker/branch-routes@d492ab12
    let (fake, cookie) = setup();
    let url = "/api/repos/o/a/branches/spec/x/comments";
    let body = |target: &str| json!({"fingerprint": "f", "target": target, "text": "Hm"});
    // `m/old` is in the default branch's graph, not in the branch's review.
    for (target, status) in [("m/old", 200), ("m/missing", 422)] {
        let res = fake.handle(post(url, &cookie, body(target).to_string().as_bytes()));
        assert_eq!(res.status, status, "{target}: {res:?}");
    }
    assert_eq!(
        fake.gh.writes().len(),
        4,
        "only the first comment is written"
    );

    // Nothing cached and the object down: no target can be looked for, so the review
    // answers 502 and nothing is written.
    let (down, cookie) = setup();
    down.replicas.fails.borrow_mut().insert("o/a".into());
    let res = down.handle(post(url, &cookie, body("m/old").to_string().as_bytes()));
    assert_eq!(res.status, 502, "{res:?}");
    assert!(down.gh.writes().is_empty());
    assert!(
        down.replicas
            .asked("o/a")
            .iter()
            .any(|c| c.path.starts_with("/view/snapshot")),
        "the snapshot view was asked for"
    );
}

#[test]
fn the_list_is_the_reviews_or_502() {
    // req:web.worker/branch-routes@d492ab12
    let (fake, cookie) = setup();
    let res = fake.handle(get("/api/repos/o/a/branches", &cookie));
    assert_eq!(res.status, 200, "{res:?}");
    let list = body_json(&res);
    assert_eq!(list.as_array().map(Vec::len), Some(1), "{list}");
    assert_eq!(list[0]["branch"], "spec/x");
    assert_eq!(list[0]["head_sha"], "spec");
    assert_eq!(list[0]["page"]["number"], 7);

    // GitHub fails the spec branches' listing: the review gives none, and the route 502.
    let (failing, cookie) = setup();
    failing
        .gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .fail_paths
        .push("git/matching-refs/heads/spec/".into());
    let res = failing.handle(get("/api/repos/o/a/branches", &cookie));
    assert_eq!(res.status, 502, "{res:?}");
    assert!(res.body.is_empty());
}
