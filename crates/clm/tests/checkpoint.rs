//! `clm checkpoint` on a bare `origin` and its clone, with lands recorded as `clm land`
//! records them: a merge commit on `origin/main` and `landed` and `checked` records on
//! another clone's ledger branch on `origin`.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use clm_ledger::{Body, LandKind, Record};
use common::*;
use tempfile::TempDir;

/// Another clone's ledger branch, as `origin` holds it.
const LEDGER: &str = "clm/ledger/lander/00000000";

/// `clm checkpoint --root <root>`: exit code, then stdout and stderr. A `gh` that fails
/// comes first on `PATH`: checkpoint never calls it.
fn checkpoint(root: &Path) -> (Option<i32>, String) {
    let bin = TempDir::new().unwrap();
    write_exec(
        &bin.path().join("gh"),
        "#!/bin/sh\necho 'gh called' >&2\nexit 1\n",
    );
    let path = format!(
        "{}:{}",
        bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["wrap-up", "--root"])
            .arg(root)
            .env("PATH", path),
    );
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap() + &String::from_utf8(out.stderr).unwrap(),
    )
}

fn exists(dir: &Path, rf: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "-q", rf])
        .current_dir(dir)
        .output()
        .unwrap()
        .status
        .success()
}

/// Appends `body` to [`LEDGER`] on `origin` with git plumbing, as another clone would.
fn record(r: &Repo, body: Body) {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    git(&r.work, &["fetch", "-q", "origin"]);
    let remote = format!("refs/remotes/origin/{LEDGER}");
    let old = exists(&r.work, &remote).then(|| git(&r.work, &["rev-parse", &remote]));
    let mut text = match &old {
        Some(old) => git(
            &r.work,
            &["cat-file", "blob", &format!("{}:records.jsonl", old.trim())],
        ),
        None => String::new(),
    };
    let rec = Record {
        id: format!("{:016x}{n:016x}", std::process::id()),
        writer: "lander/00000000".into(),
        at: format!("2026-01-01T00:{:02}:{:02}.000Z", n / 60 % 60, n % 60),
        body,
    };
    text.push_str(&serde_json::to_string(&rec).unwrap());
    text.push('\n');
    let tmp = TempDir::new().unwrap();
    let file = tmp.path().join("records.jsonl");
    std::fs::write(&file, text).unwrap();
    let blob = git(&r.work, &["hash-object", "-w", file.to_str().unwrap()]);
    let index = tmp.path().join("index");
    let env = [("GIT_INDEX_FILE", index.to_str().unwrap())];
    let info = format!("100644,{},records.jsonl", blob.trim());
    git_env(
        &r.work,
        &["update-index", "--add", "--cacheinfo", &info],
        &env,
    );
    let tree = git_env(&r.work, &["write-tree"], &env);
    let mut args = vec!["commit-tree", tree.trim(), "-m", "ledger"];
    if let Some(old) = &old {
        args.extend(["-p", old.trim()]);
    }
    let sha = git(&r.work, &args);
    git(
        &r.work,
        &[
            "push",
            "-q",
            "origin",
            &format!("{}:refs/heads/{LEDGER}", sha.trim()),
        ],
    );
    git(&r.work, &["fetch", "-q", "origin"]);
}

/// A merge commit of `origin/main` and `tip`, `tip` its second parent, not pushed.
fn merge_commit(r: &Repo, tip: &str) -> String {
    git(&r.work, &["fetch", "-q", "origin"]);
    let tree = git(&r.work, &["merge-tree", "--write-tree", "origin/main", tip]);
    let sha = git(
        &r.work,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            "origin/main",
            "-p",
            tip,
            "-m",
            "land",
        ],
    );
    sha.trim().to_string()
}

/// `branch` at `tip` landed as `clm land` lands it: the merge pushed to `origin/main`, with
/// its `landed` record and the `checked` record for its code key. Returns the merge commit.
fn land(r: &Repo, branch: &str, tip: &str, review: bool, changes: &[&str]) -> String {
    let tip = tip.trim();
    let commit = merge_commit(r, tip);
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{commit}:refs/heads/main")],
    );
    record(r, landed(&commit, branch, tip, review, changes));
    let code_key = clm_ledger::code_key(&r.work, &commit).unwrap();
    record(
        r,
        Body::Checked {
            code_key,
            command: "ci".into(),
        },
    );
    commit
}

/// A code land of `branch` at `tip`.
fn land_code(r: &Repo, branch: &str, tip: &str) -> String {
    land(r, branch, tip, false, &[])
}

/// The `landed` record of `branch` at `tip` in `commit`; its fingerprint is `fp-<tip>`.
fn landed(commit: &str, branch: &str, tip: &str, review: bool, changes: &[&str]) -> Body {
    Body::Landed {
        commit: commit.into(),
        branch: branch.into(),
        kind: if branch.starts_with("spec/") {
            LandKind::Spec
        } else {
            LandKind::Code
        },
        review,
        fingerprint: format!("fp-{tip}"),
        code_key: None,
        changes: changes.iter().map(|c| c.to_string()).collect(),
    }
}

/// A build's worktree, where Claude Code puts a subagent's: `.claude/worktrees/agent-<item>`
/// on a new branch `impl/<item>` with one commit, pushed to `origin`. Returns its path and
/// tip.
fn impl_worktree(r: &Repo, item: &str) -> (PathBuf, String) {
    worktree_at(r, &format!("agent-{}", item.replace('/', "-")), item)
}

/// A worktree `.claude/worktrees/<dir>` on a new branch `impl/<item>` with one commit,
/// pushed to `origin`. Returns its path and tip.
fn worktree_at(r: &Repo, dir: &str, item: &str) -> (PathBuf, String) {
    let wt = r.work.join(".claude/worktrees").join(dir);
    worktree(r, &wt, &format!("impl/{item}"))
}

