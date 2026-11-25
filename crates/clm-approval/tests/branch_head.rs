//! A branch's head, by one rule for `clm land` and `clm approve`, and its change IDs
//! (land.approval/branch-head).

mod common;

use std::fs;

use clm_approval::BranchHead;
use common::{Fixture, refused};

/// Commits `file` on `branch` from the `other` clone and pushes it to `origin`; returns the
/// commit.
fn push_from_other(f: &Fixture, branch: &str, file: &str) -> String {
    let other = f.path("other");
    f.git(&other, &["fetch", "-q", "origin"]);
    let start = match f.git(&other, &["ls-remote", "--heads", "origin", branch]) {
        s if s.is_empty() => "origin/main".to_string(),
        _ => format!("origin/{branch}"),
    };
    f.git(&other, &["checkout", "-q", "-B", branch, &start]);
    fs::write(other.join(file), format!("{file}\n")).unwrap();
    f.git(&other, &["add", "-A"]);
    f.git(&other, &["commit", "-q", "-m", file]);
    f.git(&other, &["push", "-q", "origin", &format!("HEAD:{branch}")]);
    f.git(&other, &["rev-parse", "HEAD"])
}

/// Commits `file` on the work clone's local `branch`, unpushed; returns the commit.
fn commit_local(f: &Fixture, branch: &str, file: &str) -> String {
    let work = f.work();
    f.git(&work, &["checkout", "-q", branch]);
    f.commit_files(&[(file, "local\n")], file);
    let sha = f.git(&work, &["rev-parse", "HEAD"]);
    f.git(&work, &["checkout", "-q", "main"]);
    sha
}

fn rev(f: &Fixture, r: &str) -> String {
    f.git(&f.work(), &["rev-parse", r])
}

/// The head for an approve and for a land.
fn both(f: &Fixture, branch: &str) -> [Result<BranchHead, clm_approval::Error>; 2] {
    f.with_repo(|repo| {
        [
            repo.branch_head(branch, false),
            repo.branch_head(branch, true),
        ]
    })
}

fn head(r: &Result<BranchHead, clm_approval::Error>) -> (String, Option<String>) {
    let h = r.as_ref().unwrap();
    (h.head.clone(), h.origin.clone())
}

#[test]
fn at_origins_tip() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    f.branch("spec/x", &[("x.txt", "x\n")]);
    let tip = rev(&f, "origin/spec/x");
    for r in &both(&f, "spec/x") {
        assert_eq!(head(r), (tip.clone(), Some(tip.clone())));
    }
}

#[test]
fn ahead_of_origin_either_name() {
    // req:land.approval/branch-head@781206cc
    for name in ["impl/a", "fix-x"] {
        let f = Fixture::new("");
        f.branch(name, &[("x.txt", "x\n")]);
        let tip = rev(&f, &format!("origin/{name}"));
        let local = commit_local(&f, name, "more.txt");
        for r in &both(&f, name) {
            assert_eq!(head(r), (local.clone(), Some(tip.clone())), "{name}");
        }
    }
}

#[test]
fn behind_origin() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    f.branch("spec/x", &[("x.txt", "x\n")]);
    let tip = push_from_other(&f, "spec/x", "theirs.txt");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let [approve, land] = both(&f, "spec/x");
    assert_eq!(head(&approve), (tip.clone(), Some(tip)));
    let why = refused(land);
    assert!(why.contains("`spec/x`"), "{why}");
    assert!(why.contains("behind `origin/spec/x`"), "{why}");
    assert!(why.contains("bring it up to `origin/spec/x`"), "{why}");
    assert!(!why.contains("diverged"), "{why}");
}

#[test]
fn diverged_from_origin() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    f.branch("spec/x", &[("x.txt", "x\n")]);
    let tip = push_from_other(&f, "spec/x", "theirs.txt");
    let local = commit_local(&f, "spec/x", "ours.txt");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let [approve, land] = both(&f, "spec/x");
    assert_eq!(head(&approve), (local, Some(tip)));
    let why = refused(land);
    assert!(why.contains("`spec/x`"), "{why}");
    assert!(why.contains("diverged from `origin/spec/x`"), "{why}");
    assert!(why.contains("rebase it onto `origin/spec/x`"), "{why}");
    assert!(why.contains("merge `origin/spec/x` into it"), "{why}");
    assert!(!why.contains("behind"), "{why}");
}

#[test]
fn no_local_branch() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    let tip = push_from_other(&f, "spec/x", "theirs.txt");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    for r in &both(&f, "spec/x") {
        assert_eq!(head(r), (tip.clone(), Some(tip.clone())));
    }
}

#[test]
fn a_branch_origin_doesnt_hold_either_name() {
    // req:land.approval/branch-head@781206cc
    for name in ["spec/y", "fix-y"] {
        let f = Fixture::new("");
        let work = f.work();
        f.git(&work, &["branch", name, "main"]);
        let local = commit_local(&f, name, "y.txt");
        for r in &both(&f, name) {
            assert_eq!(head(r), (local.clone(), None), "{name}");
        }
    }
}

