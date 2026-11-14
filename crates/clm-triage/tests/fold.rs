//! `clm_triage::fold`: the prompts it writes.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;

use clm_triage::fold;
use common::*;

const RENAME_PROMPT: &str = ".clm/prompts/fold/20261027-r.md";

/// A rename change as `clm rename` leaves it on its own branch: archived, `done`, no live
/// `change.md`.
fn push_rename(r: &Repo, id: &str) {
    let text = "---\nstatus: done\nresolves: [x]\nrenames:\n  - term: {from: a, to: b}\n---\n";
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(&format!("changes/archive/{id}/change.md"), text)],
    );
}

fn push_change(r: &Repo, id: &str, status: &str) {
    push_files(
        &r.work,
        &format!("spec/{id}"),
        &[(&format!("changes/{id}/change.md"), &change(status))],
    );
}

fn ids(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

#[test]
fn scenario_rename_change_folded_alone() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    push_rename(&r, "20261027-r");
    let out = fold(&r.work, &ids(&["20261027-r"])).unwrap();
    assert!(out.left.is_empty(), "{:?}", out.left);
    assert!(out.launcher.is_some());
    let p = fs::read_to_string(r.work.join(RENAME_PROMPT)).unwrap();
    for needle in [
        "`x`",
        "git show origin/spec/20261027-r:changes/archive/20261027-r/change.md",
        "clm pr open spec/20261027-r",
        "proposed 20261027-r review <url>",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for needle in ["changes/20261027-r/change.md", "git switch -c"] {
        assert!(!p.contains(needle), "{needle} in {p}");
    }
}

#[test]
fn scenario_rename_change_beside_others_left() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    push_change(&r, "20261027-a", "applied");
    push_rename(&r, "20261027-r");
    let out = fold(&r.work, &ids(&["20261027-a", "20261027-r"])).unwrap();
    assert_eq!(
        out.left,
        [(
            "20261027-r".to_string(),
            "a rename change folds alone (rename-by-replay): run clm draft --fold 20261027-r"
                .to_string()
        )]
    );
    let p = fs::read_to_string(r.work.join(".clm/prompts/fold/20261027-a.md")).unwrap();
    assert!(p.contains("20261027-a") && !p.contains("20261027-r"), "{p}");
}

#[test]
fn scenario_rename_change_beside_an_unapplied_one_folded() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    push_change(&r, "20261027-p", "proposed");
    push_rename(&r, "20261027-r");
    let out = fold(&r.work, &ids(&["20261027-p", "20261027-r"])).unwrap();
    let left: Vec<&str> = out.left.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(left, ["20261027-p"]);
    let p = fs::read_to_string(r.work.join(RENAME_PROMPT)).unwrap();
    assert!(p.contains("proposed 20261027-r review <url>"), "{p}");
}

#[test]
fn scenario_two_rename_changes_left() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    push_rename(&r, "20261027-r");
    push_rename(&r, "20261027-s");
    let out = fold(&r.work, &ids(&["20261027-r", "20261027-s"])).unwrap();
    assert!(out.launcher.is_none());
    assert_eq!(out.left.len(), 2, "{:?}", out.left);
    for (_, why) in &out.left {
        assert!(why.starts_with("a rename change folds alone"), "{why}");
    }
}

#[test]
fn scenario_archived_change_without_renames_left() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    for (id, fm) in [
        ("20261027-q", "status: done\n"),
        ("20261027-e", "status: done\nrenames: []\n"),
        ("20261027-t", "status: applied\nrenames:\n  - term: x\n"),
    ] {
        push_files(
            &r.work,
            &format!("spec/{id}"),
            &[(
                &format!("changes/archive/{id}/change.md"),
                &format!("---\n{fm}---\n"),
            )],
        );
        let out = fold(&r.work, &ids(&[id])).unwrap();
        assert!(out.launcher.is_none());
        let [(left, why)] = &out.left[..] else {
            panic!("{:?}", out.left)
        };
        assert_eq!(left, id);
        assert!(!why.contains("folds alone"), "{why}");
    }
}

#[test]
fn a_live_change_is_judged_as_fold_says_whatever_the_archive_holds() {
    // req:orchestrator.triage/fold-rename-alone@2953a8eb
    let r = repo();
    push_files(
        &r.work,
        "spec/20261027-l",
        &[
            ("changes/20261027-l/change.md", &change("proposed")),
            (
                "changes/archive/20261027-l/change.md",
                "---\nstatus: done\nrenames:\n  - term: x\n---\n",
            ),
        ],
    );
    let out = fold(&r.work, &ids(&["20261027-l"])).unwrap();
    assert!(out.launcher.is_none());
    assert!(out.left[0].1.contains("not `applied`"), "{:?}", out.left);
}