/// A worktree at `wt` on a new branch `branch` from `origin/main` with one commit, pushed to
/// `origin`. Returns its path and tip.
fn worktree(r: &Repo, wt: &Path, branch: &str) -> (PathBuf, String) {
    write(&r.work, ".git/info/exclude", ".claude/\nother/\n");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            branch,
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    write(wt, &format!("work-{}.txt", branch.replace('/', "-")), "w\n");
    git(wt, &["add", "-A"]);
    git(wt, &["commit", "-qm", "work"]);
    git(wt, &["push", "-q", "-u", "origin", branch]);
    let tip = git(wt, &["rev-parse", "HEAD"]);
    (wt.to_path_buf(), tip.trim().to_string())
}

/// A local branch `branch` from `origin/main` with one commit, optionally pushed.
fn spec_branch(r: &Repo, branch: &str, push: bool) -> String {
    git(&r.work, &["branch", branch, "origin/main"]);
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let parent = git(&r.work, &["rev-parse", branch]);
    let sha = git(
        &r.work,
        &[
            "commit-tree",
            tree.trim(),
            "-p",
            parent.trim(),
            "-m",
            "spec",
        ],
    );
    git(&r.work, &["branch", "-f", branch, sha.trim()]);
    if push {
        git(&r.work, &["push", "-q", "origin", branch]);
    }
    sha.trim().to_string()
}

/// Checks out a new `spec/y` adding change `y`, pushed to `origin`. Returns its tip.
fn propose_y(r: &Repo) -> String {
    git(&r.work, &["checkout", "-qb", "spec/y"]);
    write(&r.work, "changes/y/change.md", &change("proposed"));
    git(&r.work, &["add", "-A"]);
    git(&r.work, &["commit", "-qm", "propose y"]);
    git(&r.work, &["push", "-q", "origin", "spec/y"]);
    git(&r.work, &["rev-parse", "HEAD"]).trim().to_string()
}

#[test]
fn scenario_landed_work_cleaned_up() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, tip) = impl_worktree(&r, "x/a");
    // Claude Code locks the worktrees it makes.
    git(&r.work, &["worktree", "lock", wt.to_str().unwrap()]);
    land_code(&r, "impl/x/a", &tip);
    let y = propose_y(&r);
    git(&r.work, &["push", "-q", "origin", "spec/y:autosync/spec/y"]);
    git(&r.work, &["checkout", "-q", "main"]);
    land(&r, "spec/y", &y, true, &["y"]);

    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert!(!out.contains("gh called"), "{out}");
    assert!(!wt.exists(), "{out}");
    for line in [
        format!("removed {}", wt.display()),
        "removed impl/x/a".into(),
        "removed spec/y".into(),
        "removed origin/impl/x/a".into(),
        "removed origin/spec/y".into(),
        "removed origin/autosync/spec/y".into(),
    ] {
        assert!(out.lines().any(|l| l == line), "{line}: {out}");
    }
    for rf in ["impl/x/a", "spec/y", "autosync/spec/y"] {
        assert!(!exists(&r.origin, &format!("refs/heads/{rf}")), "{rf}");
    }
    for rf in ["impl/x/a", "spec/y"] {
        assert!(!exists(&r.work, &format!("refs/heads/{rf}")), "{rf}");
    }
}

