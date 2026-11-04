//! `clm claim backlog/<slug>` and `clm proposals` against a bare `origin` and a clone.

mod common;

use std::collections::BTreeSet;

use clm_orchestrator::{
    BareClaim, ClaimRef, Error, Holding, HoldingChange, SpecRef, claim, proposals, taken_from_refs,
    taken_items,
};
use common::*;

const C: &str = "20260101-c";
const D: &str = "20260101-d";

fn refused(res: Result<String, Error>) -> String {
    match res {
        Err(Error::Refused(m)) => m,
        other => panic!("{other:?}"),
    }
}

/// The repo with `backlog/q.md` (nodes `c`) on main.
fn with_q() -> Repo {
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q");
    r
}

/// `change.md` of a proposed change with `extra` frontmatter lines.
fn proposed(extra: &str) -> String {
    change_md("proposed", extra)
}

fn lines(r: &Repo, nodes: &[&str]) -> Vec<String> {
    let nodes: Vec<String> = nodes.iter().map(|n| n.to_string()).collect();
    proposals(&r.work, &nodes).unwrap().lines
}

#[test]
fn scenario_racing_proposers() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    let other = r.work.with_file_name("other");
    git(
        &r.work,
        &[
            "clone",
            "-q",
            r.origin.to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    identify(&other);
    let (w1, w2) = (r.work.clone(), other.clone());
    let t1 = std::thread::spawn(move || claim(&w1, "backlog/q", 4));
    let t2 = std::thread::spawn(move || claim(&w2, "backlog/q", 4));
    let results = [t1.join().unwrap(), t2.join().unwrap()];
    let wins = results.iter().filter(|r| r.is_ok()).count();
    assert_eq!(wins, 1, "{results:?}");
    for res in &results {
        match res {
            Ok(branch) => assert_eq!(branch, "propose/q"),
            Err(Error::Refused(m)) => assert!(m.contains("someone else"), "{m}"),
            other => panic!("{other:?}"),
        }
    }
    // The claim is a fresh commit on `origin/main`.
    let tip = remote_tip(&r, "propose/q").unwrap();
    let main = git(&r.origin, &["rev-parse", "main"]);
    assert_eq!(git(&r.origin, &["rev-parse", &format!("{tip}^")]), main);
}

#[test]
fn scenario_live_claim_refused() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    let sha = push_propose(&r.work, "q", 1);
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains("claimed by propose/q"), "{why}");
    assert_eq!(remote_tip(&r, "propose/q").unwrap(), sha);
}

#[test]
fn scenario_proposed_in_a_spec_branch() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    push_files(
        &r.work,
        &format!("spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("resolves: [q]\n"),
        )],
    );
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains(&format!("proposed in {C}")), "{why}");
    assert_eq!(remote_tip(&r, "propose/q"), None);
}

#[test]
fn scenario_proposed_only_in_an_autosync_snapshot() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    push_files(
        &r.work,
        &format!("autosync/spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("from: backlog/q\n"),
        )],
    );
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains(&format!("proposed in {C}")), "{why}");
}

#[test]
fn scenario_own_spec_work() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    let branch = format!("spec/{C}");
    push_files(
        &r.work,
        &branch,
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("resolves: [q]\n"),
        )],
    );
    git(
        &r.work,
        &["checkout", "-q", "-b", &branch, &format!("origin/{branch}")],
    );
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
    let first = remote_tip(&r, "propose/q").unwrap();
    // Its own live claim doesn't hold it either: the claim is renewed.
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
    assert_ne!(remote_tip(&r, "propose/q").unwrap(), first);
}

#[test]
fn expiry_decides_whether_a_claim_is_live() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    push_propose(&r.work, "q", 2);
    assert!(refused(claim(&r.work, "backlog/q", 4)).contains("claimed by propose/q"));
    assert_eq!(claim(&r.work, "backlog/q", 1).unwrap(), "propose/q");
}

