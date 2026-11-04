//! `replica/apply-what-changed`, `replica/batched-recompute` and `replica/preparing`.

mod common;

use clm_replica::{Answer, Change, Event, Replica};
use common::*;
use serde_json::Value;

const NAMES: [&str; 4] = ["snapshot", "implementation", "capabilities", "architecture"];

/// A repository at `111`: `a/p` cited by `t.rs`, and files `u.rs` and `v.rs`.
fn base() -> FileMap {
    let mut f = clm_repo();
    let v = version_of(&f, "a/p");
    f.insert("t.rs".into(), cites("a/p", &v).into_bytes());
    f.insert("u.rs".into(), b"fn u() {}\n".to_vec());
    f.insert("v.rs".into(), b"fn v() {}\n".to_vec());
    f
}

fn views(r: &Replica) -> Vec<Value> {
    NAMES.iter().map(|n| view(r, n)).collect()
}

#[test]
fn incremental_push_equals_scratch() {
    // req:replica/apply-what-changed@2585c229
    let before = base();
    let mut after = before.clone();
    // `t.rs` now cites `a/p` at an old version, `specs/a/SPEC.md` gains a requirement, and
    // `u.rs` is gone.
    after.insert("t.rs".into(), cites("a/p", "00000000").into_bytes());
    after.insert(
        "specs/a/SPEC.md".into(),
        node("", &["p", "r"], "active").into_bytes(),
    );
    after.remove("u.rs");

    let mut r = ready(&before, "111", 100);
    r.apply(Change::Tip {
        sha: "222".into(),
        files: vec![
            file("t.rs", &after["t.rs"]),
            file("specs/a/SPEC.md", &after["specs/a/SPEC.md"]),
        ],
        removed: vec!["u.rs".into()],
    });
    assert!(r.stale());
    r.recompute(100);

    let fresh = ready(&after, "222", 100);
    assert_eq!(views(&r), views(&fresh));
    assert_eq!(r, fresh, "the state too");
    let snap = view(&r, "snapshot");
    assert_eq!(snap["sha"], "222");
    assert_eq!(snap["nodes"][0]["requirements"][0]["state"], "behind");
}

#[test]
fn force_push_recovers_from_a_tree() {
    // req:replica/apply-what-changed@2585c229
    let before = base();
    let mut after = before.clone();
    after.insert("v.rs".into(), b"fn v2() {}\n".to_vec());
    after.insert("w.rs".into(), b"fn w() {}\n".to_vec());
    after.remove("t.rs");

    let mut r = ready(&before, "111", 100);
    r.apply(tree("333", &after));
    // Only the files whose blob SHAs it doesn't hold are still to fetch.
    assert_eq!(r.pending_fetch(), ["v.rs", "w.rs"]);
    fetch(&mut r, &after, "333");
    r.recompute(100);

    let fresh = ready(&after, "333", 100);
    assert_eq!(views(&r), views(&fresh));
    assert_eq!(r, fresh);
    let snap = view(&r, "snapshot");
    assert_eq!(snap["nodes"][0]["requirements"][0]["state"], "unbuilt");
}

#[test]
fn ledgers_refs_and_pulls_apply_too() {
    // req:replica/apply-what-changed@2585c229
    let f = base();
    let mut r = ready(&f, "111", 100);
    let mut other = ready(&f, "111", 100);
    for target in [&mut r, &mut other] {
        target.apply(Change::Ledger {
            branch: "clm/ledger/a/b".into(),
            text: Some("not a record\n".into()),
        });
        target.apply(Change::Ledger {
            branch: "clm/ledger/c/d".into(),
            text: Some(String::new()),
        });
    }
    // One ledger deleted, one ref moved twice, one pull request opened then closed.
    r.apply(Change::Ledger {
        branch: "clm/ledger/c/d".into(),
        text: None,
    });
    other.apply(Change::Ledger {
        branch: "clm/ledger/c/d".into(),
        text: None,
    });
    let moved = |sha: &str, time| Change::Ref {
        name: "impl/a".into(),
        sha: Some(sha.into()),
        time,
        change: None,
    };
    r.apply(moved("x", 1));
    r.apply(moved("y", 2));
    other.apply(moved("y", 2));
    r.apply(Change::Pull(pull(7, "impl/a", "y")));
    let mut closed = pull(7, "impl/a", "y");
    closed.open = false;
    r.apply(Change::Pull(closed));
    r.recompute(100);
    other.recompute(100);
    assert_eq!(r, other);
    // And a replica that never held the deleted ledger or the closed pull request.
    let mut never = ready(&f, "111", 100);
    never.apply(Change::Ledger {
        branch: "clm/ledger/a/b".into(),
        text: Some("not a record\n".into()),
    });
    never.apply(moved("y", 2));
    never.recompute(100);
    assert_eq!(r, never);
}