#[test]
fn land_off_main_is_ignored() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let tip = spec_branch(&r, "spec/z", true);
    // A `landed` record whose commit never reached `origin/main`.
    let commit = merge_commit(&r, &tip);
    record(&r, landed(&commit, "spec/z", &tip, false, &[]));
    let (code, out) = checkpoint(&r.work);
    assert!(exists(&r.work, "refs/heads/spec/z"), "{out}");
    assert!(exists(&r.origin, "refs/heads/spec/z"), "{out}");
    assert!(out.lines().any(|l| l == "open spec/z"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_live_claim_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    push_claim(&r.work, "x/b", 0);
    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert!(exists(&r.origin, "refs/heads/impl/x/b"));
}

#[test]
fn scenario_new_branch_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    // No commits of its own: its tip is in `origin/main`, but no land names it.
    git(&r.work, &["branch", "spec/n", "origin/main"]);
    git(&r.work, &["push", "-q", "origin", "spec/n"]);
    let (code, out) = checkpoint(&r.work);
    assert!(exists(&r.work, "refs/heads/spec/n"), "{out}");
    assert!(exists(&r.origin, "refs/heads/spec/n"), "{out}");
    assert!(out.lines().any(|l| l == "open spec/n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_merged_without_a_record() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    // Never pushed, merged into `origin/main` by a merge commit, as a GitHub pull request
    // merged it before the cutover: no `landed` record names it.
    let tip = spec_branch(&r, "impl/c", false);
    let merge = merge_commit(&r, &tip);
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{merge}:refs/heads/main")],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!exists(&r.work, "refs/heads/impl/c"), "{out}");
    assert!(out.lines().any(|l| l == "removed impl/c"), "{out}");
    assert!(!out.contains("unpushed"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn landed_without_a_record_on_origin_is_removed() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let tip = spec_branch(&r, "spec/g", true);
    let merge = merge_commit(&r, &tip);
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{merge}:refs/heads/main")],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!exists(&r.origin, "refs/heads/spec/g"), "{out}");
    assert!(!exists(&r.work, "refs/heads/spec/g"), "{out}");
    assert!(out.lines().any(|l| l == "removed origin/spec/g"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_old_branch_from_main_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    // At an older commit on `origin/main`'s first-parent chain, with no commits of its own.
    git(&r.work, &["branch", "spec/m", "origin/main"]);
    git(&r.work, &["push", "-q", "origin", "spec/m"]);
    write(&r.work, "more.txt", "m\n");
    push_main(&r.work, "move main on");
    write(&r.work, "more.txt", "mm\n");
    push_main(&r.work, "and again");
    let (_, out) = checkpoint(&r.work);
    assert!(exists(&r.work, "refs/heads/spec/m"), "{out}");
    assert!(exists(&r.origin, "refs/heads/spec/m"), "{out}");
    assert!(!out.contains("removed"), "{out}");
}

#[test]
fn scenario_unpushed_work() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    spec_branch(&r, "spec/z", true);
    let tree = git(&r.work, &["rev-parse", "spec/z^{tree}"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", "spec/z", "-m", "more"],
    );
    git(&r.work, &["branch", "-f", "spec/z", sha.trim()]);
    // The checkout `clm checkpoint` runs in is on `spec/z`.
    git(&r.work, &["checkout", "-q", "spec/z"]);
    spec_branch(&r, "impl/x/c", false);
    let (code, out) = checkpoint(&r.work);
    assert!(out.lines().any(|l| l == "unpushed spec/z"), "{out}");
    // Not checked out in the session's own checkouts: someone else's.
    assert!(
        out.lines().any(|l| l == "elsewhere unpushed impl/x/c"),
        "{out}"
    );
    assert!(!out.contains("open spec/z"), "{out}");
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1));
}

/// Makes this clone's ledger branch `clm/ledger/me/1234abcd`, its clone part in
/// `.git/clm/writer` as `append` writes it, with one commit, pushed or not.
fn own_ledger(r: &Repo, push: bool) -> String {
    write(&r.work, ".git/clm/writer", "1234abcd\n");
    ledger_branch(r, "clm/ledger/me/1234abcd", push)
}

/// A local ledger branch `branch` with one commit, pushed or not.
fn ledger_branch(r: &Repo, branch: &str, push: bool) -> String {
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-m", "ledger: landed"],
    );
    git(&r.work, &["branch", branch, sha.trim()]);
    if push {
        git(&r.work, &["push", "-q", "origin", branch]);
    }
    branch.to_string()
}

#[test]
fn another_writers_local_ledger_isnt_the_clones() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    write(&r.work, ".git/clm/writer", "1234abcd\n");
    // Another clone's ledger, checked out here to read it, its `origin` branch since gone.
    ledger_branch(&r, "clm/ledger/lander/99999999", false);
    let (code, out) = checkpoint(&r.work);
    assert!(!out.contains("unpushed"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_unpushed_ledger() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let branch = own_ledger(&r, false);
    let (code, out) = checkpoint(&r.work);
    assert!(
        out.lines().any(|l| l == format!("unpushed {branch}")),
        "{out}"
    );
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1), "{out}");

    // A record `origin` lacks on a branch it has.
    git(&r.work, &["push", "-q", "origin", &branch]);
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", &branch, "-m", "more"],
    );
    git(&r.work, &["branch", "-f", &branch, sha.trim()]);
    let (code, out) = checkpoint(&r.work);
    assert!(
        out.lines().any(|l| l == format!("unpushed {branch}")),
        "{out}"
    );
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn pushed_ledger_is_safe() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    own_ledger(&r, true);
    let (code, out) = checkpoint(&r.work);
    assert!(!out.contains("unpushed"), "{out}");
    assert!(!out.contains("open clm/ledger"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_open_branch_only() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    spec_branch(&r, "spec/z", true);
    let (code, out) = checkpoint(&r.work);
    assert!(out.lines().any(|l| l == "open spec/z"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0));
}

#[test]
fn checkpoint_reports_no_review() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    // A land recorded `SOFT` by an older `clm`, which then awaited an after-merge review.
    let r = repo();
    let tip = spec_branch(&r, "spec/y", true);
    let commit = merge_commit(&r, tip.trim());
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{commit}:refs/heads/main")],
    );
    let line = format!(
        r#"{{"id":"1","writer":"t/1","at":"2026-01-01T00:00:00.000Z","kind":"landed","commit":"{commit}","branch":"spec/y","land_kind":"spec","tier":"SOFT","fingerprint":"f","changes":["y"]}}"#
    );
    let fold = clm_ledger::fold_records(&[("l", &line)]);
    let land = fold.records[0].body.clone();
    record(&r, land);
    let code_key = clm_ledger::code_key(&r.work, &commit).unwrap();
    record(
        &r,
        Body::Checked {
            code_key,
            command: "ci".into(),
        },
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!out.lines().any(|l| l.starts_with("review")), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn recreated_branch_with_an_old_land_is_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let tip = spec_branch(&r, "spec/z", true);
    land(&r, "spec/z", &tip, false, &[]);
    // `spec/z` made again from `origin/main`: its tip is a commit `main` itself was at, on
    // the first-parent chain.
    git(&r.work, &["branch", "-f", "spec/z", "origin/main"]);
    git(&r.work, &["push", "-q", "-f", "origin", "spec/z"]);
    let (code, out) = checkpoint(&r.work);
    assert!(exists(&r.work, "refs/heads/spec/z"), "{out}");
    assert!(exists(&r.origin, "refs/heads/spec/z"), "{out}");
    assert!(out.lines().any(|l| l == "open spec/z"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_unrecorded_land() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let tip = spec_branch(&r, "spec/z", true);
    land(&r, "spec/z", &tip, false, &[]);
    // Merged into `origin/main` without `clm land`.
    let other = spec_branch(&r, "spec/o", true);
    let merge = merge_commit(&r, &other);
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{merge}:refs/heads/main")],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(
        out.lines()
            .any(|l| l.starts_with("origin/main: ledger/unrecorded-land: ") && l.contains(&merge)),
        "{out}"
    );
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1), "{out}");
    // Landed all the same, so removed like any other landed branch.
    assert!(!exists(&r.work, "refs/heads/spec/o"), "{out}");
    assert!(!exists(&r.origin, "refs/heads/spec/o"), "{out}");
}

#[test]
fn build_worktree_merged_without_a_record_is_removed() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, tip) = impl_worktree(&r, "x/a");
    let merge = merge_commit(&r, &tip);
    git(
        &r.work,
        &["push", "-q", "origin", &format!("{merge}:refs/heads/main")],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!wt.exists(), "{out}");
    assert!(!exists(&r.work, "refs/heads/impl/x/a"), "{out}");
    assert!(!exists(&r.origin, "refs/heads/impl/x/a"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn landed_at_an_older_tip_is_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let old = spec_branch(&r, "spec/z", true);
    land(&r, "spec/z", &old, false, &[]);
    let sha = git(
        &r.work,
        &["commit-tree", "spec/z^{tree}", "-p", "spec/z", "-m", "more"],
    );
    git(&r.work, &["branch", "-f", "spec/z", sha.trim()]);
    git(&r.work, &["push", "-q", "origin", "spec/z"]);
    let (code, out) = checkpoint(&r.work);
    assert!(exists(&r.work, "refs/heads/spec/z"), "{out}");
    assert!(exists(&r.origin, "refs/heads/spec/z"), "{out}");
    assert!(out.lines().any(|l| l == "open spec/z"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_humans_files_on_main() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    write(&r.work, "notes.txt", "mine\n");
    let (code, out) = checkpoint(&r.work);
    assert!(!out.contains("dirty"), "{out}");
    assert_eq!(code, Some(0), "{out}");
    assert!(r.work.join("notes.txt").exists());
}

#[test]
fn dirty_landed_build_worktree_is_kept_and_reported_elsewhere() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, tip) = impl_worktree(&r, "x/a");
    land_code(&r, "impl/x/a", &tip);
    write(&wt, "scratch.txt", "unsaved\n");
    let (code, out) = checkpoint(&r.work);
    assert!(wt.join("scratch.txt").exists());
    assert!(exists(&r.work, "refs/heads/impl/x/a"));
    assert!(
        out.lines()
            .any(|l| l == format!("elsewhere dirty {}", wt.display())),
        "{out}"
    );
    // The landed branch's work is on `main`, so it isn't unpushed.
    assert!(!out.contains("unpushed"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0));
}

#[test]
fn main_checkout_on_a_landed_branch_switches_to_main() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let y = propose_y(&r);
    land(&r, "spec/y", &y, false, &["y"]);

    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "main");
    assert_eq!(
        git(&r.work, &["rev-parse", "main"]),
        git(&r.work, &["rev-parse", "origin/main"])
    );
    assert!(!exists(&r.work, "refs/heads/spec/y"));
}