#[test]
fn scenario_resolved_by_a_change_on_main() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    write(
        &r.work,
        &format!("changes/{D}/change.md"),
        &proposed("resolves: [q]\n"),
    );
    push_main(&r.work, "d");
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains(&format!("resolved by {D} on main")), "{why}");
    assert_eq!(remote_tip(&r, "propose/q"), None);
}

#[test]
fn scenario_merged_spec_work_no_longer_holds() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    push_files(
        &r.work,
        &format!("spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("from: backlog/q\n"),
        )],
    );
    // The archived change names `q` both ways; neither holds it.
    write(
        &r.work,
        &format!("changes/archive/{C}/change.md"),
        &change_md("done", "from: backlog/q\nresolves: [q]\n"),
    );
    push_main(&r.work, "archived c");
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");

    // Under `changes/<id>/` on main it is held by main, not by the branch.
    let r = with_q();
    push_files(
        &r.work,
        &format!("spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("from: backlog/q\n"),
        )],
    );
    write(&r.work, &format!("changes/{C}/change.md"), &proposed(""));
    push_main(&r.work, "c on main");
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
}

#[test]
fn scenario_lapsed_claim_taken_over() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = with_q();
    let old = push_propose(&r.work, "q", 5);
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
    let tip = remote_tip(&r, "propose/q").unwrap();
    assert_ne!(tip, old);
    assert_eq!(
        git(&r.origin, &["log", "-1", "--format=%s", &tip]).trim(),
        "Claim backlog/q"
    );
}

#[test]
fn scenario_unknown_item() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = repo();
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains("no such item"), "{why}");
    assert_eq!(remote_tip(&r, "propose/q"), None);
    // An item in the checkout only isn't one yet.
    write(&r.work, "backlog/q.md", &item("c"));
    assert!(refused(claim(&r.work, "backlog/q", 4)).contains("no such item"));
}

#[test]
fn scenario_unknown_frontmatter_keys() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = with_q();
    push_files(
        &r.work,
        "spec/20260101-f",
        &[(
            "changes/20260101-f/change.md",
            &proposed("resolves: [q]\nreviewers: [x]\n"),
        )],
    );
    let why = refused(claim(&r.work, "backlog/q", 4));
    assert!(why.contains("proposed in 20260101-f"), "{why}");
}

#[test]
fn scenario_nodes_before_deltas() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = with_q();
    push_files(
        &r.work,
        &format!("autosync/spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("from: backlog/q\n"),
        )],
    );
    assert_eq!(lines(&r, &["c"]), [format!("{C} proposed [c] q")]);
}

/// Change `d` on main resolving `r` (delta for `b`); `origin/spec/c` with a delta for `a`
/// and `resolves: [q]`, `q` on no node; `propose/s` 1 hour old, `s` on nodes `c`. The
/// spec's `gate`, `check` and `ci` are `b`, `a` and `c` here.
fn each_kind() -> Repo {
    let r = with_q();
    write(&r.work, "backlog/q.md", &item(""));
    write(&r.work, "backlog/r.md", &item("a"));
    write(&r.work, "backlog/s.md", &item("c"));
    write(
        &r.work,
        &format!("changes/{D}/change.md"),
        &proposed("resolves: [r]\n"),
    );
    write(
        &r.work,
        &format!("changes/{D}/deltas/b.md"),
        &delta("b", &["t"]),
    );
    push_main(&r.work, "d, r, s");
    push_files(
        &r.work,
        &format!("spec/{C}"),
        &[
            (
                &format!("changes/{C}/change.md"),
                &proposed("resolves: [q]\n"),
            ),
            (&format!("changes/{C}/deltas/a.md"), &delta("a", &["u"])),
        ],
    );
    push_propose(&r.work, "s", 1);
    r
}