#[test]
fn a_pruned_tracking_ref() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    f.branch("spec/x", &[("x.txt", "x\n")]);
    let local = rev(&f, "spec/x");
    let other = f.path("other");
    f.git(&other, &["push", "-q", "origin", "--delete", "spec/x"]);
    f.git(&f.work(), &["fetch", "-q", "--prune", "origin"]);
    for r in &both(&f, "spec/x") {
        assert_eq!(head(r), (local.clone(), None));
    }
}

#[test]
fn a_name_neither_has() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    for r in both(&f, "nope") {
        let why = refused(r);
        assert!(why.contains("`nope`"), "{why}");
    }
}

#[test]
fn a_branchs_change_ids() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    let work = f.work();
    f.commit_files(
        &[
            ("changes/a/change.md", "a\n"),
            ("changes/c/change.md", "c\n"),
            ("changes/old/change.md", "old\n"),
        ],
        "changes on main",
    );
    f.git(&work, &["push", "-q", "origin", "main"]);
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["checkout", "-q", "-b", "spec/z"]);
    f.git(&work, &["rm", "-q", "changes/c/change.md"]);
    // A whole renamed directory, which git's rename detection would list as `R`.
    f.git(&work, &["mv", "changes/old", "changes/new"]);
    f.commit_files(
        &[
            ("changes/b/change.md", "b\n"),
            ("changes/a/change.md", "a, modified\n"),
            ("changes/archive/d/change.md", "d\n"),
            ("changes/e/notes/change.md", "e\n"),
            ("changes/a-b/change.md", "a-b\n"),
        ],
        "spec/z",
    );
    let head = f.git(&work, &["rev-parse", "HEAD"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let ids = f.with_repo(|repo| repo.changes(&head)).unwrap();
    assert_eq!(ids, ["a", "a-b", "b", "new"]);
}

/// A `change.md` whose type changes names its change, and a non-ASCII ID comes through
/// whole, whatever the user's `diff.relative`.
#[cfg(unix)]
#[test]
fn a_type_change_and_a_non_ascii_id() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    let work = f.work();
    f.commit_files(&[("changes/t/change.md", "t\n")], "t on main");
    f.git(&work, &["push", "-q", "origin", "main"]);
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["config", "diff.relative", "true"]);
    f.git(&work, &["checkout", "-q", "-b", "spec/z"]);
    fs::remove_file(work.join("changes/t/change.md")).unwrap();
    std::os::unix::fs::symlink("../../a.txt", work.join("changes/t/change.md")).unwrap();
    f.commit_files(&[("changes/café/change.md", "c\n")], "spec/z");
    let head = f.git(&work, &["rev-parse", "HEAD"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let ids = f.with_repo(|repo| repo.changes(&head)).unwrap();
    assert_eq!(ids, ["café", "t"]);
}

/// The paths are read relative to a CLM root that is a subdirectory of the repository.
#[test]
fn change_ids_under_a_subdirectory_root() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    let work = f.work();
    f.commit_files(&[("s/clm.toml", "")], "s on main");
    f.git(&work, &["push", "-q", "origin", "main"]);
    f.git(&work, &["fetch", "-q", "origin"]);
    f.git(&work, &["checkout", "-q", "-b", "spec/z"]);
    f.commit_files(
        &[
            ("s/changes/x/change.md", "x\n"),
            ("changes/y/change.md", "y\n"),
        ],
        "spec/z",
    );
    let head = f.git(&work, &["rev-parse", "HEAD"]);
    f.git(&work, &["checkout", "-q", "main"]);
    let env = f.isolated_env();
    let root = fs::canonicalize(work.join("s")).unwrap();
    let settings = clm_config::load(Some(&root), &env).unwrap();
    let ids = clm_approval::Repo::new(&root, &env, &settings)
        .changes(&head)
        .unwrap();
    assert_eq!(ids, ["x"]);
}

#[test]
fn the_fingerprint_is_against_the_base_given() {
    // req:land.approval/branch-head@781206cc
    let f = Fixture::new("");
    f.branch("spec/x", &[("x.txt", "x\n")]);
    let fp = f.fingerprint("spec/x");
    let base = rev(&f, "origin/main");
    let tip = rev(&f, "origin/spec/x");
    let got = f.with_repo(|repo| repo.fingerprint(&base, &tip)).unwrap();
    assert_eq!(got, fp);
    // A base other than `origin/main`: the branch's own first commit, so only the second
    // commit's diff is fingerprinted.
    let first = commit_local(&f, "spec/x", "second.txt");
    let against_first = f.with_repo(|repo| repo.fingerprint(&tip, &first)).unwrap();
    let against_main = f.with_repo(|repo| repo.fingerprint(&base, &first)).unwrap();
    assert_ne!(against_first, against_main);
}
