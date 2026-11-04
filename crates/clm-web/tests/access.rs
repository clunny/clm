//! Every request for a repo's data checks, with the requester's own token, that they can
//! read it, whatever is cached or held by the repo's `Replica` object.

mod common;

use clm_web_platform::platform::Auth;
use common::*;

const SECRET: &str = "SECRET-CONTENT-OF-THE-REPO";

/// The repo routes, as (method, path suffix).
const ROUTES: [(&str, &str); 7] = [
    ("GET", "snapshot"),
    ("GET", "capabilities"),
    ("GET", "architecture"),
    ("GET", "branches"),
    ("GET", "branches/spec/x/review"),
    ("GET", "implementation"),
    ("POST", "branches/spec/x/approve"),
];

fn request(method: &str, repo: &str, route: &str, cookie: &str) -> clm_web_platform::http::Request {
    let path = format!("/api/repos/{repo}/{route}");
    match method {
        "GET" => get(&path, cookie),
        // An approve names what was reviewed, so the request reaches the access check.
        _ => post(&path, cookie, br#"{"fingerprint": "f"}"#),
    }
}

/// Puts the views of `repo` where the Worker would find them: in its `Replica` object and in
/// the Cache API, with live state beside them.
fn hold_everything(fake: &Fake, repo: &str) {
    let secret = serde_json::json!({ "secret": SECRET });
    let snap = secret.to_string();
    for view in ["snapshot", "implementation", "capabilities", "architecture"] {
        fake.set_view(repo, view, secret.clone());
    }
    block_on(async {
        use clm_web_platform::platform::Cache;
        for view in ["snapshot", "implementation", "capabilities", "architecture"] {
            fake.cache_put(&format!("view:{repo}:{view}"), &snap, 60)
                .await;
        }
        for key in [
            "pulls",
            "branches",
            "impl/",
            "events",
            "ledger",
            "page:spec/x",
        ] {
            fake.cache_put(&format!("live:{repo}:{key}"), &snap, 60)
                .await;
        }
    });
}

fn repo_calls(fake: &Fake, repo: &str) -> usize {
    let url = format!("https://api.github.com/repos/{repo}");
    fake.gh
        .calls
        .borrow()
        .iter()
        .filter(|c| c.url == url)
        .count()
}

#[test]
fn held_repo_no_access() {
    // req:web.worker/repo-access@d563a5e1
    let fake = Fake::default();
    fake.gh
        .add_repo("o/private", FakeRepo::new("111", clm_repo()));
    hold_everything(&fake, "o/private");
    let cookie = fake.user("u", &[]);
    for (method, route) in ROUTES {
        let res = fake.handle(request(method, "o/private", route, &cookie));
        assert_eq!(res.status, 404, "{method} {route}");
        assert!(res.body.is_empty(), "{method} {route}");
    }
    assert!(fake.gh.writes().is_empty());
    assert!(
        fake.replicas.calls.borrow().is_empty(),
        "the object was asked nothing"
    );
}

#[test]
fn another_organisation() {
    // req:web.worker/repo-access@d563a5e1
    let fake = Fake::default();
    fake.gh.add_repo("a/x", FakeRepo::new("111", clm_repo()));
    fake.gh.add_repo("b/y", FakeRepo::new("111", clm_repo()));
    hold_everything(&fake, "a/x");
    hold_everything(&fake, "b/y");
    let cookie = fake.user("u", &["a/x"]);
    for (method, route) in ROUTES {
        let res = fake.handle(request(method, "b/y", route, &cookie));
        assert_eq!(res.status, 404, "{method} {route}");
        assert!(!String::from_utf8_lossy(&res.body).contains(SECRET));
    }
    // The check used the user's own token.
    let calls = fake.gh.calls.borrow();
    let check = calls
        .iter()
        .find(|c| c.url == "https://api.github.com/repos/b/y")
        .unwrap();
    assert_eq!(check.auth, Auth::User("u".into()));
}

#[test]
fn access_memo_is_per_repo_and_token() {
    // req:web.worker/repo-access@d563a5e1
    let fake = Fake::default();
    fake.gh.add_repo("a/x", FakeRepo::new("111", clm_repo()));
    fake.gh.add_repo("a/x2", FakeRepo::new("111", clm_repo()));
    let u = fake.user("u", &["a/x"]);
    let v = fake.user("v", &[]);

    fake.handle(get("/api/repos/a/x/branches", &u));
    fake.handle(get("/api/repos/a/x/branches", &u));
    assert_eq!(repo_calls(&fake, "a/x"), 1, "memoised for the same token");

    assert_eq!(fake.handle(get("/api/repos/a/x2/branches", &u)).status, 404);
    assert_eq!(fake.handle(get("/api/repos/a/x/branches", &v)).status, 404);
    assert_eq!(repo_calls(&fake, "a/x"), 2, "another token checks again");

    fake.now.set(NOW + 301);
    fake.handle(get("/api/repos/a/x/branches", &fake.cookie("u")));
    assert_eq!(repo_calls(&fake, "a/x"), 3, "the memo lasts 5 minutes");
}
