//! `?fresh=<delivery>`: a refresh a live notice prompts skips the Worker's cache of a view
//! and asks the repo's `Replica` object with `fresh=1`, and reads the live state of the
//! spec-branch routes from GitHub anew the first time the delivery is asked for, and from the
//! Cache API for 60 seconds after.

mod common;

use common::*;
use serde_json::json;

fn setup() -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

/// Puts `view` where the Worker keeps the object's answers, as of now.
fn cached(fake: &Fake, name: &str, view: &serde_json::Value) {
    block_on(async {
        use clm_web_platform::platform::Cache;
        fake.cache_put(&format!("view:o/a:{name}"), &view.to_string(), 30)
            .await;
    });
}

#[test]
fn a_notices_refresh() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    cached(&fake, "implementation", &json!({"sha": "111"}));
    fake.now.set(NOW + 10);
    fake.set_view("o/a", "implementation", json!({"sha": "222"}));
    let stale = body_json(&fake.handle(get("/api/repos/o/a/implementation", &cookie)));
    assert_eq!(stale["sha"], "111", "the Worker's copy, 10 seconds old");
    assert!(fake.replicas.asked("o/a").is_empty());
    let fresh = body_json(&fake.handle(get("/api/repos/o/a/implementation?fresh=d1", &cookie)));
    assert_eq!(fresh["sha"], "222", "the newer view");
    let again = body_json(&fake.handle(get("/api/repos/o/a/implementation", &cookie)));
    assert_eq!(again["sha"], "222", "kept for the reads after");
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1, "the object was asked once");
    assert_eq!(asked[0].path, "/view/implementation?fresh=1");
}

#[test]
fn every_view_route_takes_it() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    for name in ["snapshot", "capabilities", "architecture"] {
        cached(&fake, name, &json!({"sha": "111"}));
        fake.set_view("o/a", name, json!({"sha": "222"}));
        let res = fake.handle(get(&format!("/api/repos/o/a/{name}?fresh=d2"), &cookie));
        assert_eq!(body_json(&res)["sha"], "222", "{name}");
    }
    let paths: Vec<String> = fake
        .replicas
        .asked("o/a")
        .into_iter()
        .map(|c| c.path)
        .collect();
    assert_eq!(
        paths,
        [
            "/view/snapshot?fresh=1",
            "/view/capabilities?fresh=1",
            "/view/architecture?fresh=1"
        ]
    );
}

#[test]
fn other_values_are_ignored() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    cached(&fake, "snapshot", &json!({"sha": "111"}));
    fake.set_view("o/a", "snapshot", json!({"sha": "222"}));
    let long = "a".repeat(65);
    for value in ["", "D1", "d1;x", "d%2F1", "g", long.as_str()] {
        let path = format!("/api/repos/o/a/snapshot?fresh={value}");
        let snap = body_json(&fake.handle(get(&path, &cookie)));
        assert_eq!(snap["sha"], "111", "{value:?} is ignored");
    }
    assert!(fake.replicas.asked("o/a").is_empty());
    let longest = "0123456789abcdef-".repeat(4)[..64].to_string();
    let path = format!("/api/repos/o/a/snapshot?fresh={longest}");
    assert_eq!(body_json(&fake.handle(get(&path, &cookie)))["sha"], "222");
}

#[test]
fn fresh_keys_name_the_delivery() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    fake.handle(get("/api/repos/o/a/branches?fresh=d3", &cookie));
    let cache = fake.cache.borrow();
    assert!(
        cache.keys().any(|k| k.starts_with("live:o/a#d3:")),
        "{:?}",
        cache.keys()
    );
}

#[test]
fn reviews_read_fresh_live_state() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        let mut head = repo.commits["111"].clone();
        head.insert(
            "specs/a/SPEC.md".into(),
            node("", &["p", "x"], "active").into_bytes(),
        );
        repo.commits.insert("222".into(), head);
        repo.branches.insert("spec/x".into(), "222".into());
    }
    let res = fake.handle(get(
        "/api/repos/o/a/branches/spec/x/review?fresh=d4",
        &cookie,
    ));
    assert_eq!(res.status, 200);
    let cache = fake.cache.borrow();
    assert!(
        cache.contains_key("live:o/a#d4:ledger"),
        "{:?}",
        cache.keys()
    );
}

#[test]
fn an_approval_just_made() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    let base = fake.gh.repos.borrow()["o/a"].commits["111"].clone();
    let mut head = base.clone();
    head.insert(
        "specs/a/SPEC.md".into(),
        node("", &["p", "x"], "active").into_bytes(),
    );
    let fingerprint = fingerprint_of(&base, &head);
    {
        let mut repos = fake.gh.repos.borrow_mut();
        let repo = repos.get_mut("o/a").unwrap();
        repo.commits.insert("222".into(), head);
        repo.branches.insert("spec/x".into(), "222".into());
    }
    let url = "/api/repos/o/a/branches/spec/x/review";
    let cached = body_json(&fake.handle(get(url, &cookie)));
    assert_eq!(cached["approvals"], serde_json::json!([]));
    // An approval is appended 10 seconds later.
    fake.now.set(NOW + 10);
    ledger(
        fake.gh.repos.borrow_mut().get_mut("o/a").unwrap(),
        "alice/web",
        "L1",
        &[serde_json::json!({
            "id": "a1", "writer": "alice/web", "at": "2026-10-26T10:00:00.000Z",
            "kind": "approved", "fingerprint": fingerprint, "branch": "spec/x", "change": null,
        })],
    );
    let stale = body_json(&fake.handle(get(url, &cookie)));
    assert_eq!(
        stale["approvals"],
        serde_json::json!([]),
        "the minute-old fold"
    );
    let fresh = body_json(&fake.handle(get(
        &format!("{url}?fresh=7c9e6679-7425-40de-944b-e07fc1f90ae7"),
        &cookie,
    )));
    assert_eq!(fresh["approvals"][0]["writer"], "alice/web");
}

#[test]
fn a_deliverys_live_state_is_shared_for_60_seconds() {
    // req:web.worker/fresh-live-state@3f2f7d38
    let (fake, cookie) = setup();
    let queries = |fake: &Fake| fake.gh.calls.borrow().len();
    fake.handle(get("/api/repos/o/a/branches?fresh=d5", &cookie));
    let after_first = queries(&fake);
    fake.now.set(NOW + 30);
    fake.handle(get("/api/repos/o/a/branches?fresh=d5", &cookie));
    // Only the access check, memoised, is left: the live state is shared.
    assert_eq!(
        queries(&fake),
        after_first,
        "the second shares the first's fetch"
    );
    fake.now.set(NOW + 61);
    fake.handle(get("/api/repos/o/a/branches?fresh=d5", &fake.cookie("u")));
    assert!(
        queries(&fake) > after_first,
        "fetched anew after 60 seconds"
    );
}