#[test]
fn main_checkout_on_a_builds_branch_switches_to_main() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    git(
        &r.work,
        &["checkout", "-qb", "worktree-agent-1", "origin/main"],
    );

    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "main");
    assert!(!exists(&r.work, "refs/heads/worktree-agent-1"), "{out}");
    assert!(
        out.lines().any(|l| l == "removed worktree-agent-1"),
        "{out}"
    );
}

#[test]
fn dirty_main_checkout_on_a_builds_branch_is_left() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    git(
        &r.work,
        &["checkout", "-qb", "worktree-agent-1", "origin/main"],
    );
    write(&r.work, "scratch.txt", "s\n");

    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(
        git(&r.work, &["branch", "--show-current"]).trim(),
        "worktree-agent-1"
    );
    assert!(exists(&r.work, "refs/heads/worktree-agent-1"), "{out}");
    assert!(r.work.join("scratch.txt").exists());
}

#[test]
fn dirty_main_checkout_on_a_landed_branch_is_left() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let y = propose_y(&r);
    land(&r, "spec/y", &y, false, &["y"]);
    write(&r.work, "draft.md", "wip\n");
    let (code, out) = checkpoint(&r.work);
    assert_eq!(git(&r.work, &["branch", "--show-current"]).trim(), "spec/y");
    assert!(
        out.lines()
            .any(|l| l == format!("dirty {}", r.work.canonicalize().unwrap().display())),
        "{out}"
    );
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn scenario_abandoned_autosync_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    git(
        &r.work,
        &[
            "push",
            "-q",
            "origin",
            "origin/main:refs/heads/autosync/spec/q",
        ],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(exists(&r.origin, "refs/heads/autosync/spec/q"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn archived_change_releases_its_autosync() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    write(&r.work, "changes/archive/q/change.md", &change("done"));
    push_main(&r.work, "archive q");
    git(
        &r.work,
        &[
            "push",
            "-q",
            "origin",
            "origin/main:refs/heads/autosync/spec/q",
        ],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!exists(&r.origin, "refs/heads/autosync/spec/q"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn violations_on_main_are_reported() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    write(&r.work, "specs/d/SPEC.md", "no frontmatter\n");
    push_main(&r.work, "break main");
    let (code, out) = checkpoint(&r.work);
    assert!(out.contains("specs/d/SPEC.md: "), "{out}");
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1));
}

#[test]
fn runs_from_inside_a_worktree() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, _) = impl_worktree(&r, "x/b");
    let (done, tip) = impl_worktree(&r, "x/a");
    land_code(&r, "impl/x/a", &tip);
    let (code, out) = checkpoint(&wt);
    assert!(!done.exists(), "{out}");
    assert!(wt.exists());
    assert!(out.lines().any(|l| l == "open impl/x/b"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn the_callers_own_landed_worktree_is_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, tip) = impl_worktree(&r, "x/a");
    land_code(&r, "impl/x/a", &tip);
    let (code, out) = checkpoint(&wt);
    // Removing it would pull the directory out from under the session that ran it.
    assert!(wt.exists(), "{out}");
    assert!(!out.contains("unpushed"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn landed_worktree_outside_the_shelf_is_left_alone() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let elsewhere = TempDir::new().unwrap();
    let (wt, tip) = worktree(&r, &elsewhere.path().join("mine"), "spec/m");
    land(&r, "spec/m", &tip, false, &[]);
    let (code, out) = checkpoint(&r.work);
    assert!(wt.exists(), "{out}");
    assert!(exists(&r.work, "refs/heads/spec/m"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

/// A linked worktree `w` of `r.work`, on `main`.
fn linked_main(r: &Repo) -> PathBuf {
    let w = r.work.with_file_name("w");
    git(
        &r.work,
        &["worktree", "add", "-q", w.to_str().unwrap(), "main"],
    );
    w
}

#[test]
fn scenario_another_sessions_work() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    spec_branch(&r, "spec/z", true);
    let tree = git(&r.work, &["rev-parse", "spec/z^{tree}"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", "spec/z", "-m", "more"],
    );
    git(&r.work, &["branch", "-f", "spec/z", sha.trim()]);
    git(&r.work, &["checkout", "-q", "spec/z"]);
    write(&r.work, "draft.md", "wip\n");
    spec_branch(&r, "spec/o", true);
    let w = linked_main(&r);
    let (code, out) = checkpoint(&w);
    let main = r.work.canonicalize().unwrap();
    let dirty = format!("elsewhere dirty {}", main.display());
    let lines: Vec<&str> = out.lines().collect();
    let at = |l: &str| lines.iter().position(|x| *x == l);
    assert!(at(&dirty).is_some(), "{out}");
    assert!(at("elsewhere unpushed spec/z").is_some(), "{out}");
    assert!(!lines.iter().any(|l| l.starts_with("dirty ")), "{out}");
    assert!(!lines.iter().any(|l| l.starts_with("unpushed ")), "{out}");
    // `elsewhere` lines come before `open` ones.
    assert!(at("elsewhere unpushed spec/z") < at("open spec/o"), "{out}");
    assert_eq!(lines.last(), Some(&"wrap-up ok"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_builds_worktree_is_elsewhere() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    // `main` is free for `w`; the main checkout is clean and on no branch.
    git(&r.work, &["checkout", "-q", "--detach"]);
    let w = linked_main(&r);
    let (wt, _) = worktree_at(&r, "agent-1", "a");
    write(&wt, "scratch.txt", "unsaved\n");
    let (code, out) = checkpoint(&w);
    let wt = wt.canonicalize().unwrap();
    assert!(
        out.lines()
            .any(|l| l == format!("elsewhere dirty {}", wt.display())),
        "{out}"
    );
    assert!(!out.lines().any(|l| l.starts_with("dirty ")), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn own_checkout_and_its_branch_are_the_sessions_own() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, _) = impl_worktree(&r, "x/a");
    write(&wt, "scratch.txt", "unsaved\n");
    git(&wt, &["commit", "-q", "--allow-empty", "-m", "unpushed"]);
    let (code, out) = checkpoint(&wt);
    let wt = wt.canonicalize().unwrap();
    assert!(
        out.lines().any(|l| l == format!("dirty {}", wt.display())),
        "{out}"
    );
    assert!(out.lines().any(|l| l == "unpushed impl/x/a"), "{out}");
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn scenario_builds_abandoned_branch() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    git(&r.work, &["branch", "worktree-agent-1", "origin/main"]);
    spec_branch(&r, "worktree-agent-2", false);
    // At `origin/main`'s tip too, but a worktree has it checked out.
    let held = r.work.join(".claude/worktrees/agent-3");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "worktree-agent-3",
            held.to_str().unwrap(),
            "origin/main",
        ],
    );
    let (code, out) = checkpoint(&r.work);
    assert!(!exists(&r.work, "refs/heads/worktree-agent-1"), "{out}");
    assert!(
        out.lines().any(|l| l == "removed worktree-agent-1"),
        "{out}"
    );
    assert!(exists(&r.work, "refs/heads/worktree-agent-2"), "{out}");
    assert!(exists(&r.work, "refs/heads/worktree-agent-3"), "{out}");
    assert!(held.exists());
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_humans_worktree_kept() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let (wt, tip) = worktree_at(&r, "p3", "a");
    land_code(&r, "impl/a", &tip);
    git(&r.work, &["checkout", "-q", "--detach"]);
    let w = linked_main(&r);
    let (code, out) = checkpoint(&w);
    assert!(wt.exists(), "{out}");
    assert!(exists(&r.work, "refs/heads/impl/a"), "{out}");
    assert!(!out.contains(&format!("removed {}", wt.display())), "{out}");
    assert!(!out.lines().any(|l| l == "removed impl/a"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn only_impl_worktrees_in_claude_worktrees_agent_dirs_are_builds() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    // `agent-1` outside `.claude/worktrees/`, on a landed `impl/a`.
    let outside = r.work.join("other/worktrees/agent-1");
    // `.claude/worktrees/agent-2` on a landed `spec/s`.
    let spec = r.work.join(".claude/worktrees/agent-2");
    for (wt, branch) in [(&outside, "impl/a"), (&spec, "spec/s")] {
        let (_, tip) = worktree(&r, wt, branch);
        land(&r, branch, &tip, false, &[]);
    }
    let (code, out) = checkpoint(&r.work);
    for (wt, branch) in [(&outside, "impl/a"), (&spec, "spec/s")] {
        assert!(wt.exists(), "{}: {out}", wt.display());
        assert!(exists(&r.work, &format!("refs/heads/{branch}")), "{out}");
    }
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn git_failure_exits_2() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let dir = TempDir::new().unwrap();
    let (code, out) = checkpoint(dir.path());
    assert_eq!(code, Some(2), "{out}");
}

#[test]
fn scenario_complete_change() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    cite(&r.work, "all.rs", &["a/p", "a/q", "b/r", "c/s"]);
    push_main(&r.work, "build everything");
    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.contains("complete x\n"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");

    // Half-built, it isn't complete.
    cite(&r.work, "all.rs", &["a/p", "b/r"]);
    push_main(&r.work, "less");
    let (_, out) = checkpoint(&r.work);
    assert!(!out.contains("complete x"), "{out}");
}

/// Pushes a commit on `origin/main` to `propose/<slug>`.
fn propose_claim(r: &Repo, slug: &str) {
    let tree = git(&r.work, &["rev-parse", "origin/main^{tree}"]);
    let main = git(&r.work, &["rev-parse", "origin/main"]);
    let sha = git(
        &r.work,
        &["commit-tree", tree.trim(), "-p", main.trim(), "-m", "claim"],
    );
    git(
        &r.work,
        &[
            "push",
            "-q",
            "origin",
            &format!("{}:refs/heads/propose/{slug}", sha.trim()),
        ],
    );
}

#[test]
fn scenario_done_backlog_claims_removed() {
    // req:orchestrator.checkpoint/checkpoint@f59546e3
    let r = repo();
    let w = &r.work;
    for slug in ["r", "s", "t"] {
        write(
            w,
            &format!("backlog/{slug}.md"),
            "---\ntitle: I\nkind: idea\n---\n",
        );
    }
    write(
        w,
        "changes/y/change.md",
        "---\nstatus: applied\nresolves: [r]\n---\n",
    );
    write(
        w,
        "changes/archive/z/change.md",
        "---\nstatus: done\nresolves: [t]\n---\n",
    );
    push_main(w, "items and changes");
    for slug in ["q", "r", "s", "t"] {
        propose_claim(&r, slug);
    }
    let (code, out) = checkpoint(w);
    assert_eq!(code, Some(0), "{out}");
    for (slug, kept) in [("q", false), ("r", false), ("s", true), ("t", true)] {
        assert_eq!(
            exists(&r.origin, &format!("refs/heads/propose/{slug}")),
            kept,
            "propose/{slug}: {out}"
        );
    }
    assert!(out.contains("removed origin/propose/q\n"), "{out}");
}

/// The clone's places directory, for a clone whose git directory is `.git`.
fn builds(work: &Path) -> PathBuf {
    work.join(".git/clm/builds")
}

#[test]
fn scenario_stale_places_freed() {
    // req:orchestrator.checkpoint/checkpoint-frees-places@b8507909
    use clm_orchestrator::{PlaceKind, take_place};
    let r = repo();
    // `c` was just taken by a `clm run` that hasn't claimed yet: it counts by its `taken`
    // time under the default expiry.
    for node in ["a", "b", "c"] {
        let branch = [format!("impl/{node}")];
        take_place(&r.work, node, PlaceKind::Build, &branch, 3, 4).unwrap();
    }
    // `a`'s build landed, `clm land` deleted `origin/impl/a`, and its worktree remains.
    let (wt, tip) = impl_worktree(&r, "a");
    land_code(&r, "impl/a", &tip);
    git(&r.work, &["push", "-q", "origin", "--delete", "impl/a"]);
    // `b`'s build is still running on a live claim.
    push_claim(&r.work, "b", 1);

    let (code, out) = checkpoint(&r.work);
    assert!(!builds(&r.work).join("a").exists(), "{out}");
    assert!(builds(&r.work).join("b").exists(), "{out}");
    assert!(builds(&r.work).join("c").exists(), "{out}");
    assert!(out.lines().any(|l| l == "freed a"), "{out}");
    assert!(
        !out.contains("freed b") && !out.contains("freed c"),
        "{out}"
    );
    // The place was judged before checkpoint removed the build's branch and worktree.
    assert!(out.lines().any(|l| l == "removed impl/a"), "{out}");
    assert!(!wt.exists(), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn scenario_damaged_place_kept() {
    // req:orchestrator.checkpoint/checkpoint-frees-places@b8507909
    let r = repo();
    write(&builds(&r.work), "z", "not a place");
    let (code, out) = checkpoint(&r.work);
    let file = builds(&r.work).join("z");
    assert!(file.exists(), "{out}");
    let prefix = format!("place {}: ", file.canonicalize().unwrap().display());
    assert!(
        out.lines().any(|l| l.starts_with(&prefix)),
        "{prefix}: {out}"
    );
    assert!(!out.contains("freed"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[cfg(unix)]
#[test]
fn a_place_the_sweep_cant_delete_is_kept_and_reported() {
    // req:orchestrator.checkpoint/checkpoint-frees-places@b8507909
    use std::os::unix::fs::PermissionsExt;
    let r = repo();
    let dir = builds(&r.work);
    // Taken long ago for nodes with no branch: they don't count. `m` is damaged.
    for name in ["a", "z"] {
        let place = format!("kind build\ntaken 2020-01-01T00:00:00Z\nbranch impl/{name}\n");
        write(&dir, name, &place);
    }
    write(&dir, "m", "not a place");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
    // Root deletes from a read-only directory: nothing to test there.
    let probe = dir.join("probe");
    let writable = std::fs::write(&probe, "").is_ok();
    let run = (!writable).then(|| checkpoint(&r.work));
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
    let Some((code, out)) = run else {
        std::fs::remove_file(&probe).unwrap();
        eprintln!("skipped: the places directory stays writable");
        return;
    };
    let paths: Vec<String> = ["a", "m", "z"]
        .iter()
        .map(|n| dir.join(n).canonicalize().unwrap().display().to_string())
        .collect();
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with("place ")).collect();
    assert_eq!(lines.len(), 3, "{out}");
    // In the sweep's order, each with why and its path once.
    for (line, path) in lines.iter().zip(&paths) {
        let prefix = format!("place {path}: ");
        assert!(
            line.starts_with(&prefix) && line.len() > prefix.len(),
            "{out}"
        );
        assert_eq!(line.matches(path.as_str()).count(), 1, "{line}");
    }
    assert!(!out.contains("freed"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
}

#[test]
fn a_sweep_that_fails_exits_2() {
    // req:orchestrator.checkpoint/checkpoint-frees-places@b8507909
    let r = repo();
    // No places directory can be made under a file.
    write(&r.work.join(".git"), "clm", "not a directory");
    let (code, out) = checkpoint(&r.work);
    assert_eq!(code, Some(2), "{out}");
    // The sweep's failure, naming the places directory it couldn't make.
    assert!(out.contains(".git/clm/builds"), "{out}");
    assert!(!out.contains("wrap-up ok"), "{out}");
}

/// An ed25519 key `name` in `dir`: its path and its public key.
fn keygen(dir: &Path, name: &str) -> (PathBuf, String) {
    let key = dir.join(name);
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
        .arg(&key)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen -t ed25519");
    let public = std::fs::read_to_string(key.with_extension("pub")).unwrap();
    (key, public.trim().to_string())
}

/// A signed repository (`signed-ledger-records`): `docs/allowed_signers` on `origin/main`
/// lists `christopher-lunny`, whose key this clone signs with.
struct Signed {
    r: Repo,
    _keys: TempDir,
    home: String,
    global: String,
}

fn signed() -> Signed {
    let r = repo();
    let keys = TempDir::new().unwrap();
    let (key, public) = keygen(keys.path(), "christopher");
    write(
        &r.work,
        "docs/allowed_signers",
        &format!("christopher-lunny {public}\n"),
    );
    push_main(&r.work, "signers");
    git(&r.work, &["config", "gpg.format", "ssh"]);
    git(
        &r.work,
        &["config", "user.signingkey", key.to_str().unwrap()],
    );
    let home = r.work.to_string_lossy().into_owned();
    let global = r
        .work
        .join(".git/no-gitconfig")
        .to_string_lossy()
        .into_owned();
    Signed {
        r,
        _keys: keys,
        home,
        global,
    }
}

impl Signed {
    /// Branch `name` off `main` writing `files`, merged into `origin/main` and landed as
    /// `kind` and `review` by a signed and checked `landed` record. Returns the merge commit.
    fn land(&self, name: &str, files: &[(&str, &[u8])], kind: LandKind, review: bool) -> String {
        let work = &self.r.work;
        git(work, &["switch", "-q", "-c", name]);
        for (path, bytes) in files {
            let path = work.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, bytes).unwrap();
        }
        git(work, &["add", "-A"]);
        git(work, &["commit", "-qm", name]);
        let merge = self.merge(name);
        self.record(&merge, name, kind, review);
        merge
    }

    /// The branch `name`, checked out, merged into `main` and pushed: the merge commit.
    fn merge(&self, name: &str) -> String {
        let work = &self.r.work;
        git(work, &["switch", "-q", "main"]);
        git(work, &["merge", "-q", "--no-ff", "-m", "Merge", name]);
        git(work, &["push", "-q", "origin", "main"]);
        git(work, &["rev-parse", "HEAD"]).trim().to_string()
    }

    /// A signed `landed` record of `merge`, landed from `name` as `kind` and `review`, and a
    /// `checked` record of its code key.
    fn record(&self, merge: &str, name: &str, kind: LandKind, review: bool) {
        let work = &self.r.work;
        let env = |k: &str| match k {
            "HOME" => Some(self.home.clone()),
            "CLM_WRITER" => Some("christopher-lunny".to_string()),
            "GIT_CONFIG_GLOBAL" => Some(self.global.clone()),
            _ => None,
        };
        let code_key = clm_ledger::code_key(work, merge).unwrap();
        let checked = Body::Checked {
            code_key: code_key.clone(),
            command: "check".into(),
        };
        clm_ledger::append(work, checked, &env).unwrap();
        let land = Body::Landed {
            commit: merge.into(),
            branch: name.into(),
            kind,
            review,
            fingerprint: format!("f-{name}"),
            code_key: Some(code_key),
            changes: vec![],
        };
        let signed = clm_ledger::sign(work, land, "clm-land", &env).unwrap();
        clm_ledger::append(work, signed, &env).unwrap();
    }

    /// `ledger`'s *review recorded as none*: an edit to `ARCHITECTURE.md`, which needs
    /// review, recorded as not needing it, with no approval.
    fn understated(&self) -> String {
        let files: &[(&str, &[u8])] = &[("ARCHITECTURE.md", b"invariants\n")];
        self.land("architecture", files, LandKind::Spec, false)
    }

    /// The spec land of `ledger`'s *honest lands*: a requirement added to `c`.
    fn honest_spec(&self) -> String {
        self.requirement_added(false)
    }

    /// A land adding a requirement to `c`, which needs no review, recorded as `review`.
    fn requirement_added(&self, review: bool) -> String {
        let spec = std::fs::read_to_string(self.r.work.join("specs/c/SPEC.md")).unwrap()
            + "\n### Requirement: t\nIt SHALL.\n\n#### Scenario: s\n- ok\n";
        let files: &[(&str, &[u8])] = &[("specs/c/SPEC.md", spec.as_bytes())];
        self.land("req", files, LandKind::Spec, review)
    }
}

/// The local review mark, or `None` when it is unset.
fn mark(work: &Path) -> Option<String> {
    exists(work, "refs/clm/tiers-verified").then(|| {
        git(work, &["rev-parse", "refs/clm/tiers-verified"])
            .trim()
            .to_string()
    })
}

fn set_mark(work: &Path, rev: &str) {
    git(work, &["update-ref", "refs/clm/tiers-verified", rev]);
}

fn rev(work: &Path, rev: &str) -> String {
    git(work, &["rev-parse", rev]).trim().to_string()
}

fn has(out: &str, rule: &str, commit: &str) -> bool {
    out.lines().any(|l| l.contains(rule) && l.contains(commit))
}

#[test]
fn scenario_understated_land() {
    // req:orchestrator.checkpoint/checkpoint-verifies-review@0c4afbbd
    let s = signed();
    let merge = s.understated();
    let (code, out) = checkpoint(&s.r.work);
    assert!(has(&out, "ledger/understated-land", &merge), "{out}");
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn scenario_clean_lands_marked() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    s.honest_spec();
    let code_files: &[(&str, &[u8])] = &[("src/x.rs", b"fn x() {}\n")];
    s.land("code", code_files, LandKind::Code, false);
    let work = &s.r.work;
    assert_eq!(mark(work), None);
    let (code, out) = checkpoint(work);
    assert!(!out.contains("ledger/"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(mark(work), Some(rev(work, "origin/main")), "{out}");
}

#[test]
fn scenario_lands_before_the_mark_not_recomputed() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    s.understated();
    let work = &s.r.work;
    set_mark(work, &rev(work, "origin/main"));
    let (code, out) = checkpoint(work);
    assert!(!out.contains("ledger/understated-land"), "{out}");
    assert!(out.ends_with("wrap-up ok\n"), "{out}");
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(mark(work), Some(rev(work, "origin/main")), "{out}");

    // A clean land after the mark moves it on.
    s.honest_spec();
    let (code, out) = checkpoint(work);
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(mark(work), Some(rev(work, "origin/main")), "{out}");
}

#[test]
fn unapproved_land_alone_doesnt_hold_the_mark() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    // A diff that needs no review, recorded as needing it, with no approval: the audit
    // without `--review` reports it at every run, so it never holds the mark.
    let s = signed();
    let merge = s.requirement_added(true);
    let work = &s.r.work;
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/unapproved-land", &merge), "{out}");
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(mark(work), Some(rev(work, "origin/main")), "{out}");
}

#[test]
fn scenario_finding_holds_the_mark() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    let first = s.understated();
    s.honest_spec();
    let work = &s.r.work;
    for run in 0..2 {
        let (code, out) = checkpoint(work);
        assert!(
            has(&out, "ledger/understated-land", &first),
            "run {run}: {out}"
        );
        // Printed once, though both audits report the land's missing approval.
        let unapproved = out
            .lines()
            .filter(|l| l.contains("ledger/unapproved-land"))
            .count();
        assert_eq!(unapproved, 1, "run {run}: {out}");
        assert!(out.ends_with("wrap-up incomplete\n"), "run {run}: {out}");
        assert_eq!(code, Some(1), "run {run}: {out}");
        assert_eq!(mark(work), Some(rev(work, &format!("{first}^1"))), "{out}");
    }
}

#[test]
fn scenario_rewritten_main() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    let merge = s.understated();
    let work = &s.r.work;
    // A commit `origin/main` doesn't hold, as a rewritten `main` leaves behind, whose own
    // history holds the land: a walk after it would skip the land.
    let tree = rev(work, "origin/main^{tree}");
    let old = git(
        work,
        &["commit-tree", &tree, "-p", "origin/main", "-m", "old"],
    );
    set_mark(work, old.trim());
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/understated-land", &merge), "{out}");
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn mark_that_isnt_a_commit_recomputes_the_whole_chain() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    let merge = s.understated();
    let work = &s.r.work;
    set_mark(work, &rev(work, "origin/main^{tree}"));
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/understated-land", &merge), "{out}");
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(mark(work), Some(rev(work, &format!("{merge}^1"))), "{out}");
}

#[test]
fn scenario_no_lands_yet() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let r = repo();
    for run in 0..2 {
        if run == 1 {
            // A commit after the mark: the walk after it, unlike the one from the oldest
            // land, names it unrecorded.
            write(&r.work, "notes.md", "n\n");
            push_main(&r.work, "notes");
        }
        let (code, out) = checkpoint(&r.work);
        assert!(!out.contains("ledger/unrecorded-land"), "run {run}: {out}");
        assert_eq!(code, Some(0), "run {run}: {out}");
        assert_eq!(mark(&r.work), Some(rev(&r.work, "origin/main")), "{out}");
    }
}

#[test]
fn scenario_unrated_land() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    // `ledger`'s *unrated land*: a land adding `changes/u/change.md` as a gitlink to a commit
    // the repository doesn't hold, which the gate can't read, then *review recorded as none*.
    let s = signed();
    let work = &s.r.work;
    git(work, &["switch", "-q", "-c", "unreadable"]);
    let missing = format!("160000,{},changes/u/change.md", "1".repeat(40));
    git(work, &["update-index", "--add", "--cacheinfo", &missing]);
    git(work, &["commit", "-qm", "unreadable"]);
    let first = s.merge("unreadable");
    s.record(&first, "unreadable", LandKind::Spec, false);
    let second = s.understated();
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/unrated-land", &first), "{out}");
    assert!(has(&out, "ledger/understated-land", &second), "{out}");
    assert!(out.ends_with("wrap-up incomplete\n"), "{out}");
    assert_eq!(code, Some(1), "{out}");
    // The oldest land with a finding holds the mark.
    assert_eq!(mark(work), Some(rev(work, &format!("{first}^1"))), "{out}");
}

#[test]
fn scenario_record_arriving_late() {
    // req:orchestrator.checkpoint/checkpoint-review-mark@b17eb6fa
    let s = signed();
    let code_files: &[(&str, &[u8])] = &[("src/x.rs", b"fn x() {}\n")];
    s.land("code", code_files, LandKind::Code, false);
    let work = &s.r.work;
    git(work, &["switch", "-q", "-c", "architecture"]);
    write(work, "ARCHITECTURE.md", "invariants\n");
    git(work, &["add", "-A"]);
    git(work, &["commit", "-qm", "architecture"]);
    let late = s.merge("architecture");
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/unrecorded-land", &late), "{out}");
    assert_eq!(code, Some(1), "{out}");
    assert_eq!(mark(work), Some(rev(work, &format!("{late}^1"))), "{out}");
    s.record(&late, "architecture", LandKind::Spec, false);
    let (code, out) = checkpoint(work);
    assert!(has(&out, "ledger/understated-land", &late), "{out}");
    assert_eq!(code, Some(1), "{out}");
}

#[test]
fn wrap_up_failure_names_the_new_command() {
    // req:clm/renamed-commands@f600d2ff
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["wrap-up", "--root", "/nonexistent-dir"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm wrap-up: "), "{stderr}");
}

#[test]
fn scenario_wrap_up_by_its_new_name() {
    // req:clm/renamed-commands@f600d2ff
    let r = repo();
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["wrap-up", "--root"])
            .arg(&r.work),
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert_eq!(text.lines().last(), Some("wrap-up ok"), "{text}");
}
