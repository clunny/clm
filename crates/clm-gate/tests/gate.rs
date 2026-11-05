use std::fs;
use std::path::Path;
use std::process::Command;

use clm_gate::{Kind, evaluate_at};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(status.status.success(), "git {args:?}: {status:?}");
}

fn write(dir: &Path, rel: &str, content: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit_all(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", msg]);
}

const SPEC: &str = "---\ntitle: A\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n";

/// A repo with `main` holding a spec and some code, and HEAD on a fresh branch `pr`.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/a/SPEC.md", SPEC);
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    commit_all(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

fn rules(outcome: &clm_gate::Outcome) -> Vec<&str> {
    outcome.violations.iter().map(|v| v.rule).collect()
}

#[test]
fn mixed_pr_is_blocked() {
    // req:gate/classify@2825a73f
    let dir = repo();
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &SPEC.replace("c\n", "changed\n"),
    );
    write(dir.path(), "crates/x/src/lib.rs", "pub fn y() {}\n");
    commit_all(dir.path(), "both");

    let out = evaluate_at(dir.path(), "main", "HEAD", true).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
    assert_eq!(rules(&out), ["gate/classify"]);
}

#[test]
fn code_only_pr_is_code() {
    // req:gate/classify@2825a73f
    let dir = repo();
    write(dir.path(), "crates/x/src/lib.rs", "pub fn y() {}\n");
    commit_all(dir.path(), "code");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn every_spec_path_counts_as_spec() {
    // req:gate/classify@2825a73f
    let dir = repo();
    for rel in [
        "specs/b/SPEC.md",
        "changes/c/change.md",
        "docs/x.md",
        "ARCHITECTURE.md",
        "CLAUDE.md",
    ] {
        write(dir.path(), rel, "x\n");
    }
    commit_all(dir.path(), "spec");

    let out = evaluate_at(dir.path(), "main", "HEAD", true).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
}

#[test]
fn paths_git_would_quote_are_classified() {
    // req:gate/classify@2825a73f
    // `git diff --name-status` quotes non-ASCII paths unless `-z` is used.
    for file in ["docs/naïve.md", "specs/é/SPEC.md"] {
        let dir = repo();
        write(dir.path(), file, "x\n");
        commit_all(dir.path(), "quoted");
        let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
        assert_eq!(out.kind, Kind::Spec, "{file}");
    }
}

#[test]
fn status_only_spec_change_does_not_make_code_pr_mixed() {
    // req:gate/status-only-exemption@aa4f5b5c
    let dir = repo();
    write(dir.path(), "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &SPEC.replace("status: specd", "status: active"),
    );
    commit_all(dir.path(), "impl");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn status_plus_other_edit_is_a_spec_change() {
    // req:gate/status-only-exemption@aa4f5b5c
    let dir = repo();
    write(dir.path(), "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &SPEC
            .replace("status: specd", "status: active")
            .replace("c\n", "sneaky\n"),
    );
    commit_all(dir.path(), "impl");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
}

#[test]
fn unapproved_spec_pr_is_blocked() {
    // req:gate/approval@492f6a25
    let dir = repo();
    write(dir.path(), "ARCHITECTURE.md", "invariants\n");
    commit_all(dir.path(), "spec");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert_eq!(rules(&out), ["gate/approval"]);
    assert!(out.violations[0].message.contains("clm approve"));
}

#[test]
fn approved_spec_pr_passes() {
    // req:gate/approval@492f6a25
    let dir = repo();
    write(dir.path(), "ARCHITECTURE.md", "invariants\n");
    commit_all(dir.path(), "spec");

    let out = evaluate_at(dir.path(), "main", "HEAD", true).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

/// Commits `on_pr` on `pr` and `on_main` on `main`, leaving HEAD on `pr`, behind `main`.
fn diverge(d: &Path, on_pr: &[(&str, &str)], on_main: &[(&str, &str)]) {
    for (rel, text) in on_pr {
        write(d, rel, text);
    }
    commit_all(d, "pr");
    git(d, &["checkout", "-q", "main"]);
    for (rel, text) in on_main {
        write(d, rel, text);
    }
    commit_all(d, "main moved on");
    git(d, &["checkout", "-q", "pr"]);
}

#[test]
fn behind_branch_that_merges_cleanly_passes() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    diverge(
        dir.path(),
        &[("crates/b.rs", "b\n")],
        &[("crates/a.rs", "a\n")],
    );

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert!(rules(&out).is_empty(), "{:?}", out.violations);
    assert_eq!(out.kind, Kind::Code);
}

#[test]
fn code_file_changed_on_both_sides_that_merges_cleanly_passes() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    let d = dir.path();
    let base = "fn one() {}\n\nfn two() {}\n\nfn three() {}\n";
    git(d, &["checkout", "-q", "main"]);
    write(d, "crates/a.rs", base);
    commit_all(d, "a.rs");
    git(d, &["checkout", "-q", "-B", "pr"]);
    diverge(
        d,
        &[("crates/a.rs", &base.replace("one", "uno"))],
        &[("crates/a.rs", &base.replace("three", "tres"))],
    );

    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert!(rules(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn up_to_date_branch_has_no_violation() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    write(dir.path(), "crates/b.rs", "b\n");
    commit_all(dir.path(), "code");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert!(rules(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn conflicting_branch_is_blocked() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    diverge(
        dir.path(),
        &[("crates/x/src/lib.rs", "pub fn y() {}\n")],
        &[("crates/x/src/lib.rs", "pub fn z() {}\n")],
    );

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(rules(&out), ["gate/up-to-date"]);
    assert_eq!(out.violations[0].path, Path::new("main..HEAD"));
    assert!(out.violations[0].message.contains("clm rebase"));
}

const TWO_REQS: &str = "---\ntitle: A\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n\
    ## Requirements\n\n### Requirement: one\nThe system SHALL do one.\n\n\
    ### Requirement: two\nThe system SHALL do two.\n";

#[test]
fn spec_changed_on_both_sides_is_blocked_though_it_merges() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", TWO_REQS);
    commit_all(d, "two requirements");
    git(d, &["checkout", "-q", "-B", "pr"]);
    diverge(
        d,
        &[("specs/a/SPEC.md", &TWO_REQS.replace("do one", "do 1"))],
        &[("specs/a/SPEC.md", &TWO_REQS.replace("do two", "do 2"))],
    );

    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    let up: Vec<_> = out
        .violations
        .iter()
        .filter(|v| v.rule == "gate/up-to-date")
        .collect();
    assert_eq!(up.len(), 1, "{:?}", out.violations);
    assert_eq!(up[0].path, Path::new("specs/a/SPEC.md"));
    assert!(up[0].message.contains("clm rebase"));
}

#[test]
fn each_spec_file_changed_on_both_sides_is_reported_in_order() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", TWO_REQS);
    write(d, "specs/b/SPEC.md", TWO_REQS);
    commit_all(d, "two nodes");
    git(d, &["checkout", "-q", "-B", "pr"]);
    let (one, two) = (
        TWO_REQS.replace("do one", "do 1"),
        TWO_REQS.replace("do two", "do 2"),
    );
    diverge(
        d,
        &[("specs/b/SPEC.md", &one), ("specs/a/SPEC.md", &one)],
        &[("specs/b/SPEC.md", &two), ("specs/a/SPEC.md", &two)],
    );

    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    let up: Vec<_> = out
        .violations
        .iter()
        .filter(|v| v.rule == "gate/up-to-date")
        .map(|v| v.path.as_path())
        .collect();
    assert_eq!(
        up,
        [Path::new("specs/a/SPEC.md"), Path::new("specs/b/SPEC.md")]
    );
}

#[test]
fn specs_changed_in_different_files_are_not_blocked() {
    // req:gate/up-to-date@6cf6950e
    let dir = repo();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/b/SPEC.md", &SPEC.replace("title: A", "title: B"));
    commit_all(d, "node b");
    git(d, &["checkout", "-q", "-B", "pr"]);
    diverge(
        d,
        &[("specs/a/SPEC.md", &SPEC.replace("c\n", "changed\n"))],
        &[("specs/b/SPEC.md", &SPEC.replace("title: A", "title: B 2"))],
    );

    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert!(
        !rules(&out).contains(&"gate/up-to-date"),
        "{:?}",
        out.violations
    );
}

#[test]
fn removed_line_starting_with_dashes_is_not_mistaken_for_a_header() {
    // req:gate/status-only-exemption@aa4f5b5c
    let dir = repo();
    write(dir.path(), "crates/x/src/lib.rs", "pub fn y() {}\n");
    // Status flip plus deleting a line that begins with `--`.
    let edited = SPEC
        .replace("status: specd", "status: active")
        .replace("## Contract\nc\n", "## Contract\n");
    write(dir.path(), "specs/a/SPEC.md", &format!("{edited}-- note\n"));
    commit_all(dir.path(), "setup");
    git(dir.path(), &["checkout", "-q", "main"]);
    write(dir.path(), "specs/a/SPEC.md", &format!("{SPEC}-- note\n"));
    commit_all(dir.path(), "main has the note");
    git(dir.path(), &["checkout", "-q", "-b", "pr2"]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &SPEC.replace("status: specd", "status: active"),
    );
    write(dir.path(), "crates/x/src/lib.rs", "pub fn z() {}\n");
    commit_all(dir.path(), "status flip + delete `-- note`");

    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

/// `repo()` plus `changes/x/` (status applied) and an archived `changes/archive/old/` on `main`.
fn repo_with_changes() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\n---\n\n# x\n",
    );
    write(d, "changes/x/deltas/a.md", "---\ncomponent: a\n---\n");
    write(
        d,
        "changes/archive/old/change.md",
        "---\nstatus: done\n---\n",
    );
    write(d, "backlog/x.md", "---\ntitle: X\nkind: idea\n---\n");
    commit_all(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

fn archive_x(d: &Path, change_md: &str) {
    git(d, &["mv", "changes/x", "changes/archive/x"]);
    write(d, "changes/archive/x/change.md", change_md);
}

#[test]
fn impl_pr_may_finish_and_archive_its_change() {
    // req:gate/archive-move@4779ee7a
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, "---\nstatus: done\n---\n\n# x\n");
    commit_all(d, "impl + archive");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn archive_move_carries_plan_and_status_files() {
    // req:gate/archive-move@4779ee7a
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "changes/x/plan.md", "---\ntasks: []\n---\n");
    write(d, "changes/x/tasks/a.md", "---\nstatus: done\n---\n");
    commit_all(d, "plan");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, "---\nstatus: done\n---\n\n# x\n");
    commit_all(d, "finish");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn archive_move_with_other_edits_is_a_spec_change() {
    // req:gate/archive-move@4779ee7a
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, "---\nstatus: done\n---\n\n# x rewritten\n");
    commit_all(d, "impl + archive + edit");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
}

#[test]
fn editing_or_deleting_archived_files_is_blocked() {
    // req:gate/archive-immutable@e5a6c8c1
    let dir = repo_with_changes();
    let d = dir.path();
    write(
        d,
        "changes/archive/old/change.md",
        "---\nstatus: done\n---\nedited\n",
    );
    commit_all(d, "edit history");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert_eq!(rules(&out), ["gate/archive-immutable"]);
    assert_eq!(
        out.violations[0].path,
        Path::new("changes/archive/old/change.md")
    );

    git(d, &["rm", "-q", "changes/archive/old/change.md"]);
    commit_all(d, "delete history");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert_eq!(rules(&out), ["gate/archive-immutable"]);
}

#[test]
fn code_pr_may_add_backlog_items() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "backlog/found-bug.md",
        "---\ntitle: Found\nkind: bug\n---\n",
    );
    commit_all(d, "impl + defer");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Code, false));
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn editing_or_deleting_backlog_items_is_a_spec_change() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(d, "backlog/x.md", "---\ntitle: X2\nkind: idea\n---\n");
    commit_all(d, "impl + edit item");
    assert_eq!(
        evaluate_at(d, "main", "HEAD", false).unwrap().kind,
        Kind::Mixed
    );

    git(d, &["checkout", "-q", "main"]);
    git(d, &["checkout", "-q", "-b", "pr2"]);
    git(d, &["rm", "-q", "backlog/x.md"]);
    commit_all(d, "delete item");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, false));
}

