//! The architecture route answers the view of the repo's `Replica` object, `indexing` and
//! all, and computes nothing of it.

mod common;

use common::*;
use serde_json::json;

#[test]
fn the_objects_report() {
    // req:web.worker/architecture@d99c1357
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    let view = json!({"architecture": [{"slug": "e"}], "signed": false, "indexing": true});
    fake.set_view("o/a", "architecture", view.clone());
    let res = fake.handle(get("/api/repos/o/a/architecture", &cookie));
    assert_eq!(res.status, 200, "{res:?}");
    assert_eq!(body_json(&res), view, "the object's view, as it gave it");
    let asked = fake.replicas.asked("o/a");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].path, "/view/architecture?fresh=0");
    assert!(fake.gh.writes().is_empty());
}

#[test]
fn the_object_still_indexing() {
    // req:web.worker/architecture@d99c1357
    let fake = Fake::default();
    fake.gh.add_repo("o/a", FakeRepo::new("111", clm_repo()));
    let cookie = fake.user("u", &["o/a"]);
    let view = json!({"architecture": [], "signed": true, "indexing": false});
    fake.set_view("o/a", "architecture", view.clone());
    assert_eq!(
        body_json(&fake.handle(get("/api/repos/o/a/architecture", &cookie))),
        view
    );
}
