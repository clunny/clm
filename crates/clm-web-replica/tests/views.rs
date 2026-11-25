//! `GET /view/<name>`: the views the object holds, while it prepares them and when the
//! reader asks for the tip (`answers-views`).

mod common;

use clm_replica::{Change, Entry, File, Replica, Store};
use clm_web_platform::fake::blob_sha;
use common::*;
use futures::executor::block_on;
use serde_json::{Value, json};

fn parsed(body: &str) -> Value {
    serde_json::from_str(body).unwrap()
}

/// An object whose replica holds a tip whose two spec files are listed and one fetched.
fn half_fetched() -> (Obj, Slot) {
    let texts = [
        ("specs/SPEC.md", "---\ntitle: R\n---\n"),
        ("specs/a/SPEC.md", "---\ntitle: A\n---\n"),
    ];
    let entry = |(path, text): (&str, &str)| Entry {
        path: path.into(),
        sha: blob_sha(text.as_bytes()),
        size: text.len() as u64,
        mode: "100644".into(),
    };
    let mut replica = Replica::default();
    replica.apply(Change::Tree {
        sha: "111".into(),
        entries: texts.iter().map(|t| entry(*t)).collect(),
    });
    replica.apply(Change::Tip {
        sha: "111".into(),
        files: vec![File::text(entry(texts[0]), texts[0].1)],
        removed: Vec::new(),
    });
    let mut mem = Mem::default();
    replica.save(&mut mem);
    mem.put("meta", json!({"tip": "111"}).to_string().into_bytes());
    let slot = Slot::default();
    (
        clm_web_replica::Object::load("o/a", mem, slot.clone()),
        slot,
    )
}

#[test]
fn a_view_read() {
    // req:web.worker.replica/answers-views@158021b4
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, _slot) = built(&fake);
    let answer = block_on(object.view(&fake, "snapshot", false));
    assert_eq!(answer.status, 200, "{}", answer.body);
    let read = parsed(&answer.body);
    let keys: Vec<&String> = read.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["computed_at", "sha", "view"]);
    assert_eq!(read["sha"], "111");
    assert_eq!(read["view"]["sha"], "111", "the view is the snapshot");
    assert!(read["view"]["coverage"].is_object(), "{read}");
    // `computed_at` is the last recompute's time: a recompute at a later time moves it.
    let computed = read["computed_at"].as_u64().unwrap();
    assert!(computed <= fake.now.get());
    fake.now.set(fake.now.get() + 120);
    commit(&fake, "222", repo_files(&[("t.rs", "fn t() {}")]));
    let fresh = parsed(&block_on(object.view(&fake, "snapshot", true)).body);
    assert_eq!(fresh["computed_at"], fake.now.get());

    // `nope` is no view's name: 404 with no body.
    let nope = block_on(object.view(&fake, "nope", false));
    assert_eq!((nope.status, nope.body.as_str()), (404, ""));

    // The spec files half fetched: 503 with the share, and the alarm set to now.
    let (mut half, slot) = half_fetched();
    let answer = block_on(half.view(&fake, "snapshot", false));
    assert_eq!(answer.status, 503);
    assert_eq!(parsed(&answer.body), json!({"preparing": 0.5}));
    assert_eq!(
        slot.0.get(),
        Some(fake.now.get()),
        "the alarm is set to now"
    );

    // An object that holds nothing answers 503 `{"preparing": 0}` for any name.
    let (mut empty, _mem, slot) = common::object();
    let answer = block_on(empty.view(&fake, "nope", false));
    assert_eq!(
        (answer.status, parsed(&answer.body)),
        (503, json!({"preparing": 0}))
    );
    assert_eq!(slot.0.get(), Some(fake.now.get()), "the build starts");
}

#[test]
fn a_fresh_read() {
    // req:web.worker.replica/answers-views@158021b4
    let fake = fake_with("111", repo_files(&[]));
    let (mut object, _mem, _slot) = built(&fake);
    // GitHub's default branch holds a push the object hasn't been told of.
    commit(&fake, "222", repo_files(&[("t.rs", "fn t() {}")]));
    let stale = view(&fake, &mut object, "snapshot");
    assert_eq!(stale["sha"], "111", "a plain read answers what it holds");
    let fresh = block_on(object.view(&fake, "snapshot", true));
    assert_eq!(fresh.status, 200, "{}", fresh.body);
    let fresh = parsed(&fresh.body);
    assert_eq!(fresh["sha"], "222");
    assert_eq!(fresh["view"]["sha"], "222");

    // Then GitHub fails the ref read: the object answers the view it holds.
    commit(&fake, "333", repo_files(&[("t.rs", "fn t3() {}")]));
    fake.gh
        .repos
        .borrow_mut()
        .get_mut("o/a")
        .unwrap()
        .ref_fails
        .insert("main".into());
    let held = block_on(object.view(&fake, "snapshot", true));
    assert_eq!(held.status, 200, "{}", held.body);
    assert_eq!(parsed(&held.body)["sha"], "222");
}
