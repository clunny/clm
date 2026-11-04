//! `replica/implementation-ledger`, `replica/taken-items` and `replica/implementation-models`.

mod common;

use clm_replica::{Change, Event, RefChange, Replica};
use common::*;
use serde_json::{Value, json};

/// 2026-10-01 00:00 UTC, in Unix seconds.
const DAY: u64 = 1_790_812_800;

fn at(h: u64, m: u64) -> u64 {
    DAY + h * 3600 + m * 60
}

fn stamp(h: u64, m: u64) -> String {
    format!("2026-10-01T{h:02}:{m:02}:00.000Z")
}

fn record(fields: Value) -> String {
    format!("{fields}\n")
}

fn ledger(r: &mut Replica, branch: &str, lines: &[String]) {
    r.apply(Change::Ledger {
        branch: branch.into(),
        text: Some(lines.concat()),
    });
}

fn push(r: &mut Replica, branch: &str, h: u64, m: u64) {
    r.apply(Change::Event(Event::Push {
        branch: branch.into(),
        at: at(h, m),
        actor: "gh-user".into(),
    }));
}

fn branch_ref(r: &mut Replica, name: &str, sha: &str, time: u64, change: Option<RefChange>) {
    r.apply(Change::Ref {
        name: name.into(),
        sha: Some(sha.into()),
        time,
        change,
    });
}