#[test]
fn scenario_each_kind_of_line() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = each_kind();
    assert_eq!(
        lines(&r, &[]),
        [
            format!("{C} proposed [a] q"),
            format!("{D} main [b] r"),
            "backlog/s claimed [c]".to_string(),
        ]
    );
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|i| i.to_string()).collect()
}

#[test]
fn scenario_taken_items_same_as_proposals() {
    // req:orchestrator/taken-items@b29275d6
    let r = each_kind();
    // `q` on node `c` (the spec's `model`) joins its delta node `a` (the `check`).
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q on c");
    push_propose(&r.work, "q", 1);
    push_propose(&r.work, "t", 5);
    let changes = vec![
        HoldingChange {
            id: C.to_string(),
            state: Holding::Proposed,
            items: set(&["q"]),
            nodes: set(&["a", "c"]),
        },
        HoldingChange {
            id: D.to_string(),
            state: Holding::Main,
            items: set(&["r"]),
            nodes: set(&["b"]),
        },
    ];
    let s = BareClaim {
        item: "s".to_string(),
        nodes: set(&["c"]),
    };
    let taken = taken_items(&r.work, 4).unwrap();
    assert_eq!(taken.changes, changes);
    assert_eq!(taken.claims, std::slice::from_ref(&s));
    // The expiry given decides which claims are live: `t`, 5 hours old, is under 6.
    let longer = taken_items(&r.work, 6).unwrap();
    let t = BareClaim {
        item: "t".to_string(),
        nodes: set(&[]),
    };
    assert_eq!(longer.claims, [s, t]);
    // `clm proposals` prints exactly what `taken_items` returns at the default expiry.
    assert_eq!(
        lines(&r, &[]),
        [
            format!("{C} proposed [a,c] q"),
            format!("{D} main [b] r"),
            "backlog/s claimed [c]".to_string(),
        ]
    );
}

#[test]
fn scenario_taken_from_refs_same_as_taken_items() {
    // req:orchestrator/taken-from-refs@6c708d1b
    let r = each_kind();
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q on c");
    let e = "20260101-e";
    push_files(
        &r.work,
        &format!("autosync/spec/{e}"),
        &[(&format!("changes/{e}/change.md"), "# e\n")],
    );
    push_propose(&r.work, "q", 1);
    let hour_ago = now() - 3600;
    let refs = [
        SpecRef {
            name: format!("origin/spec/{C}"),
            change: Some(proposed("resolves: [q]\n")),
            deltas: vec!["a.md".to_string()],
        },
        SpecRef {
            name: format!("origin/autosync/spec/{e}"),
            change: Some("# e\n".to_string()),
            deltas: vec![],
        },
    ];
    let claims = ["s", "q"].map(|slug| ClaimRef {
        node: slug.to_string(),
        tip_time: hour_ago,
        merged: false,
    });
    let from_refs = taken_from_refs(&load(&r.work), &refs, &claims, now(), 4);
    assert_eq!(from_refs, taken_items(&r.work, 4).unwrap());
    let ids: Vec<(&str, Holding, &BTreeSet<String>)> = from_refs
        .changes
        .iter()
        .map(|c| (c.id.as_str(), c.state, &c.items))
        .collect();
    assert_eq!(
        ids,
        [
            (C, Holding::Proposed, &set(&["q"])),
            (D, Holding::Main, &set(&["r"])),
            (e, Holding::Proposed, &set(&[])),
        ]
    );
    assert_eq!(from_refs.changes[0].nodes, set(&["a", "c"]));
    assert_eq!(from_refs.changes[1].nodes, set(&["b"]));
    assert_eq!(
        from_refs.claims,
        [BareClaim {
            item: "s".to_string(),
            nodes: set(&["c"]),
        }]
    );
    assert_eq!(from_refs.warnings.len(), 1, "{:?}", from_refs.warnings);
    assert!(
        from_refs.warnings[0].contains(e),
        "{:?}",
        from_refs.warnings
    );
}