#[test]
fn three_pushes_one_recompute() {
    // req:replica/batched-recompute@5e04a3ca
    let mut r = ready(&base(), "111", 100);
    for sha in ["222", "333", "444"] {
        r.apply(Change::Tip {
            sha: sha.into(),
            files: Vec::new(),
            removed: Vec::new(),
        });
    }
    let described = |r: &Replica| match r.view("snapshot").unwrap() {
        Answer::Ready {
            sha, computed_at, ..
        } => (sha, computed_at),
        Answer::Preparing { .. } => panic!("preparing"),
    };
    assert_eq!(described(&r), ("111".to_string(), 100));
    r.recompute(160);
    assert_eq!(described(&r), ("444".to_string(), 160));
    assert!(!r.stale());
}

#[test]
fn a_claim_lapses_four_hours_after_its_push() {
    // req:replica/batched-recompute@5e04a3ca
    let ten = 10 * 3600;
    let mut r = scratch(&base(), "111");
    r.apply(Change::Ref {
        name: "impl/a".into(),
        sha: Some("c1".into()),
        time: ten,
        change: None,
    });
    let next = r.recompute(11 * 3600).next;
    assert_eq!(next, Some(14 * 3600));
    // A lapsed claim is no longer live, and nothing is left to wait for.
    assert_eq!(r.recompute(15 * 3600).next, None);
    // At the lapse itself it is still live, so the next recompute is a moment later.
    assert_eq!(r.recompute(14 * 3600).next, Some(14 * 3600 + 1));
    // Without a claim, no time.
    let mut none = scratch(&base(), "111");
    assert_eq!(none.recompute(100).next, None);
}

#[test]
fn recompute_waits_for_the_spec_files() {
    // req:replica/batched-recompute@5e04a3ca
    let f = base();
    let mut r = Replica::default();
    r.apply(tree("111", &f));
    assert_eq!(r.recompute(100).next, None);
    assert!(r.stale(), "nothing was computed");
    assert!(matches!(r.view("snapshot"), Some(Answer::Preparing { .. })));
    assert_eq!(r.view("nothing"), None);
}

#[test]
fn preparing_until_the_spec_files_are_in() {
    // req:replica/preparing@73597926
    let mut f = files(&[
        ("ARCHITECTURE.md", "# A\n"),
        ("specs/SPEC.md", &node("", &[], "active")),
    ]);
    for i in 0..8 {
        f.insert(
            format!("specs/n{i}/SPEC.md"),
            node("", &[], "active").into_bytes(),
        );
    }
    for i in 0..990 {
        f.insert(
            format!("src/f{i}.rs"),
            format!("fn f{i}() {{}}\n").into_bytes(),
        );
    }
    // The 10 spec files: the root, 8 nodes and ARCHITECTURE.md.
    let spec: Vec<String> = f
        .keys()
        .filter(|p| !p.starts_with("src/"))
        .cloned()
        .collect();
    assert_eq!(spec.len(), 10);

    let mut r = Replica::default();
    assert_eq!(r.view("snapshot"), Some(Answer::Preparing { done: 0.0 }));
    r.apply(tree("111", &f));
    assert_eq!(r.view("snapshot"), Some(Answer::Preparing { done: 0.0 }));
    // The spec files come first in the order to fetch.
    assert_eq!(&r.pending_fetch()[..10], &spec[..]);

    let batch = |paths: &[String]| Change::Tip {
        sha: "111".into(),
        files: paths.iter().map(|p| file(p, &f[p])).collect(),
        removed: Vec::new(),
    };
    r.apply(batch(&spec[..5]));
    r.recompute(100);
    assert_eq!(r.view("snapshot"), Some(Answer::Preparing { done: 0.5 }));

    r.apply(batch(&spec[5..]));
    r.recompute(100);
    let snap = view(&r, "snapshot");
    assert_eq!(snap["coverage"]["state"], "indexing");
    assert_eq!(snap["coverage"]["done"], 0.0);
    assert_eq!(r.pending_fetch().len(), 990);
    assert_eq!(r.pending_fetch()[0], "src/f0.rs");
    assert!(snap["nodes"][0]["requirements"][0]["state"].is_null(),);
    let arch = view(&r, "architecture");
    assert_eq!(arch["indexing"], true);
    assert!(
        snap["spec"]["nodes"].is_array(),
        "the views are the model's shape"
    );
}

#[test]
fn events_keep_the_latest_hundred() {
    // req:replica/apply-what-changed@2585c229
    let mut r = ready(&base(), "111", 1000);
    for at in 0..150u64 {
        r.apply(Change::Event(Event::Push {
            branch: "impl/a".into(),
            at,
            actor: "u".into(),
        }));
    }
    r.apply(Change::Event(Event::Create {
        branch: "spec/x".into(),
        at: 500,
        actor: "u".into(),
    }));
    r.recompute(1000);
    let feed = view(&r, "implementation")["activity"].clone();
    let feed = feed.as_array().unwrap();
    assert_eq!(feed.len(), 100);
    assert_eq!(feed[0]["at"], 149);
    assert_eq!(feed[99]["at"], 50);
}