/// `repo_with_changes()` with `backlog/x.md` holding `blocks: [a/x]` on `main`; HEAD on `pr`
/// with a code change.
fn repo_with_blocks() -> TempDir {
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/x]\n---\nBody.\n",
    );
    commit_all(d, "block");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    dir
}

#[test]
fn scenario_impl_pr_adds_a_block() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_blocks();
    let d = dir.path();
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/x, a/y]\n---\nBody.\n",
    );
    commit_all(d, "impl + block a/y");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn a_block_added_to_an_item_without_blocks_is_code() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks:\n  - a/y\n---\n",
    );
    commit_all(d, "impl + first block");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn scenario_impl_pr_removes_a_block() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_blocks();
    let d = dir.path();
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: []\n---\nBody.\n",
    );
    commit_all(d, "impl + unblock");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(
        (out.kind, rules(&out)),
        (Kind::Mixed, vec!["gate/classify"])
    );
}

#[test]
fn a_block_added_with_another_edit_is_a_spec_change() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_blocks();
    let d = dir.path();
    for text in [
        "---\ntitle: X2\nkind: idea\nblocks: [a/x, a/y]\n---\nBody.\n",
        "---\ntitle: X\nkind: idea\nblocks: [a/x, a/y]\n---\nNew body.\n",
        "---\ntitle: X\nkind: idea\nblocks: [a/y, a/z]\n---\nBody.\n",
        "---\ntitle: X\nkind: idea\nblocks: [a/x]\npriority: P2\n---\nBody.\n",
        // A repeated entry adds nothing.
        "---\ntitle: X\nkind: idea\nblocks: [a/x, a/x]\n---\nBody.\n",
    ] {
        git(d, &["checkout", "-q", "-B", "pr", "main"]);
        write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
        write(d, "backlog/x.md", text);
        commit_all(d, "impl + edit");
        assert_eq!(
            evaluate_at(d, "main", "HEAD", false).unwrap().kind,
            Kind::Mixed,
            "{text}"
        );
    }
}

#[test]
fn reordering_blocks_adds_nothing_so_it_is_a_spec_change() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/x, a/y]\n---\n",
    );
    commit_all(d, "blocks");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/y, a/x]\n---\n",
    );
    commit_all(d, "impl + reorder");
    assert_eq!(
        evaluate_at(d, "main", "HEAD", false).unwrap().kind,
        Kind::Mixed
    );
}