#[test]
fn lands_and_approvals_come_from_the_ledger() {
    // req:replica/implementation-ledger@cda28ef8
    let mut r = scratch(&clm_repo(), "111");
    ledger(
        &mut r,
        "clm/ledger/alice/web",
        &[record(json!({
            "id": "1", "kind": "approved", "writer": "alice/web", "at": stamp(10, 0),
            "fingerprint": "f", "branch": "spec/x", "change": null,
        }))],
    );
    ledger(
        &mut r,
        "clm/ledger/bob/1a2b3c4d",
        &[record(json!({
            "id": "2", "kind": "landed", "writer": "bob/1a2b3c4d", "at": stamp(10, 10),
            "commit": "c", "branch": "spec/x", "land_kind": "spec", "review": false,
            "fingerprint": "f", "changes": [],
        }))],
    );
    push(&mut r, "impl/a", 10, 5);
    push(&mut r, "clm/ledger/bob/1a2b3c4d", 10, 10);
    r.recompute(at(11, 0));
    let imp = view(&r, "implementation");
    let feed: Vec<(&str, &str, &str)> = imp["activity"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| {
            (
                a["kind"].as_str().unwrap(),
                a["actor"].as_str().unwrap(),
                a["ref"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        feed,
        [
            ("land", "bob", "spec/x"),
            ("push", "gh-user", "impl/a"),
            ("approval", "alice", "spec/x")
        ]
    );
    assert_eq!(imp["activity"][0]["at"], at(10, 10));
    assert!(
        imp.get("pulls").is_none() && imp.get("soft_lands").is_none(),
        "{imp}"
    );
}

#[test]
fn a_claim_is_a_created_impl_branch() {
    // req:replica/implementation-ledger@cda28ef8
    let mut r = scratch(&clm_repo(), "111");
    r.apply(Change::Event(Event::Create {
        branch: "impl/a".into(),
        at: at(9, 0),
        actor: "gh-user".into(),
    }));
    r.apply(Change::Event(Event::Create {
        branch: "spec/y".into(),
        at: at(9, 5),
        actor: "gh-user".into(),
    }));
    r.recompute(at(10, 0));
    let imp = view(&r, "implementation");
    assert_eq!(
        imp["activity"],
        json!([{"kind": "claim", "at": at(9, 0), "actor": "gh-user", "ref": "a"}])
    );
}

#[test]
fn the_feed_is_a_hundred_entries_at_most() {
    // req:replica/implementation-ledger@cda28ef8
    let mut r = scratch(&clm_repo(), "111");
    let lines: Vec<String> = (0..120)
        .map(|i| {
            record(json!({
                "id": format!("r{i}"), "kind": "approved", "writer": "alice/web",
                "at": format!("2026-10-01T10:{:02}:{:02}.000Z", i / 60, i % 60),
                "fingerprint": "f", "branch": "spec/x", "change": null,
            }))
        })
        .collect();
    ledger(&mut r, "clm/ledger/alice/web", &lines);
    r.recompute(at(11, 0));
    let imp = view(&r, "implementation");
    let feed = imp["activity"].as_array().unwrap();
    assert_eq!(feed.len(), 100);
    assert!(
        feed.windows(2)
            .all(|w| w[0]["at"].as_u64() >= w[1]["at"].as_u64())
    );
}

#[test]
fn a_page_says_whether_its_head_was_checked() {
    // req:replica/implementation-ledger@cda28ef8
    let mut r = scratch(&clm_repo(), "111");
    ledger(
        &mut r,
        "clm/ledger/bob/1a2b3c4d",
        &[record(json!({
            "id": "c", "kind": "checked", "writer": "bob/1a2b3c4d", "at": stamp(10, 0),
            "code_key": "key-a", "command": "ci",
        }))],
    );
    let code = |key: &str| Some(RefChange::Code { key: key.into() });
    branch_ref(&mut r, "impl/a", "sa", at(9, 0), code("key-a"));
    branch_ref(&mut r, "impl/b", "sb", at(9, 0), code("key-b"));
    branch_ref(&mut r, "impl/c", "sc", at(9, 0), None);
    branch_ref(&mut r, "impl/d", "sd", at(9, 0), None);
    r.apply(Change::Pull(pull(7, "impl/a", "sa")));
    r.apply(Change::Pull(pull(8, "impl/b", "sb")));
    // Its head's key isn't known, so `checked` is left out.
    r.apply(Change::Pull(pull(9, "impl/d", "sd")));
    // A fork's pull request from a branch of the same name is no page either.
    let mut fork = pull(5, "impl/a", "sa");
    fork.fork = true;
    r.apply(Change::Pull(fork));
    // A pull request on another branch has no page.
    r.apply(Change::Pull(pull(10, "spec/x", "sx")));
    r.recompute(at(11, 0));
    let pages = &view(&r, "implementation")["pages"];
    assert_eq!(pages["impl/a"]["number"], 7);
    assert_eq!(pages["impl/a"]["url"], "https://example.test/pull/7");
    assert_eq!(pages["impl/a"]["checked"], true);
    assert_eq!(pages["impl/b"]["number"], 8);
    assert_eq!(pages["impl/b"]["checked"], false);
    assert!(pages["impl/d"].get("checked").is_none(), "{pages}");
    assert!(pages.get("impl/c").is_none(), "no pull request, no page");
    assert!(pages.get("spec/x").is_none());
    assert_eq!(pages.as_object().unwrap().len(), 3);

    // A head that moved since the key was read isn't known to be checked.
    r.apply(Change::Pull(pull(7, "impl/a", "newer")));
    r.recompute(at(11, 0));
    assert!(
        view(&r, "implementation")["pages"]["impl/a"]
            .get("checked")
            .is_none()
    );
}

// ---- taken ----

fn change_md(extra: &str) -> String {
    format!("---\nstatus: applied\n{extra}---\n\n## Intent\nI.\n")
}

fn held_fixture() -> FileMap {
    let mut f = clm_repo();
    let delta = "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: z\nIt SHALL z.\n\n#### Scenario: s\n- ok\n";
    f.extend(files(&[
        ("changes/x/change.md", &change_md("resolves: [a]\n")),
        ("changes/x/deltas/a.md", delta),
        (
            "backlog/a.md",
            "---\ntitle: A\nkind: human\ncomponents: [a]\n---\nBody.\n",
        ),
        (
            "backlog/c.md",
            "---\ntitle: C\nkind: human\ncomponents: [b]\n---\nBody.\n",
        ),
    ]));
    f
}

#[test]
fn held_items() {
    // req:replica/taken-items@6f007a48
    let mut r = scratch(&held_fixture(), "111");
    let now = at(12, 0);
    branch_ref(
        &mut r,
        "spec/y",
        "sy",
        at(11, 0),
        Some(RefChange::Spec {
            change: Some(change_md("resolves: [b]\n")),
            deltas: vec!["web.md".into()],
        }),
    );
    branch_ref(&mut r, "propose/c", "pc", now - 3600, None);
    branch_ref(&mut r, "propose/d", "pd", now - 5 * 3600, None);
    r.recompute(now);
    let taken = &view(&r, "implementation")["taken"];
    assert_eq!(
        taken["changes"],
        json!([
            {"id": "x", "state": "main", "items": ["a"], "nodes": ["a"]},
            {"id": "y", "state": "proposed", "items": ["b"], "nodes": ["web"]},
        ])
    );
    assert_eq!(taken["claims"], json!([{"item": "c", "nodes": ["b"]}]));
}

#[test]
fn a_rename_branch_names_its_archived_change() {
    // req:replica/taken-items@6f007a48
    let mut r = scratch(&clm_repo(), "111");
    // The source read `changes/archive/r/` because the tip has no `changes/r/change.md`.
    branch_ref(
        &mut r,
        "spec/r",
        "sr",
        at(11, 0),
        Some(RefChange::Spec {
            change: Some(change_md("resolves: [e]\n")),
            deltas: vec!["web.md".into()],
        }),
    );
    // An autosync copy of it is the same change, and a nested name is no spec ref.
    branch_ref(&mut r, "autosync/spec/r", "ar", at(11, 5), None);
    branch_ref(&mut r, "spec/nested/z", "nz", at(11, 5), None);
    r.recompute(at(12, 0));
    let taken = &view(&r, "implementation")["taken"];
    assert_eq!(
        taken["changes"],
        json!([{"id": "r", "state": "proposed", "items": ["e"], "nodes": ["web"]}])
    );
    assert_eq!(taken["claims"], json!([]));
}

#[test]
fn taken_carries_the_recompute_time_and_expiry() {
    // req:replica/taken-items@6f007a48
    let mut r = scratch(&clm_repo(), "111");
    r.recompute(at(12, 0));
    let imp = view(&r, "implementation");
    assert_eq!(imp["now"], at(12, 0));
    assert_eq!(imp["expiry_hours"], 4);
    assert_eq!(imp["taken"], json!({"changes": [], "claims": []}));
}

// ---- flight and blockers ----

#[test]
fn blocked_by_a_need_on_a_claimed_component() {
    // req:replica/implementation-models@26aaf4c5
    let mut f = clm_repo();
    // `b` has one pending requirement, which needs `a/p`, uncited on `a`.
    f.insert(
        "specs/b/SPEC.md".into(),
        node("a", &["q"], "specd")
            .replacen("#### Scenario", "#### Needs\n- a/p\n\n#### Scenario", 1)
            .into_bytes(),
    );
    let now = at(12, 0);
    let mut r = scratch(&f, "111");
    branch_ref(&mut r, "impl/a", "sa", now - 3600, None);
    r.recompute(now);
    let imp = view(&r, "implementation");
    let nodes = imp["flight"]["nodes"].as_array().unwrap();
    let state = |id: &str| nodes.iter().find(|n| n["node"] == id).unwrap()["state"].clone();
    assert_eq!(state("a"), "claimed");
    assert_eq!(state("b"), "blocked");
    let b = nodes.iter().find(|n| n["node"] == "b").unwrap();
    assert_eq!(b["blocked_by"].as_array().unwrap().len(), 1);
    assert_eq!(b["blocked_by"][0]["requirement"], "a/p");
    let roots = imp["blockers"]["roots"].as_array().unwrap();
    assert_eq!(roots.len(), 1);
    assert_eq!(roots[0]["root"], "a");
    assert_eq!(roots[0]["weight"], 1);
    assert_eq!(roots[0]["holds"][0]["node"], "b");
    for text in [imp["flight"].to_string(), imp["blockers"].to_string()] {
        assert!(!text.contains("waiting"), "{text}");
    }
    // Both are the `view` component's models: the same as it computes for the same inputs.
    let dir = tempfile::tempdir().unwrap();
    write_checkout(dir.path(), &f);
    let (graph, _) = clm_model::load(dir.path());
    let refs = [clm_view::ClaimRef {
        node: "a".into(),
        tip_time: now - 3600,
        merged: false,
    }];
    let flight = clm_view::flight_view(&graph, &[], &refs, now, 4);
    assert_eq!(imp["flight"], serde_json::to_value(flight).unwrap());
}

#[test]
fn an_incomplete_index_gives_the_models_nothing_cited() {
    // req:replica/implementation-models@26aaf4c5
    let f = clm_repo();
    let mut r = Replica::default();
    r.apply(tree("111", &f));
    let spec: Vec<String> = r
        .pending_fetch()
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    r.apply(Change::Tip {
        sha: "111".into(),
        files: spec.iter().map(|p| file(p, &f[p])).collect(),
        removed: Vec::new(),
    });
    // A source file still to scan.
    let mut with_src = f.clone();
    with_src.insert("src.rs".into(), b"fn x() {}\n".to_vec());
    r.apply(tree("111", &with_src));
    r.recompute(at(12, 0));
    let imp = view(&r, "implementation");
    assert_eq!(imp["coverage"]["state"], "indexing");
    assert_eq!(imp["coverage"]["done"], 0.0);
}

fn proposed(id: &str, writer: &str, h: u64, m: u64, slug: &str) -> String {
    record(json!({
        "id": id, "kind": "proposed", "writer": writer, "at": stamp(h, m),
        "slug": slug, "title": format!("Title {slug}"), "body": "Body.", "nodes": ["a"],
    }))
}

#[test]
fn open_proposals_leave_out_filed_resolved_and_withdrawn() {
    // req:replica/open-proposals@53a076e1
    let mut f = clm_repo();
    f.extend(files(&[
        (
            "backlog/s2.md",
            "---\ntitle: S2\nkind: idea\ncomponents: [a]\n---\nBody.\n",
        ),
        (
            "changes/archive/old/change.md",
            &change_md("resolves: [s3]\n"),
        ),
        ("changes/live/change.md", &change_md("resolves: [s5]\n")),
    ]));
    let mut r = scratch(&f, "111");
    ledger(
        &mut r,
        "clm/ledger/alice/web",
        &[
            proposed("p1", "alice/web", 9, 0, "s1"),
            proposed("p2", "alice/web", 9, 1, "s2"),
            proposed("p3", "alice/web", 9, 2, "s3"),
            proposed("p4", "alice/web", 9, 3, "s4"),
            proposed("p5", "alice/web", 9, 5, "s5"),
            record(json!({
                "id": "w", "kind": "withdrawn", "writer": "alice/web", "at": stamp(9, 4),
                "proposal": "p4",
            })),
        ],
    );
    r.recompute(at(11, 0));
    let imp = view(&r, "implementation");
    assert_eq!(
        imp["proposals"],
        json!([{
            "id": "p1", "writer": "alice/web", "at": stamp(9, 0), "slug": "s1",
            "title": "Title s1", "body": "Body.", "nodes": ["a"],
        }])
    );
    let snap = view(&r, "snapshot");
    assert!(snap["resolved"].as_array().unwrap().contains(&json!("s3")));
}

#[test]
fn open_proposals_sort_by_time_then_id() {
    // req:replica/open-proposals@53a076e1
    let mut r = scratch(&clm_repo(), "111");
    ledger(
        &mut r,
        "clm/ledger/alice/web",
        &[
            proposed("p2", "alice/web", 9, 0, "b"),
            proposed("p1", "alice/web", 9, 0, "a"),
            proposed("p3", "alice/web", 8, 0, "c"),
        ],
    );
    r.recompute(at(11, 0));
    let imp = view(&r, "implementation");
    let ids: Vec<&str> = imp["proposals"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["p3", "p1", "p2"]);
}

#[test]
fn no_ledger_no_proposals() {
    // req:replica/open-proposals@53a076e1
    let mut r = scratch(&clm_repo(), "111");
    r.recompute(at(11, 0));
    assert_eq!(view(&r, "implementation")["proposals"], json!([]));
}
