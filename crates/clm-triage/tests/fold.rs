//! `clm_triage::fold`: the prompts it writes.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;

use clm_triage::fold;
use common::*;

/// The fold prompt for `ids`, each pushed as `origin/spec/<id>` with an applied change.
fn fold_prompt(ids: &[&str]) -> String {
    let r = repo();
    for id in ids {
        push_files(
            &r.work,
            &format!("spec/{id}"),
            &[(&format!("changes/{id}/change.md"), &change("applied"))],
        );
    }
    let ids: Vec<String> = ids.iter().map(|s| s.to_string()).collect();
    assert!(fold(&r.work, &ids).unwrap().launcher.is_some());
    let file = r
        .work
        .join(".clm/prompts/fold")
        .join(format!("{}.md", ids[0]));
    fs::read_to_string(file).unwrap()
}

#[test]
fn scenario_draft_scratch_files_fold_prompt() {
    // req:orchestrator.triage/draft-scratch-files@e35b49c4
    for p in [
        fold_prompt(&["20261027-a"]),
        fold_prompt(&["20261027-a", "20261027-b"]),
    ] {
        let paras: Vec<&str> = p
            .split("\n\n")
            .filter(|s| s.contains("running unattended"))
            .collect();
        let [para] = paras[..] else {
            panic!("one paragraph with running unattended: {paras:?}");
        };
        for needle in [".clm/", "scratch", "scratchpad"] {
            assert!(para.contains(needle), "{needle} missing from {para}");
        }
    }
}

#[test]
fn scenario_builder_check_model_fold_prompt() {
    // req:orchestrator.triage/builder-check-model@4b8c1e8a
    for p in [
        fold_prompt(&["20261027-a"]),
        fold_prompt(&["20261027-a", "20261027-b"]),
    ] {
        for needle in [
            "model: \"sonnet\"",
            "never launch a subagent without naming its model",
        ] {
            assert!(p.contains(needle), "{needle} missing from {p}");
        }
    }
}

#[test]
fn scenario_builder_check_order_fold_prompt() {
    // req:orchestrator.triage/builder-check-order@7dc9565c
    let p = fold_prompt(&["20261027-a", "20261027-b"]);
    let para = p
        .split("\n\n")
        .find(|s| s.contains("Then the builder check"))
        .expect("builder check paragraph");
    for needle in [
        "builder-check-asks-order",
        "clm pending",
        "its own component",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

#[test]
fn scenario_draft_architecture_audit_fold_prompt() {
    // req:orchestrator.triage/draft-architecture-audit@4efa5dcb
    let p = fold_prompt(&["20261027-a", "20261027-b"]);
    let para = p
        .split("\n\n")
        .find(|s| s.contains("Then the builder check"))
        .expect("builder check paragraph");
    for needle in [
        "architecture-auditor",
        "your own model",
        "## Open questions",
        "spec-architecture-audit",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

#[test]
fn fold_prompt_names_the_human_task_kind() {
    // req:orchestrator.triage/fold@8b73785c
    let p = fold_prompt(&["20261027-a", "20261027-b"]);
    assert!(
        p.contains("A `human-task` item the fold adds is a burden"),
        "{p}"
    );
    assert!(
        p.contains("`question` or `human-task` backlog items"),
        "{p}"
    );
}

#[test]
fn scenario_fold_no_old_command_names() {
    // req:orchestrator.triage/fold@8b73785c
    for p in [
        fold_prompt(&["20261027-a"]),
        fold_prompt(&["20261027-a", "20261027-b"]),
    ] {
        let words: Vec<&str> = p
            .split(|c: char| !c.is_alphanumeric() && c != '-')
            .filter(|w| !w.is_empty())
            .collect();
        assert!(!words.contains(&"page"), "the word page is in {p}");
        for pair in words.windows(2) {
            assert!(
                !(pair[0] == "clm" && ["sync", "checkpoint", "page"].contains(&pair[1])),
                "old command name `clm {}` in {p}",
                pair[1]
            );
        }
    }
}

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

#[test]
fn scenario_draft_capability_test_fixes_fold_prompt() {
    // req:orchestrator.triage/draft-capability-test-fixes@daea8e6f
    let p = fold_prompt(&["20261027-a", "20261027-b"]);
    let para = p
        .split("\n\n")
        .find(|s| s.contains("Then the builder check"))
        .expect("builder check paragraph");
    for needle in ["capability-tests-follow-rules", "CAPABILITIES.md"] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
}

#[test]
fn scenario_prompts_push_through_clm_fold_prompt() {
    // req:orchestrator.triage/prompts-push-through-clm@0f8b1003
    let several = fold_prompt(&["20261027-a", "20261027-b"]);
    assert!(several.contains("clm branch push"), "{several}");
    for p in [several, fold_prompt(&["20261027-a"])] {
        for absent in ["git push", "--force-with-lease"] {
            assert!(!p.contains(absent), "{absent} in {p}");
        }
    }
}
