//! `clm init` names the rename to `main` first among the next steps, when there is none.

use std::path::Path;
use std::process::Command;

use clm_config::Host;
use clm_init::{Options, init};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn repo(branch: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", branch]);
    dir
}

fn commit(dir: &Path) {
    git(dir, &["commit", "-q", "--allow-empty", "-m", "x"]);
}

/// A repository on `branch` with one commit and `remote` as its `origin`.
fn clone_of(branch: &str, remote: &Path) -> TempDir {
    let dir = repo(branch);
    commit(dir.path());
    git(
        dir.path(),
        &["remote", "add", "origin", remote.to_str().unwrap()],
    );
    dir
}

fn git_host() -> Options {
    Options {
        host: Some(Host::Git),
        ..Options::default()
    }
}

/// The first line under `Next steps (yours):`, when there is one.
fn first_step(out: &str) -> Option<&str> {
    out.split_once("Next steps (yours):\n")?.1.lines().next()
}

const RENAME: &str = "git branch -m master main";

#[test]
fn a_repository_on_master() {
    // req:init/init-names-missing-main@a3a17a23
    let dir = repo("master");
    commit(dir.path());
    let out = init(dir.path(), &Options::default()).unwrap();
    let step = first_step(&out).unwrap();
    assert!(step.contains(RENAME) && step.contains("`main`"), "{step}");
    assert!(!step.contains("CI triggers"), "{step}");
    assert!(dir.path().join("clm.toml").exists());
    let head = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&head.stdout).trim(), "master");
}

#[test]
fn an_empty_repository_whose_unborn_branch_is_master() {
    // req:init/init-names-missing-main@a3a17a23
    let dir = repo("master");
    let out = init(dir.path(), &Options::default()).unwrap();
    assert!(first_step(&out).unwrap().contains(RENAME));
}

#[test]
fn a_remote_on_master() {
    // req:init/init-names-missing-main@a3a17a23
    let remote = repo("master");
    commit(remote.path());
    let dir = clone_of("master", remote.path());
    let out = init(dir.path(), &git_host()).unwrap();
    let step = first_step(&out).unwrap();
    assert!(step.contains(RENAME), "{step}");
    for words in ["default branch", "CI triggers", "branch protection"] {
        assert!(step.contains(words), "{step}");
    }
}

#[test]
fn a_remote_branch_ending_in_main_is_not_main() {
    // req:init/init-names-missing-main@a3a17a23
    let remote = repo("master");
    commit(remote.path());
    git(remote.path(), &["branch", "feature/main"]);
    let dir = clone_of("master", remote.path());
    let out = init(dir.path(), &git_host()).unwrap();
    assert!(first_step(&out).unwrap().contains(RENAME), "{out}");
}

#[test]
fn main_already_there() {
    // req:init/init-names-missing-main@a3a17a23
    let dir = repo("main");
    commit(dir.path());
    git(dir.path(), &["switch", "-q", "-c", "feature"]);
    let out = init(dir.path(), &Options::default()).unwrap();
    assert!(!out.contains("git branch -m"), "{out}");

    // An empty repository on an unborn `main`, likewise.
    let unborn = repo("main");
    let out = init(unborn.path(), &Options::default()).unwrap();
    assert!(!out.contains("git branch -m"), "{out}");

    // With `main` only on the remote, likewise.
    let remote = repo("main");
    commit(remote.path());
    let other = clone_of("master", remote.path());
    let out = init(other.path(), &git_host()).unwrap();
    assert!(!out.contains("git branch -m"), "{out}");
}