#[test]
fn a_block_added_to_a_nested_backlog_file_is_a_spec_change() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "backlog/sub/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/x]\n---\n",
    );
    commit_all(d, "nested");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "backlog/sub/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/x, a/y]\n---\n",
    );
    commit_all(d, "impl + nested block");
    assert_eq!(
        evaluate_at(d, "main", "HEAD", false).unwrap().kind,
        Kind::Mixed
    );
}

/// `repo_with_changes()` where change `x` resolves `backlog/x.md`, plus an unrelated `backlog/y.md`.
fn repo_with_resolves() -> TempDir {
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [x]\n---\n\n# x\n",
    );
    write(d, "backlog/y.md", "---\ntitle: Y\nkind: idea\n---\n");
    commit_all(d, "resolves");
    git(d, &["checkout", "-q", "-B", "pr"]);
    dir
}

const X_DONE: &str = "---\nstatus: done\nresolves: [x]\n---\n\n# x\n";

#[test]
fn impl_pr_may_delete_the_items_its_change_resolves() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_resolves();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, X_DONE);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "impl + archive + resolve");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn impl_pr_may_not_delete_unrelated_items() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_resolves();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, X_DONE);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    fs::remove_file(d.join("backlog/y.md")).unwrap();
    commit_all(d, "impl + archive + delete unrelated");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn impl_pr_may_not_edit_the_items_its_change_resolves() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_resolves();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, X_DONE);
    write(d, "backlog/x.md", "---\ntitle: X edited\nkind: idea\n---\n");
    commit_all(d, "impl + archive + edit resolved item");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn task_pr_may_delete_an_item_a_live_change_resolves() {
    // req:gate/backlog-additions@85a57cc9
    // req:gate/resolves-named-items@84abf9a3 (item deleted by a task)
    let dir = repo_with_resolves();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "delete without archiving");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

const BUG_X: &str = "---\ntitle: X\nkind: bug\ncomponents: [a]\n---\nBroken.\n";

