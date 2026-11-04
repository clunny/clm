//! The view routes answer what the repo's `Replica` object computed, kept in the Cache API
//! for 30 seconds, and compute none of it.

mod common;

use common::*;
use serde_json::json;

fn setup() -> (Fake, String) {
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    (fake, cookie)
}

#[test]
fn polls_within_30_seconds() {
    // req:web.worker/views-from-replica@b85c6eb3
    let (fake, cookie) = setup();
    let view = json!({"sha": "111", "nodes": []});
    fake.set_view("o/a", "snapshot", view.clone());
    for _ in 0..3 {
        let res = fake.handle(get("/api/repos/o/a/snapshot", &cookie));
        assert_eq!(res.status, 200);
        assert_eq!(body_json(&res), view);
    }
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1, "the object was asked once");
    assert_eq!(asked[0].method, "GET");
    assert_eq!(asked[0].path, "/view/snapshot?fresh=0");
    // Past the 30 seconds it is asked again.
    fake.now.set(NOW + 31);
    fake.set_view("o/a", "snapshot", json!({"sha": "222"}));
    let res = fake.handle(get("/api/repos/o/a/snapshot", &fake.cookie("u")));
    assert_eq!(body_json(&res)["sha"], "222");
    assert_eq!(fake.replicas.asked("o/a").len(), 2);
}

#[test]
fn views_are_kept_per_repo_and_name() {
    // req:web.worker/views-from-replica@b85c6eb3
    let (fake, cookie) = setup();
    fake.gh.add_repo("o/b", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/b");
    fake.set_view("o/a", "snapshot", json!({"repo": "a"}));
    fake.set_view("o/b", "snapshot", json!({"repo": "b"}));
    fake.set_view("o/a", "capabilities", json!({"report": "a"}));
    let one = body_json(&fake.handle(get("/api/repos/o/a/snapshot", &cookie)));
    let two = body_json(&fake.handle(get("/api/repos/o/b/snapshot", &cookie)));
    let three = body_json(&fake.handle(get("/api/repos/o/a/capabilities", &cookie)));
    assert_eq!(
        (
            one["repo"].clone(),
            two["repo"].clone(),
            three["report"].clone()
        ),
        (json!("a"), json!("b"), json!("a"))
    );
}

#[test]
fn preparing() {
    // req:web.worker/views-from-replica@b85c6eb3
    let (fake, cookie) = setup();
    fake.set_answer("o/a", "snapshot", 503, json!({"preparing": 0.4}));
    for _ in 0..2 {
        let res = fake.handle(get("/api/repos/o/a/snapshot", &cookie));
        assert_eq!(res.status, 503);
        assert_eq!(body_json(&res), json!({"preparing": 0.4}));
    }
    assert_eq!(fake.replicas.asked("o/a").len(), 2, "uncached");
    // Once it is ready the view is served and kept.
    fake.set_view("o/a", "snapshot", json!({"sha": "111"}));
    assert_eq!(
        fake.handle(get("/api/repos/o/a/snapshot", &cookie)).status,
        200
    );
}

#[test]
fn object_down() {
    // req:web.worker/views-from-replica@b85c6eb3
    let (fake, cookie) = setup();
    fake.replicas.bound.set(false);
    for route in ["capabilities", "snapshot", "implementation", "architecture"] {
        let res = fake.handle(get(&format!("/api/repos/o/a/{route}"), &cookie));
        assert_eq!(res.status, 503, "{route}");
        assert_eq!(body_json(&res), json!({"error": "unavailable"}), "{route}");
    }
    // An answer the Worker can't read is the same.
    fake.replicas.bound.set(true);
    fake.set_answer("o/a", "architecture", 500, json!({"boom": true}));
    let res = fake.handle(get("/api/repos/o/a/architecture", &cookie));
    assert_eq!(body_json(&res), json!({"error": "unavailable"}));
}

#[test]
fn proposals_for_each_reader() {
    // req:web.worker/views-from-replica@b85c6eb3
    let fake = Fake::default();
    fake.assets.borrow_mut().insert(
        "/_clm/invited.json".into(),
        ("application/json".into(), br#"["alice", "bob"]"#.to_vec()),
    );
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    fake.gh.grant("u", "o/a");
    fake.gh.grant("v", "o/a");
    let alice = fake.cookie_as("u", "alice");
    let bob = fake.cookie_as("v", "bob");
    let proposal = json!({
        "id": "p1", "writer": "alice/web", "at": "2027-01-15T07:00:00.000Z",
        "slug": "export", "title": "Export", "body": "", "nodes": [],
    });
    fake.set_view(
        "o/a",
        "implementation",
        json!({"sha": "111", "flight": {}, "proposals": [proposal]}),
    );
    let calls = fake.gh.calls.borrow().len();
    let mine = body_json(&fake.handle(get("/api/repos/o/a/implementation", &alice)));
    assert_eq!(mine["flight"], json!({}), "the object's view, kept whole");
    assert_eq!(mine["proposals"][0]["id"], "p1");
    assert_eq!(mine["proposals"][0]["mine"], true);
    let theirs = body_json(&fake.handle(get("/api/repos/o/a/implementation", &bob)));
    assert_eq!(
        theirs["proposals"][0]["mine"], false,
        "the cached view, their own mine"
    );
    let asked: Vec<String> = fake
        .replicas
        .asked("o/a")
        .into_iter()
        .map(|c| c.path)
        .collect();
    assert_eq!(
        asked,
        ["/view/implementation?fresh=0"],
        "asked once, for the implementation view"
    );
    // The access check reads GitHub for each reader, and nothing else does.
    let after = fake.gh.calls.borrow().len();
    let reads = fake.gh.calls.borrow()[calls..after].to_vec();
    assert!(
        reads.iter().all(|c| !format!("{c:?}").contains("ledger")),
        "no ledger was read from GitHub: {reads:?}"
    );
}

#[test]
fn a_view_without_proposals() {
    // req:web.worker/views-from-replica@b85c6eb3
    let (fake, cookie) = setup();
    fake.set_view("o/a", "implementation", json!({"sha": "111"}));
    let view = body_json(&fake.handle(get("/api/repos/o/a/implementation", &cookie)));
    assert_eq!(view["sha"], "111");
    assert_eq!(view["proposals"], json!([]));
    assert_eq!(fake.replicas.asked("o/a").len(), 1);
}