#[test]
fn taken_from_refs_skips_changes_on_main_and_refs_without_the_change() {
    // req:orchestrator/taken-from-refs@6c708d1b
    let r = with_q();
    write(
        &r.work,
        "changes/archive/20260101-m/change.md",
        &change("applied"),
    );
    push_main(&r.work, "m archived");
    let refs = [
        SpecRef {
            name: "spec/20260101-m".to_string(),
            change: Some(proposed("resolves: [q]\n")),
            deltas: vec![],
        },
        SpecRef {
            name: format!("spec/{C}"),
            change: None,
            deltas: vec!["a.md".to_string()],
        },
    ];
    // Nested branches name no change, whatever their last segment.
    let nested = [
        "origin/spec/foo-spec/bar",
        "origin/spec/x/spec/bar",
        "origin/myspec/bar",
    ]
    .map(|name| SpecRef {
        name: name.to_string(),
        change: Some(proposed("resolves: [q]\n")),
        deltas: vec![],
    });
    let refs: Vec<SpecRef> = refs.into_iter().chain(nested).collect();
    let taken = taken_from_refs(&load(&r.work), &refs, &[], now(), 4);
    assert_eq!(taken, Default::default());
}

#[test]
fn scenario_node_filter() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = each_kind();
    assert_eq!(
        lines(&r, &["c", "b"]),
        [
            format!("{D} main [b] r"),
            "backlog/s claimed [c]".to_string()
        ]
    );
    assert_eq!(lines(&r, &["b", "nope"]), [format!("{D} main [b] r")]);
    assert_eq!(lines(&r, &["nope"]), Vec::<String>::new());
}

#[test]
fn a_claim_a_listed_change_names_isnt_listed_again() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = each_kind();
    push_propose(&r.work, "r", 1);
    push_propose(&r.work, "q", 1);
    assert_eq!(
        lines(&r, &[]),
        [
            format!("{C} proposed [a] q"),
            format!("{D} main [b] r"),
            "backlog/s claimed [c]".to_string(),
        ]
    );
}

#[test]
fn both_refs_of_a_change_are_joined() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = with_q();
    write(&r.work, "backlog/r.md", &item("b"));
    push_main(&r.work, "r");
    push_files(
        &r.work,
        &format!("spec/{C}"),
        &[
            (
                &format!("changes/{C}/change.md"),
                &proposed("resolves: [q]\n"),
            ),
            (&format!("changes/{C}/deltas/a.md"), &delta("a", &["u"])),
        ],
    );
    push_files(
        &r.work,
        &format!("autosync/spec/{C}"),
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("from: backlog/r\nresolves: [q]\n"),
        )],
    );
    assert_eq!(lines(&r, &[]), [format!("{C} proposed [a,b,c] q r")]);
}

#[test]
fn a_resolved_item_already_gone_isnt_listed() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = repo();
    write(
        &r.work,
        &format!("changes/{D}/change.md"),
        &proposed("resolves: [gone]\n"),
    );
    push_main(&r.work, "d");
    assert_eq!(lines(&r, &[]), Vec::<String>::new());
}

#[test]
fn scenario_own_work_and_lapsed_claims_left_out() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = with_q();
    let branch = format!("spec/{C}");
    push_files(
        &r.work,
        &branch,
        &[(
            &format!("changes/{C}/change.md"),
            &proposed("resolves: [q]\n"),
        )],
    );
    git(
        &r.work,
        &["checkout", "-q", "-b", &branch, &format!("origin/{branch}")],
    );
    push_propose(&r.work, "q", 1);
    push_propose(&r.work, "t", 5);
    assert_eq!(lines(&r, &[]), Vec::<String>::new());
    // Another checkout sees the work and the live claim.
    git(&r.work, &["checkout", "-q", "main"]);
    assert_eq!(lines(&r, &[]), [format!("{C} proposed [c] q")]);
}

