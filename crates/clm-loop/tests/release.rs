//! Giving up a component's build (`release`).

mod common;

use std::fs;

use clm_loop::{Kind, RunState, release};
use common::{Fixture, run};

fn git(fx: &Fixture, args: &[&str]) -> String {
    clm_git::git(&fx.work, args).unwrap()
}

/// A worktree with the local `impl/a` checked out, pushed once and then one commit further, and
/// an untracked file. Returns the worktree's path and the unpushed commit.
fn built_branch(fx: &Fixture) -> (std::path::PathBuf, String) {
    let tree = fx.work.join(".claude/worktrees/agent-1");
    git(
        fx,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            tree.to_str().unwrap(),
            "origin/main",
        ],
    );
    git(fx, &["push", "-q", "origin", "impl/a"]);
    fs::write(tree.join("work.txt"), "w\n").unwrap();
    clm_git::git(&tree, &["add", "-A"]).unwrap();
    clm_git::git(&tree, &["commit", "-q", "-m", "unpushed work"]).unwrap();
    fs::write(tree.join("notes.txt"), "n\n").unwrap();
    let c = clm_git::git(&tree, &["rev-parse", "HEAD"]).unwrap();
    (tree, c)
}

fn origin_has(fx: &Fixture, branch: &str) -> bool {
    !git(fx, &["ls-remote", "--heads", "origin", branch]).is_empty()
}

fn orphan(fx: &Fixture) {
    let mut r = run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    );
    r.orphan = true;
    fx.add_run(r);
}

#[test]
fn a_dead_sessions_build_released() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    let (tree, c) = built_branch(&fx);
    orphan(&fx);
    let lines = release(&fx.options(), "a").unwrap();
    assert!(!origin_has(&fx, "impl/a"));
    assert!(
        clm_git::git(
            &fx.work,
            &["rev-parse", "--verify", "-q", "refs/heads/impl/a"]
        )
        .is_err()
    );
    let kept = lines
        .iter()
        .find_map(|l| l.strip_prefix("kept "))
        .unwrap_or_else(|| panic!("no kept line in {lines:?}"));
    let time = kept.strip_prefix("released/a/").unwrap();
    assert_eq!(time.len(), 16, "{time}");
    assert_eq!(git(&fx, &["rev-parse", kept]), c);
    assert_eq!(
        clm_git::git(&tree, &["branch", "--show-current"]).unwrap(),
        kept
    );
    assert!(tree.join("notes.txt").exists());
    assert_eq!(fx.ran("branch delete impl/a"), 1);
    assert_eq!(fx.ran("run --finish a"), 1);
    assert!(fx.file().run("build:a").is_none());
}

#[test]
fn a_live_run_is_refused() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    built_branch(&fx);
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    let before = fs::read(fx.loop_dir().join("loop")).unwrap();
    let err = release(&fx.options(), "a").unwrap_err();
    assert!(matches!(err, clm_git::Failure::Refused(_)));
    assert!(err.to_string().contains("build:a"), "{err}");
    assert_eq!(fx.ran("branch delete impl/a"), 0);
    assert!(origin_has(&fx, "impl/a"));
    assert!(
        clm_git::git(
            &fx.work,
            &["rev-parse", "--verify", "-q", "refs/heads/impl/a"]
        )
        .is_ok()
    );
    assert_eq!(fs::read(fx.loop_dir().join("loop")).unwrap(), before);
}

#[test]
fn an_orphan_a_take_over_holds_is_refused() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    built_branch(&fx);
    let mut r = run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    );
    r.orphan = true;
    fx.add_run(r);
    let tip = git(&fx, &["rev-parse", "origin/impl/a"]);
    fx.change_run("build:a", |r| {
        r.hold = Some(clm_loop::Hold {
            tip,
            since: clm_git::now(),
        });
    });
    let err = release(&fx.options(), "a").unwrap_err();
    assert!(err.to_string().contains("build:a"), "{err}");
    assert_eq!(fx.ran("branch delete impl/a"), 0);
}

#[test]
fn a_stopped_run_may_be_released() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    built_branch(&fx);
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Stopped,
        Some("t-1"),
    ));
    release(&fx.options(), "a").unwrap();
    assert!(fx.file().run("build:a").is_none());
}

#[test]
fn another_clones_claim_is_left_alone() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    built_branch(&fx);
    fx.set("branch-exit", "1");
    fx.set("branch-reason", "this clone didn't push the tip");
    let err = release(&fx.options(), "a").unwrap_err();
    assert!(matches!(err, clm_git::Failure::Refused(_)));
    assert!(err.to_string().contains("didn't push the tip"), "{err}");
    assert!(
        clm_git::git(
            &fx.work,
            &["rev-parse", "--verify", "-q", "refs/heads/impl/a"]
        )
        .is_ok()
    );
    assert_eq!(fx.ran("run --finish a"), 0);
}

#[test]
fn a_git_failure_of_the_delete_exits_2() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    fx.set("branch-exit", "2");
    fx.set("branch-reason", "git failed");
    let err = release(&fx.options(), "a").unwrap_err();
    assert!(matches!(err, clm_git::Failure::Git(_)));
}

#[test]
fn nothing_to_release() {
    // req:orchestrator.loop/release@4d5201e5
    let fx = Fixture::new();
    let lines = release(&fx.options(), "a").unwrap();
    assert!(lines.iter().all(|l| !l.starts_with("kept")), "{lines:?}");
}