/// `main` with root and node `a` whose `paths` are `["crates/a/**"]`, code in `crates/a`
/// and `crates/b`, and `backlog/x.md` holding `item`; HEAD on a fresh branch `pr`.
fn repo_with_bug(item: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    );
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n## Contract\nc\n",
    );
    write(d, "crates/a/src/lib.rs", "pub fn a() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b() {}\n");
    write(d, "backlog/x.md", item);
    commit_all(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

/// Applies `edits`, deletes `backlog/x.md`, and commits with `trailer` closing the message.
fn fix_with_trailer(dir: &TempDir, edits: &[Edit], trailer: &str) -> clm_gate::Outcome {
    let d = dir.path();
    for (rel, content) in edits {
        match content {
            Some(c) => write(d, rel, c),
            None => fs::remove_file(d.join(rel)).unwrap(),
        }
    }
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", &format!("fix x\n\n{trailer}\n")]);
    evaluate_at(d, "main", "HEAD", false).unwrap()
}

const FIX_A: &[Edit] = &[("crates/a/src/lib.rs", Some("pub fn a2() {}\n"))];

#[test]
fn scenario_code_fix_deletes_its_bug() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn scenario_code_fix_outside_the_bugs_nodes() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(
        &dir,
        &[("crates/b/src/lib.rs", Some("pub fn b2() {}\n"))],
        "Resolves: backlog/x",
    );
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn scenario_trailer_naming_an_idea() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(&BUG_X.replace("kind: bug", "kind: idea"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn scenario_trailer_naming_deferred_work() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(&BUG_X.replace("kind: bug", "kind: deferred"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn scenario_trailer_naming_a_question() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(&BUG_X.replace("kind: bug", "kind: question"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn trailer_naming_a_human_item_is_mixed() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(&BUG_X.replace("kind: bug", "kind: human-task"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn scenario_trailer_naming_another_item() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/y");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn deleting_a_code_file_of_the_bugs_node_counts() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(
        &dir,
        &[("crates/a/src/lib.rs", None)],
        "Resolves: backlog/x",
    );
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn trailer_value_must_be_exactly_backlog_slug() {
    // req:gate/backlog-additions@85a57cc9
    for trailer in [
        "Resolves: x",
        "Resolves: backlog/x.md",
        "Resolves: backlog/sub/x",
        "Resolves: see backlog/x",
    ] {
        let dir = repo_with_bug(BUG_X);
        let out = fix_with_trailer(&dir, FIX_A, trailer);
        assert_eq!(out.kind, Kind::Mixed, "{trailer}");
    }
    // Surrounding blanks are trimmed; several trailers may each name an item.
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/y\nResolves:   backlog/x  ");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn trailer_in_an_earlier_commit_of_the_branch_counts() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_bug(BUG_X);
    let d = dir.path();
    write(d, "crates/a/src/lib.rs", "pub fn a2() {}\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "fix x\n\nResolves: backlog/x\n"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "drop item");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn trailer_on_the_base_side_does_not_count() {
    // req:gate/backlog-additions@85a57cc9
    // A trailer in a commit the merge base already holds names nothing for this branch.
    let dir = repo_with_bug(BUG_X);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "crates/b/src/lib.rs", "pub fn b3() {}\n");
    git(d, &["add", "-A"]);
    git(d, &["commit", "-qm", "unrelated\n\nResolves: backlog/x\n"]);
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/a/src/lib.rs", "pub fn a2() {}\n");
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "fix without trailer");
    assert_eq!(
        evaluate_at(d, "main", "HEAD", false).unwrap().kind,
        Kind::Mixed
    );
}

#[test]
fn bug_items_that_do_not_qualify_are_not_exempt() {
    // req:gate/backlog-additions@85a57cc9
    for item in [
        // frontmatter that doesn't parse
        "---\ntitle: X\nkind: bug\ncomponents: [a\n---\n",
        // a node that doesn't exist at the merge base
        "---\ntitle: X\nkind: bug\ncomponents: [zz]\n---\n",
        // no nodes
        "---\ntitle: X\nkind: bug\n---\n",
        // no frontmatter
        "Just text.\n",
    ] {
        let dir = repo_with_bug(item);
        let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
        assert_eq!(out.kind, Kind::Mixed, "{item}");
    }
}

#[test]
fn a_spec_file_matching_the_nodes_paths_does_not_count() {
    // req:gate/backlog-additions@85a57cc9
    // Only code files (outside the spec paths) count; a status-only spec edit is no fix.
    let dir = repo_with_bug(&BUG_X.replace("components: [a]", "components: [b]"));
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "specs/b/SPEC.md",
        "---\ntitle: B\nstatus: specd\nowners: []\npaths: [\"specs/b/**\"]\n---\n\n## Contract\nc\n",
    );
    commit_all(d, "node b");
    git(d, &["checkout", "-q", "-B", "pr"]);
    let out = fix_with_trailer(
        &dir,
        &[
            ("crates/a/src/lib.rs", Some("pub fn a2() {}\n")),
            (
                "specs/b/SPEC.md",
                Some(
                    "---\ntitle: B\nstatus: active\nowners: []\npaths: [\"specs/b/**\"]\n---\n\n## Contract\nc\n",
                ),
            ),
        ],
        "Resolves: backlog/x",
    );
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

// Fixes count where cited (`fixes-count-where-cited`). Markers are built in pieces, so this
// file cites nothing (`dangling-markers`).
const CITES_A: &str = concat!("// req", ":a/y@1234abcd\n#[test]\nfn t() {}\n");
const CITES_A_B: &str = concat!("// req", ":a.b/y@1234abcd\n#[test]\nfn t() {}\n");
const TEST_T: &str = "crates/b/tests/t.rs";

/// `repo_with_bug(item)` with `crates/b/tests/t.rs` holding `test` on `main`.
fn repo_with_test(item: &str, test: &[u8]) -> TempDir {
    let dir = repo_with_bug(item);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    fs::create_dir_all(d.join("crates/b/tests")).unwrap();
    fs::write(d.join(TEST_T), test).unwrap();
    commit_all(d, "a test kept under b");
    git(d, &["checkout", "-q", "-B", "pr"]);
    dir
}

#[test]
fn scenario_code_fix_to_a_test_that_cites_the_bugs_node() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_test(BUG_X, CITES_A.as_bytes());
    let edited = format!("{CITES_A}fn helper() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn scenario_code_fix_to_a_test_that_cites_a_child_of_the_bugs_node() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_test(BUG_X, CITES_A_B.as_bytes());
    let edited = format!("{CITES_A_B}fn helper() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn a_marker_on_either_side_counts() {
    // req:gate/backlog-additions@85a57cc9
    // The fix removes the file's last marker: the merge base's text still counts.
    let dir = repo_with_test(BUG_X, CITES_A.as_bytes());
    let out = fix_with_trailer(
        &dir,
        &[(TEST_T, Some("fn t() {}\n"))],
        "Resolves: backlog/x",
    );
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
    // The fix adds a test that cites the node: `HEAD`'s text counts.
    let dir = repo_with_bug(BUG_X);
    let out = fix_with_trailer(
        &dir,
        &[("crates/b/tests/new.rs", Some(CITES_A))],
        "Resolves: backlog/x",
    );
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
    // The fix deletes the citing test.
    let dir = repo_with_test(BUG_X, CITES_A.as_bytes());
    let out = fix_with_trailer(&dir, &[(TEST_T, None)], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn a_file_that_is_not_utf8_is_searched_as_bytes() {
    // req:gate/backlog-additions@85a57cc9
    let mut test = b"\xff\xfe latin-1 \xe9\n".to_vec();
    test.extend_from_slice(CITES_A.as_bytes());
    let dir = repo_with_test(BUG_X, &test);
    let d = dir.path();
    let mut edited = test.clone();
    edited.extend_from_slice(b"\xe9\n");
    fs::write(d.join(TEST_T), &edited).unwrap();
    let out = fix_with_trailer(&dir, &[], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn a_side_that_cannot_be_read_holds_no_marker() {
    // req:gate/backlog-additions@85a57cc9
    // A submodule's commit isn't in this repository, so neither side of it can be read;
    // the citing test still counts and the gate doesn't fail.
    let dir = repo_with_test(BUG_X, CITES_A.as_bytes());
    let d = dir.path();
    let link = |id: &str| format!("160000,{id},crates/b/sub");
    git(d, &["checkout", "-q", "main"]);
    git(
        d,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &link(&"1".repeat(40)),
        ],
    );
    git(d, &["commit", "-qm", "submodule"]);
    git(d, &["checkout", "-q", "-B", "pr"]);
    git(d, &["update-index", "--cacheinfo", &link(&"2".repeat(40))]);
    let edited = format!("{CITES_A}fn helper() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn a_node_without_paths_qualifies_through_markers() {
    // req:gate/backlog-additions@85a57cc9
    // The root `clm` has no `paths` in this repo.
    let cites_root = concat!("// req", ":clm/y@1234abcd\n");
    let dir = repo_with_test(
        &BUG_X.replace("components: [a]", "components: [clm]"),
        cites_root.as_bytes(),
    );
    let edited = format!("{cites_root}fn t() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn a_marker_of_a_node_missing_at_the_merge_base_does_not_count() {
    // req:gate/backlog-additions@85a57cc9
    let cites_zz = concat!("// req", ":zz/y@1234abcd\n");
    let dir = repo_with_test(
        &BUG_X.replace("components: [a]", "components: [zz]"),
        cites_zz.as_bytes(),
    );
    let edited = format!("{cites_zz}fn t() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
    // A missing node doesn't stop the others from counting.
    let dir = repo_with_test(
        &BUG_X.replace("components: [a]", "components: [zz, a]"),
        CITES_A.as_bytes(),
    );
    let edited = format!("{CITES_A}fn helper() {{}}\n");
    let out = fix_with_trailer(&dir, &[(TEST_T, Some(&edited))], "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

/// The `gate/resolves-named-items` violations of `out`.
fn unnamed(out: &clm_gate::Outcome) -> Vec<&clm_gate::Violation> {
    out.violations
        .iter()
        .filter(|v| v.rule == "gate/resolves-named-items")
        .collect()
}

#[test]
fn entries_listed_at_the_merge_base_are_not_tested() {
    // req:gate/resolves-named-items@84abf9a3 (item deleted by a task)
    let dir = repo_with_resolves();
    let d = dir.path();
    // An earlier task deleted the item it fixed; a later PR edits the change.
    git(d, &["checkout", "-q", "main"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "earlier task deletes backlog/x.md");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(d, "changes/x/change.md", X_DONE);
    commit_all(d, "finish without archiving");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn scenario_typo_in_a_new_change() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\nresolves: [nope]\n---\n\n# c\n",
    );
    commit_all(d, "new change with a typo");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    let found = unnamed(&out);
    assert_eq!(found.len(), 1, "{:?}", out.violations);
    assert_eq!(found[0].path, Path::new("changes/c/change.md"));
    assert!(found[0].message.contains("`c`"), "{}", found[0].message);
    assert!(found[0].message.contains("`nope`"), "{}", found[0].message);
}

#[test]
fn a_non_ascii_slug_names_its_item() {
    // req:gate/resolves-named-items@84abf9a3
    // Names are read unquoted, whatever `core.quotePath` says.
    let dir = repo_with_resolves();
    let d = dir.path();
    write(
        d,
        "backlog/caf\u{e9}.md",
        "---\ntitle: C\nkind: idea\n---\n",
    );
    write(
        d,
        "changes/c/change.md",
        "---\nstatus: proposed\nresolves: [\"caf\u{e9}\"]\n---\n\n# c\n",
    );
    commit_all(d, "new change resolving a new item");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert!(unnamed(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn scenario_entry_written_as_a_claim_path() {
    // req:gate/resolves-named-items@84abf9a3
    // `backlog/y.md` exists, yet none of these names an item.
    for entry in [
        "backlog/y",
        "backlog/y.md",
        "y.md",
        "backlog/backlog/y",
        "a/b",
    ] {
        let dir = repo_with_resolves();
        let d = dir.path();
        write(
            d,
            "changes/c/change.md",
            &format!("---\nstatus: proposed\nresolves: [\"{entry}\"]\n---\n\n# c\n"),
        );
        commit_all(d, "new change");
        let out = evaluate_at(d, "main", "HEAD", false).unwrap();
        let found = unnamed(&out);
        assert_eq!(found.len(), 1, "{entry}: {:?}", out.violations);
        assert!(found[0].message.contains("`c`"), "{}", found[0].message);
        assert!(
            found[0].message.contains(&format!("`{entry}`")),
            "{}",
            found[0].message
        );
    }
}

#[test]
fn typo_in_a_code_pr_is_reported() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [x, nope]\n---\n\n# x\n",
    );
    commit_all(d, "code plus a typo");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(unnamed(&out).len(), 1, "{:?}", out.violations);
}

#[test]
fn scenario_typo_added_to_a_live_change() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    // With `backlog/x.md` already deleted by a task, testing `x` again would report it.
    git(d, &["checkout", "-q", "main"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "a task deletes backlog/x.md");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves:\n  - x\n  - nope\n---\n\n# x\n",
    );
    commit_all(d, "two-line resolves");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    let found = unnamed(&out);
    assert_eq!(found.len(), 1, "{:?}", out.violations);
    assert!(found[0].message.contains("`nope`"), "{}", found[0].message);
    assert!(
        found[0].message.starts_with("change `x` resolves `nope`"),
        "{}",
        found[0].message
    );
}

#[test]
fn scenario_item_deleted_in_the_same_pr() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_changes();
    let d = dir.path();
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [x]\n---\n\n# x\n",
    );
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "resolve + delete");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert!(unnamed(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn archived_changes_are_not_tested() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "a task deletes backlog/x.md");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    archive_x(d, X_DONE);
    commit_all(d, "finish and archive");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Code, "{:?}", out.violations);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn scenario_typo_in_a_change_archived_in_its_spec_pr() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    write(
        d,
        "changes/archive/c/change.md",
        "---\nstatus: done\nresolves: [nope]\n---\n\n# c\n",
    );
    commit_all(d, "change archived in its spec PR, with a typo");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    let found = unnamed(&out);
    assert_eq!(found.len(), 1, "{:?}", out.violations);
    assert_eq!(found[0].path, Path::new("changes/archive/c/change.md"));
    assert!(found[0].message.contains("`c`"), "{}", found[0].message);
    assert!(found[0].message.contains("`nope`"), "{}", found[0].message);
    assert!(rules(&out).contains(&"gate/resolves-named-items"));
}

#[test]
fn scenario_live_change_moved_to_the_archive_with_edits() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_resolves();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "a task deletes backlog/x.md");
    git(d, &["checkout", "-q", "-B", "pr"]);
    archive_x(d, &X_DONE.replace("# x\n", "## Intent\nReworded.\n"));
    commit_all(d, "archive with an edited intent");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert_eq!(out.kind, Kind::Spec, "{:?}", out.violations);
    assert!(unnamed(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn a_new_archived_change_adds_entries_beyond_its_live_version() {
    // req:gate/resolves-named-items@84abf9a3
    // Moved with edits, adding `nope`: only the added entry is tested.
    let dir = repo_with_resolves();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "a task deletes backlog/x.md");
    git(d, &["checkout", "-q", "-B", "pr"]);
    archive_x(d, &X_DONE.replace("[x]", "[x, nope]"));
    commit_all(d, "archive adding an entry");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    let found = unnamed(&out);
    assert_eq!(found.len(), 1, "{:?}", out.violations);
    assert!(found[0].message.contains("`nope`"), "{}", found[0].message);
    assert!(
        !found[0].message.contains("resolves `x`"),
        "{}",
        found[0].message
    );
}

#[test]
fn nested_files_are_not_backlog_items() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_changes();
    let d = dir.path();
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [sub/z]\n---\n\n# x\n",
    );
    write(d, "backlog/sub/z.md", "---\ntitle: Z\nkind: idea\n---\n");
    commit_all(d, "resolve a nested file");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert_eq!(unnamed(&out).len(), 1, "{:?}", out.violations);
}

#[test]
fn item_added_in_the_same_pr_counts() {
    // req:gate/resolves-named-items@84abf9a3
    let dir = repo_with_changes();
    let d = dir.path();
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [z]\n---\n\n# x\n",
    );
    write(d, "backlog/z.md", "---\ntitle: Z\nkind: idea\n---\n");
    commit_all(d, "resolve a new item");
    let out = evaluate_at(d, "main", "HEAD", true).unwrap();
    assert!(unnamed(&out).is_empty(), "{:?}", out.violations);
}

#[test]
fn resolves_added_in_the_same_pr_dont_count() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    write(
        d,
        "changes/x/change.md",
        "---\nstatus: applied\nresolves: [x]\n---\n\n# x\n",
    );
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "resolve + delete in one go");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn archived_changes_resolves_dont_count() {
    // req:gate/backlog-additions@85a57cc9
    let dir = repo_with_changes();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "changes/archive/old/change.md",
        "---\nstatus: done\nresolves: [x]\n---\n",
    );
    commit_all(d, "history resolves x");
    git(d, &["checkout", "-q", "-B", "pr"]);
    write(d, "crates/x/src/lib.rs", "pub fn y() {}\n");
    fs::remove_file(d.join("backlog/x.md")).unwrap();
    commit_all(d, "delete item only history resolves");
    let out = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

// Review (`needs-review`).

const NODE: &str = "---\ntitle: A\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nR.\n\n#### Scenario: s\n- ok\n";
const ADDED_REQ: &str = "\n### Requirement: r2\nR2.\n\n#### Scenario: s\n- ok\n";

/// `main` holds `files` plus some code; HEAD is on a fresh branch `pr`.
fn tier_repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "crates/x/src/lib.rs", "pub fn x() {}\n");
    for (rel, content) in files {
        write(d, rel, content);
    }
    commit_all(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

/// A file and its new content, or `None` to delete it.
type Edit<'a> = (&'a str, Option<&'a str>);

/// Writes (`Some`) or deletes (`None`) each file, commits and evaluates the gate.
fn tier_after(dir: &TempDir, edits: &[Edit], approved: bool) -> clm_gate::Outcome {
    let d = dir.path();
    for (rel, content) in edits {
        match content {
            Some(c) => write(d, rel, c),
            None => fs::remove_file(d.join(rel)).unwrap(),
        }
    }
    commit_all(d, "edit");
    evaluate_at(d, "main", "HEAD", approved).unwrap()
}

/// `(kind, needs review, passes)` of an outcome.
fn verdict(out: &clm_gate::Outcome) -> (Kind, bool, bool) {
    (out.kind, out.review, out.violations.is_empty())
}

#[test]
fn spec_pr_that_needs_no_review_needs_no_approval() {
    // req:gate/approval@492f6a25
    let dir = tier_repo(&[("specs/a/SPEC.md", NODE)]);
    let edited = NODE.replace("\nc\n", "\nnew contract\n");
    let out = tier_after(&dir, &[("specs/a/SPEC.md", Some(&edited))], false);
    assert_eq!(
        verdict(&out),
        (Kind::Spec, false, true),
        "{:?}",
        out.violations
    );
}

/// A `proposed` `change.md` with `extra` frontmatter lines, such as `human_burdens: [b]\n`.
fn change_md(extra: &str) -> String {
    format!("---\nstatus: proposed\n{extra}---\n\n## Intent\nx\n")
}

fn delta(sections: &str) -> String {
    format!("---\ncomponent: a\n---\n\n{sections}")
}

/// `repo_with_bug(BUG_X)` with `main` then edited by `on_main` (`None` deletes the file),
/// and the code fix of `scenario_code_fix_deletes_its_bug` on a fresh `pr`.
fn fix_after_main(on_main: &[Edit]) -> clm_gate::Outcome {
    let dir = repo_with_bug(BUG_X);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    for (rel, content) in on_main {
        match content {
            Some(c) => write(d, rel, c),
            None => fs::remove_file(d.join(rel)).unwrap(),
        }
    }
    commit_all(d, "main moved on");
    git(d, &["checkout", "-q", "-B", "pr"]);
    fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x")
}

/// Node `a` of `repo_with_bug`.
const A_NODE: &str =
    "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\n---\n\n## Contract\nc\n";

#[test]
fn scenario_specs_marked_export_ignore() {
    // req:gate/base-specs-as-checkout@a340ed25
    let out = fix_after_main(&[(".gitattributes", Some("specs/** export-ignore\n"))]);
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn scenario_a_spec_marked_export_subst() {
    // req:gate/base-specs-as-checkout@a340ed25
    let node = A_NODE.replace("## Contract\nc\n", "## Contract\nc $Format:%H$\n");
    let out = fix_after_main(&[
        (".gitattributes", Some("specs/** export-subst\n")),
        ("specs/a/SPEC.md", Some(&node)),
    ]);
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn branch_without_specs_at_the_merge_base() {
    // req:gate/base-specs-as-checkout@a340ed25
    // `main` has no `specs/`, which loads empty: node `a` is missing, so the fix isn't exempt.
    let out = fix_after_main(&[("specs/SPEC.md", None), ("specs/a/SPEC.md", None)]);
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn scenario_key_declared_on_main() {
    // req:gate/declared-keys-loaded@1c77f6ae
    let coloured = A_NODE.replace("owners: []\n", "owners: []\ncolour: blue\n");
    let out = fix_after_main(&[
        (
            "changes/y/change.md",
            Some("---\nstatus: applied\nfrontmatter_keys: [colour]\n---\n\n# y\n"),
        ),
        ("specs/a/SPEC.md", Some(&coloured)),
    ]);
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[cfg(unix)]
#[test]
fn a_change_named_by_bytes_that_are_not_utf8_is_skipped() {
    // req:gate/base-specs-as-checkout@a340ed25
    use std::os::unix::ffi::OsStrExt;
    let dir = repo_with_bug(BUG_X);
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    let change = d
        .join("changes")
        .join(std::ffi::OsStr::from_bytes(b"caf\xe9"));
    fs::create_dir_all(&change).unwrap();
    fs::write(change.join("change.md"), change_md("")).unwrap();
    commit_all(d, "a change git can't name in UTF-8");
    git(d, &["checkout", "-q", "-B", "pr"]);
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn archived_changes_alone_load_no_change_files() {
    // req:gate/base-specs-as-checkout@a340ed25
    // `main` has neither `specs/` nor a live change, only the archive: nothing to extract.
    let out = fix_after_main(&[
        ("specs/SPEC.md", None),
        ("specs/a/SPEC.md", None),
        (
            "changes/archive/old/change.md",
            Some("---\nstatus: done\n---\n\n# old\n"),
        ),
    ]);
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}");
    String::from_utf8(out.stdout).unwrap()
}

fn rev_parse(dir: &Path, rev: &str) -> String {
    git_out(dir, &["rev-parse", rev]).trim().to_string()
}

#[test]
fn scenario_another_commit() {
    // req:gate/evaluate-at@0bfe14a1
    let dir = tier_repo(&[("specs/a/SPEC.md", NODE)]);
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &format!("{NODE}{ADDED_REQ}"));
    commit_all(d, "add a requirement");
    let s = rev_parse(d, "HEAD");
    git(d, &["checkout", "-q", "main"]);
    write(d, "ARCHITECTURE.md", "uncommitted\n");
    let (head, status) = (rev_parse(d, "HEAD"), git_out(d, &["status", "--porcelain"]));

    let out = evaluate_at(d, "main", &s, false).unwrap();
    assert_eq!(
        verdict(&out),
        (Kind::Spec, false, true),
        "{:?}",
        out.violations
    );
    assert_eq!(rev_parse(d, "HEAD"), head);
    assert_eq!(git_out(d, &["status", "--porcelain"]), status);
    assert_eq!(git_out(d, &["diff", "--cached"]), "");
}

#[test]
fn scenario_a_merge_commit_against_its_first_parent() {
    // req:gate/evaluate-at@0bfe14a1
    let dir = tier_repo(&[("specs/a/SPEC.md", NODE)]);
    let d = dir.path();
    write(d, "ARCHITECTURE.md", "invariants\n");
    commit_all(d, "architecture");
    git(d, &["checkout", "-q", "main"]);
    git(d, &["merge", "-q", "--no-ff", "-m", "land", "pr"]);
    let (m, p) = (rev_parse(d, "HEAD"), rev_parse(d, "HEAD^1"));

    let out = evaluate_at(d, &p, &m, false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    let approval = out
        .violations
        .iter()
        .find(|v| v.rule == "gate/approval")
        .expect("gate/approval");
    assert_eq!(approval.path, Path::new(&format!("{p}..{m}")));
}

const POLICY: &str = "# who approves\napproval = \"other\"\n\n[land]\ncheck = \"a\"\n";

/// The verdict on a branch from a `main` whose `clm.toml` is `base` (none when `None`).
fn policy_after(base: Option<&str>, edits: &[Edit], approved: bool) -> clm_gate::Outcome {
    let files: Vec<(&str, &str)> = base.map(|b| ("clm.toml", b)).into_iter().collect();
    tier_after(&tier_repo(&files), edits, approved)
}

#[test]
fn scenario_code_branch_weakens_the_policy() {
    // req:gate/approval-setting@c9f7ac40
    let weakened = POLICY.replace("\"other\"", "\"any\"");
    let out = policy_after(Some(POLICY), &[("clm.toml", Some(&weakened))], false);
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert_eq!(rules(&out), ["gate/approval"]);
}

#[test]
fn scenario_the_gate_agrees() {
    // req:gate/approval-changed@bd437cee
    let weakened = POLICY.replace("\"other\"", "\"any\"");
    let out = policy_after(Some(POLICY), &[("clm.toml", Some(&weakened))], false);
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert!(clm_gate::approval_changed(
        Some(POLICY.as_bytes()),
        Some(weakened.as_bytes())
    ));
}

#[test]
fn scenario_policy_set_where_it_was_unset() {
    // req:gate/approval-setting@c9f7ac40
    let unset = "[land]\ncheck = \"a\"\n";
    let set = format!("approval = \"other\"\n{unset}");
    let out = policy_after(Some(unset), &[("clm.toml", Some(&set))], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn scenario_approved_policy_change() {
    // req:gate/approval-setting@c9f7ac40
    let weakened = POLICY.replace("\"other\"", "\"any\"");
    let out = policy_after(Some(POLICY), &[("clm.toml", Some(&weakened))], true);
    assert_eq!(
        verdict(&out),
        (Kind::Spec, true, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn scenario_policy_change_with_code() {
    // req:gate/approval-setting@c9f7ac40
    let weakened = POLICY.replace("\"other\"", "\"any\"");
    let out = policy_after(
        Some(POLICY),
        &[
            ("clm.toml", Some(&weakened)),
            ("crates/x/src/lib.rs", Some("pub fn y() {}\n")),
        ],
        false,
    );
    assert_eq!(out.kind, Kind::Mixed);
    assert!(rules(&out).contains(&"gate/classify"));
}

#[test]
fn scenario_other_setting_changed() {
    // req:gate/approval-setting@c9f7ac40
    let edited = POLICY
        .replace("# who approves", "# who may approve")
        .replace("check = \"a\"", "check = \"b\"");
    let out = policy_after(
        Some(POLICY),
        &[
            ("clm.toml", Some(&edited)),
            ("crates/x/src/lib.rs", Some("pub fn y() {}\n")),
        ],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Code, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn scenario_unparseable_clm_toml() {
    // req:gate/approval-setting@c9f7ac40
    let out = policy_after(
        Some(POLICY),
        &[("clm.toml", Some("approval = \"other\"\n[land\n"))],
        false,
    );
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn editing_an_unparseable_clm_toml_is_a_spec_change() {
    // req:gate/approval-setting@c9f7ac40
    let out = policy_after(
        Some("approval = \"other\"\n[land\n"),
        &[("clm.toml", Some("approval = \"any\"\n[land\n"))],
        false,
    );
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn deleting_a_clm_toml_that_set_the_policy_is_a_spec_change() {
    // req:gate/approval-setting@c9f7ac40
    let out = policy_after(Some(POLICY), &[("clm.toml", None)], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn adding_a_clm_toml_without_the_policy_is_code() {
    // req:gate/approval-setting@c9f7ac40
    let out = policy_after(
        None,
        &[
            ("clm.toml", Some("[land]\ncheck = \"a\"\n")),
            ("crates/x/src/lib.rs", Some("pub fn y() {}\n")),
        ],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Code, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn a_clm_toml_that_isnt_utf8_is_a_spec_change() {
    // req:gate/approval-setting@c9f7ac40
    let dir = tier_repo(&[("clm.toml", POLICY)]);
    fs::write(
        dir.path().join("clm.toml"),
        b"approval = \"other\"\n# \xff\n",
    )
    .unwrap();
    let out = tier_after(&dir, &[], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn a_value_config_would_reject_is_compared_as_a_value() {
    // req:gate/approval-setting@c9f7ac40
    let same = policy_after(
        Some("approval = 1\n"),
        &[
            ("clm.toml", Some("# one\napproval = 1\n")),
            ("crates/x/src/lib.rs", Some("pub fn y() {}\n")),
        ],
        false,
    );
    assert_eq!(same.kind, Kind::Code, "{:?}", same.violations);
    let changed = policy_after(
        Some("approval = 1\n"),
        &[("clm.toml", Some("approval = 2\n"))],
        false,
    );
    assert_eq!((changed.kind, changed.review), (Kind::Spec, true));
}

#[test]
fn evaluate_at_reads_changes_and_the_policy_at_the_head() {
    // req:gate/evaluate-at@0bfe14a1
    // `HEAD` stays on `main`; the head commit adds a change with a burden and, separately,
    // a policy.
    let dir = tier_repo(&[("specs/a/SPEC.md", NODE), ("clm.toml", POLICY)]);
    let d = dir.path();
    write(d, "changes/x/change.md", &change_md("human_burdens: [b]\n"));
    write(
        d,
        "changes/x/deltas/a.md",
        &delta(&format!("## ADDED Requirements\n{ADDED_REQ}")),
    );
    commit_all(d, "change");
    let change = rev_parse(d, "HEAD");
    write(d, "clm.toml", &POLICY.replace("\"other\"", "\"any\""));
    commit_all(d, "policy");
    let policy = rev_parse(d, "HEAD");
    git(d, &["checkout", "-q", "main"]);

    let out = evaluate_at(d, "main", &change, false).unwrap();
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
    assert_eq!(rules(&out), ["gate/approval"]);
    let out = evaluate_at(d, &change, &policy, false).unwrap();
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
}

#[test]
fn a_nested_clm_toml_is_not_the_policy() {
    // req:gate/approval-setting@c9f7ac40
    let out = policy_after(
        None,
        &[("sub/clm.toml", Some("approval = \"any\"\n"))],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Code, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn architecture_needs_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[("ARCHITECTURE.md", "invariants\n")]);
    let out = tier_after(&dir, &[("ARCHITECTURE.md", Some("changed\n"))], true);
    assert_eq!(
        verdict(&out),
        (Kind::Spec, true, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn capabilities_file_needs_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[("CAPABILITIES.md", Some("# Capabilities\n"))],
        false,
    );
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
    assert_eq!(rules(&out), ["gate/approval"]);
}

#[test]
fn trusted_keys_need_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[("docs/allowed_signers", "alice ssh-ed25519 AAAA\n")]);
    let out = tier_after(
        &dir,
        &[(
            "docs/allowed_signers",
            Some("alice ssh-ed25519 AAAA\ndana ssh-ed25519 BBBB\n"),
        )],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Spec, true, false),
        "{:?}",
        out.violations
    );
    assert_eq!(rules(&out), ["gate/approval"]);
}

#[test]
fn trusted_keys_need_review_however_edited() {
    // req:gate/needs-review@3276bfca
    // A line that reads as a status edit, a deletion, and a directory in the file's place.
    let keys = "alice ssh-ed25519 AAAA\n";
    let status_line = format!("{keys}status: ssh-ed25519 BBBB\n");
    for (before, after) in [
        (
            vec![("docs/allowed_signers", keys)],
            vec![("docs/allowed_signers", Some(status_line.as_str()))],
        ),
        (
            vec![("docs/allowed_signers", keys)],
            vec![("docs/allowed_signers", None)],
        ),
        (vec![], vec![("docs/allowed_signers/x", Some(keys))]),
    ] {
        let dir = tier_repo(&before);
        let out = tier_after(&dir, &after, false);
        assert_eq!(
            verdict(&out),
            (Kind::Spec, true, false),
            "{after:?}: {:?}",
            out.violations
        );
    }
}

#[test]
fn approval_setting_needs_review() {
    // req:gate/needs-review@3276bfca
    let weakened = POLICY.replace("\"other\"", "\"any\"");
    let out = policy_after(Some(POLICY), &[("clm.toml", Some(&weakened))], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn burdens_need_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[]);
    let change = change_md("human_burdens: [a paid model run on every land]\n");
    let out = tier_after(&dir, &[("changes/x/change.md", Some(&change))], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
    assert_eq!(rules(&out), ["gate/approval"]);
}

#[test]
fn burdens_of_a_modified_change_need_review() {
    // req:gate/needs-review@3276bfca
    let change = change_md("human_burdens: [b]\n");
    let dir = tier_repo(&[("changes/x/change.md", &change)]);
    let edited = change.replace("\nx\n", "\nx, reworded\n");
    let out = tier_after(&dir, &[("changes/x/change.md", Some(&edited))], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn burdens_of_a_new_archived_change_need_review() {
    // req:gate/needs-review@3276bfca
    let dir = tier_repo(&[]);
    let change = "---\nstatus: done\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    let out = tier_after(
        &dir,
        &[("changes/archive/x/change.md", Some(change))],
        false,
    );
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn unparsable_change_needs_review() {
    // req:gate/needs-review@3276bfca
    for text in ["---\nstatus: [\n---\n", "no frontmatter\n"] {
        let dir = tier_repo(&[]);
        let out = tier_after(&dir, &[("changes/x/change.md", Some(text))], false);
        assert_eq!(verdict(&out), (Kind::Spec, true, false), "{text}");
    }
    let dir = tier_repo(&[]);
    fs::create_dir_all(dir.path().join("changes/x")).unwrap();
    fs::write(
        dir.path().join("changes/x/change.md"),
        b"---\nstatus: \xff\n---\n",
    )
    .unwrap();
    let out = tier_after(&dir, &[], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn contract_change_needs_no_review() {
    // req:gate/needs-review@3276bfca
    let node = format!("{NODE}{ADDED_REQ}");
    let dir = tier_repo(&[("specs/a/SPEC.md", &node)]);
    let out = tier_after(
        &dir,
        &[
            (
                "specs/a/SPEC.md",
                Some(&NODE.replace("\nc\n", "\nnew contract\n")),
            ),
            (
                "docs/decisions/d.md",
                Some("---\nstatus: accepted\n---\n\nD.\n"),
            ),
            (
                "changes/x/change.md",
                Some("---\nstatus: applied\n---\n\n## Intent\nx\n"),
            ),
        ],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Spec, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn review_blocked_without_approval() {
    // req:gate/approval@492f6a25
    let dir = tier_repo(&[]);
    let out = tier_after(&dir, &[("ARCHITECTURE.md", Some("x\n"))], false);
    assert_eq!(rules(&out), ["gate/approval"]);
}

#[test]
fn review_passes_with_approval() {
    // req:gate/approval@492f6a25
    let dir = tier_repo(&[]);
    let out = tier_after(&dir, &[("ARCHITECTURE.md", Some("x\n"))], true);
    assert_eq!(
        verdict(&out),
        (Kind::Spec, true, true),
        "{:?}",
        out.violations
    );
}

const HUMAN_ITEM: &str = "---\ntitle: H\nkind: human-task\n---\nMake a key.\n";

#[test]
fn undeclared_human_item_blocked() {
    // req:gate/undeclared-burden@aba2c63f
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            ("changes/x/change.md", Some(&change_md(""))),
            ("backlog/h.md", Some(HUMAN_ITEM)),
        ],
        true,
    );
    assert_eq!(rules(&out), ["gate/undeclared-burden"]);
    assert_eq!(out.violations[0].path, Path::new("backlog/h.md"));
}

#[test]
fn declared_human_item_passes() {
    // req:gate/undeclared-burden@aba2c63f
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            (
                "changes/x/change.md",
                Some(&change_md(
                    "human_burdens: [a key only the human can make]\n",
                )),
            ),
            ("backlog/h.md", Some(HUMAN_ITEM)),
        ],
        true,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Spec, true, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn a_code_branch_may_add_a_human_item() {
    // req:gate/undeclared-burden@aba2c63f
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            ("crates/x/src/lib.rs", Some("pub fn y() {}\n")),
            ("backlog/h.md", Some(HUMAN_ITEM)),
        ],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Code, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn status_only_edit_of_a_change_with_burdens_needs_review() {
    // req:gate/needs-review@3276bfca
    // req:gate/needs-review-of@988ab216
    // The status-only edit isn't a spec change, but it modifies a file of the change, so the
    // change's frontmatter reaches `needs_review_of` as the Worker's does.
    let change = "---\nstatus: proposed\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    let dir = tier_repo(&[("changes/x/change.md", change)]);
    let flipped = change.replace("status: proposed", "status: applied");
    let out = tier_after(
        &dir,
        &[
            ("changes/x/change.md", Some(&flipped)),
            ("docs/decisions/d.md", Some("# d\n")),
        ],
        false,
    );
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn deltas_of_a_change_with_burdens_need_review() {
    // req:gate/needs-review@3276bfca
    let change = "---\nstatus: applied\nhuman_burdens: [b]\n---\n\n## Intent\nx\n";
    let dir = tier_repo(&[
        ("changes/x/change.md", change),
        (
            "changes/x/deltas/a.md",
            "---\ncomponent: a\n---\n\n## ADDED Requirements\n",
        ),
    ]);
    let edited =
        "---\ncomponent: a\n---\n\n## ADDED Requirements\n\n### Requirement: r\nIt SHALL.\n";
    let out = tier_after(&dir, &[("changes/x/deltas/a.md", Some(edited))], false);
    assert_eq!(verdict(&out), (Kind::Spec, true, false));
}

#[test]
fn scenario_a_bug_filed_under_the_new_key() {
    // req:gate/item-components@dd579233
    let dir = repo_with_bug(&BUG_X.replace("nodes:", "components:"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!((out.kind, rules(&out)), (Kind::Code, vec![]));
}

#[test]
fn an_item_setting_both_keys_is_not_exempt() {
    // req:gate/item-components@dd579233
    let dir = repo_with_bug(&BUG_X.replace("components: [a]", "nodes: [a]\ncomponents: [a]"));
    let out = fix_with_trailer(&dir, FIX_A, "Resolves: backlog/x");
    assert_eq!(out.kind, Kind::Mixed, "{:?}", out.violations);
}

#[test]
fn scenario_the_old_words() {
    // req:gate/undeclared-burden@aba2c63f
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            ("changes/x/change.md", Some(&change_md(""))),
            ("backlog/h.md", Some(HUMAN_ITEM)),
        ],
        true,
    );
    assert_eq!(rules(&out), ["gate/undeclared-burden"]);
    let message = &out.violations[0].message;
    assert!(
        message.contains("human-task") && message.contains("human_burdens"),
        "{message}"
    );
    let old_item = HUMAN_ITEM.replace("human-task", "human");
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            ("changes/x/change.md", Some(&change_md(""))),
            ("backlog/h.md", Some(&old_item)),
        ],
        true,
    );
    assert_eq!(rules(&out), ["gate/undeclared-burden"]);
    let dir = tier_repo(&[]);
    let out = tier_after(
        &dir,
        &[
            (
                "changes/x/change.md",
                Some(&change_md("burdens: [a key only the human can make]\n")),
            ),
            ("backlog/h.md", Some(&old_item)),
        ],
        true,
    );
    assert_eq!((out.kind, out.review), (Kind::Spec, true));
    assert!(rules(&out).is_empty(), "{:?}", out.violations);
}

const STAMPED: &str = "approval = \"other\"\nupgraded = \"0.17.0\"\n[land]\ncheck = \"a\"\n";

#[test]
fn scenario_notes_taken() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let dir = tier_repo(&[("CLAUDE.md", "old\n"), ("clm.toml", STAMPED)]);
    let stamped = "approval = \"other\"\nupgraded = \"0.18.0\"\ndeclined_migrations = [\"m\"]\n[land]\ncheck = \"a\"\n";
    let out = tier_after(
        &dir,
        &[("CLAUDE.md", Some("new\n")), ("clm.toml", Some(stamped))],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Spec, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn the_stamp_alone_classifies_as_spec_without_review() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let dir = tier_repo(&[("clm.toml", STAMPED)]);
    let stamped = STAMPED.replace("0.17.0", "0.18.0");
    let out = tier_after(&dir, &[("clm.toml", Some(&stamped))], false);
    assert_eq!(
        verdict(&out),
        (Kind::Spec, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn scenario_mechanical_migrations_taken() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let dir = tier_repo(&[(".gitignore", "a\n"), ("clm.toml", STAMPED)]);
    let stamped = STAMPED.replace("0.17.0", "0.18.0");
    let out = tier_after(
        &dir,
        &[(".gitignore", Some("a\nb\n")), ("clm.toml", Some(&stamped))],
        false,
    );
    assert_eq!(
        verdict(&out),
        (Kind::Code, false, true),
        "{:?}",
        out.violations
    );
}

#[test]
fn scenario_the_stamp_and_another_key() {
    // req:gate/upgrade-stamp-left-out@5b8d7f0d
    let dir = tier_repo(&[("CLAUDE.md", "old\n"), ("clm.toml", STAMPED)]);
    let edited = STAMPED
        .replace("0.17.0", "0.18.0")
        .replace("check = \"a\"", "check = \"b\"");
    let out = tier_after(
        &dir,
        &[("CLAUDE.md", Some("new\n")), ("clm.toml", Some(&edited))],
        false,
    );
    assert_eq!(out.kind, Kind::Mixed);
    assert!(rules(&out).contains(&"gate/classify"));
}