#[test]
fn scenario_item_without_nodes() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = repo();
    write(&r.work, "backlog/u.md", "---\ntitle: U\nkind: idea\n---\n");
    push_main(&r.work, "u");
    push_propose(&r.work, "u", 1);
    assert_eq!(lines(&r, &[]), ["backlog/u claimed []"]);
}

#[test]
fn scenario_unreadable_change() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = repo();
    push_files(
        &r.work,
        "spec/20260101-e",
        &[
            ("changes/20260101-e/change.md", "# e\n"),
            ("changes/20260101-e/deltas/b.md", &delta("b", &["t"])),
        ],
    );
    let out = proposals(&r.work, &[]).unwrap();
    assert_eq!(out.lines, ["20260101-e proposed [b]"]);
    assert_eq!(out.warnings.len(), 1, "{:?}", out.warnings);
    assert!(
        out.warnings[0].contains("origin/spec/20260101-e"),
        "{:?}",
        out.warnings
    );
}

#[test]
fn a_ref_without_the_change_is_skipped() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = with_q();
    push_files(&r.work, &format!("spec/{C}"), &[("notes.md", "n\n")]);
    assert_eq!(lines(&r, &[]), Vec::<String>::new());
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
}

const R: &str = "20260101-r";

/// `origin/spec/<R>` holding only the archived change, resolving `q`, with a `sync` delta.
fn push_rename_branch(r: &Repo) {
    push_files(
        &r.work,
        &format!("spec/{R}"),
        &[
            (
                &format!("changes/archive/{R}/change.md"),
                &change_md("done", "renames: []\nresolves: [q]\n"),
            ),
            (
                &format!("changes/archive/{R}/deltas/sync.md"),
                &delta("sync", &["u"]),
            ),
        ],
    );
}

#[test]
fn scenario_rename_branch_holds_its_items() {
    // req:orchestrator/archived-spec-work@bdfbe8d6
    let r = with_q();
    push_rename_branch(&r);
    assert_eq!(lines(&r, &[]), [format!("{R} proposed [c,sync] q")]);
    let m = refused(claim(&r.work, "backlog/q", 4));
    assert!(m.contains(R), "{m}");
    assert_eq!(remote_tip(&r, "propose/q"), None);
}

#[test]
fn scenario_landed_rename_holds_nothing() {
    // req:orchestrator/archived-spec-work@bdfbe8d6
    let r = with_q();
    push_rename_branch(&r);
    write(
        &r.work,
        &format!("changes/archive/{R}/change.md"),
        &change_md("done", "resolves: [q]\n"),
    );
    push_main(&r.work, "r");
    assert_eq!(lines(&r, &[]), Vec::<String>::new());
    assert_eq!(claim(&r.work, "backlog/q", 4).unwrap(), "propose/q");
}

#[test]
fn scenario_live_change_wins() {
    // req:orchestrator/archived-spec-work@bdfbe8d6
    let r = with_q();
    write(&r.work, "backlog/s.md", &item("c"));
    push_main(&r.work, "s");
    push_files(
        &r.work,
        "spec/20260101-c",
        &[
            ("changes/20260101-c/change.md", &proposed("resolves: [q]\n")),
            (
                "changes/archive/20260101-c/change.md",
                &change_md("done", "resolves: [s]\n"),
            ),
        ],
    );
    assert_eq!(lines(&r, &[]), ["20260101-c proposed [c] q"]);
}

#[test]
fn scenario_own_rename_branch() {
    // req:orchestrator/archived-spec-work@bdfbe8d6
    let r = with_q();
    push_rename_branch(&r);
    git(
        &r.work,
        &[
            "checkout",
            "-q",
            "-b",
            &format!("spec/{R}"),
            &format!("origin/spec/{R}"),
        ],
    );
    push_propose(&r.work, "q", 1);
    assert_eq!(lines(&r, &[]), Vec::<String>::new());
}
