mod common;

use clm_git::{Error, Failure, REMOTE, fetch, git, now};
use common::{commit, repo, sh};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn the_remote_is_origin() {
    // req:vcs/remote-name@bd3b825d
    assert_eq!(REMOTE, "origin");
}

#[test]
fn fetch_fetches_the_remote() {
    // req:vcs/remote-name@bd3b825d
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin");
    std::fs::create_dir(&origin).unwrap();
    repo(&origin, &[]);
    let clone = dir.path().join("clone");
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            origin.to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    sh(&origin, &["switch", "-q", "-c", "b"]);
    let b = commit(&origin, &[("b", "b")], "b");
    fetch(&clone).unwrap();
    assert_eq!(sh(&clone, &["rev-parse", "refs/remotes/origin/b"]), b);
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[test]
fn now_reads_the_clock() {
    // req:vcs/unix-now@071454d8
    let before = epoch_seconds();
    let now = now();
    let after = epoch_seconds();
    assert!(before <= now && now <= after, "{before} {now} {after}");
}

#[test]
fn refused() {
    // req:vcs/failure@7eeb38e6
    let failure = Failure::Refused("not on a branch".into());
    assert_eq!(failure.to_string(), "not on a branch");
    assert_eq!(failure.exit_code(), 1);
}

#[test]
fn git_failed() {
    // req:vcs/failure@7eeb38e6
    let dir = tempfile::tempdir().unwrap();
    let error: Error = git(dir.path(), &["status"]).unwrap_err();
    let shown = error.to_string();
    let converted = || -> Result<(), Failure> {
        git(dir.path(), &["status"])?;
        Ok(())
    };
    let failure = converted().unwrap_err();
    assert!(
        matches!(&failure, Failure::Git(m) if *m == shown),
        "{failure:?}"
    );
    assert_eq!(failure.to_string(), shown);
    assert!(shown.contains("git status"), "{shown}");
    assert_eq!(failure.exit_code(), 2);
    let _: &dyn std::error::Error = &failure;
}
