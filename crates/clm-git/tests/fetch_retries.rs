//! `fetch_retrying`: a fake `git` first on this process's `PATH` fails `fetch` as each
//! clone's `.git/fake-mode` says and counts the calls it sees in `.git/fake-calls`.

mod common;
mod exec;

use clm_git::{FetchError, REMOTE, fetch_retrying};
use common::sh;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
dir=$("$real" rev-parse --absolute-git-dir)
case "$1" in
fetch|ls-remote)
  echo "$1" >> "$dir/fake-calls"
  mode=$(cat "$dir/fake-mode" 2>/dev/null)
  if [ "$1" = fetch ]; then
    n=$(grep -c '^fetch$' "$dir/fake-calls")
    if [ "$mode" = always ] || { [ "$mode" = once ] && [ "$n" = 1 ]; }; then
      echo "error: cannot lock ref 'refs/remotes/origin/main'" >&2
      exit 1
    fi
  fi
  ;;
esac
exec "$real" "$@"
"#;

/// A clone of a bare `origin`, with the fake's mode set.
fn clone_with(mode: &str) -> (tempfile::TempDir, PathBuf) {
    common::install_fake_git(FAKE, exec::write_exec);
    let dir = tempfile::tempdir().unwrap();
    let origin = dir.path().join("origin.git");
    let seed = dir.path().join("seed");
    std::fs::create_dir(&seed).unwrap();
    common::repo(&seed, &[("a", "a")]);
    sh(
        dir.path(),
        &[
            "clone",
            "-q",
            "--bare",
            seed.to_str().unwrap(),
            origin.to_str().unwrap(),
        ],
    );
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
    std::fs::write(clone.join(".git/fake-mode"), mode).unwrap();
    (dir, clone)
}

fn calls(clone: &Path, what: &str) -> usize {
    common::count_calls(&clone.join(".git/fake-calls"), what)
}

#[test]
fn a_fetch_that_loses_a_ref_race_goes_on() {
    // req:vcs/fetch-retries@f4f74e93
    let (_dir, clone) = clone_with("once");
    let start = Instant::now();
    fetch_retrying(&clone, &["fetch", REMOTE]).unwrap();
    // The waits' lengths and order are the unit test's: a bound above would fail on a loaded
    // host.
    assert!(start.elapsed() >= Duration::from_secs(1));
    assert_eq!(calls(&clone, "fetch"), 2);
}

#[test]
fn a_fetch_that_keeps_failing_with_origin_reachable_is_a_git_error() {
    // req:vcs/fetch-retries@f4f74e93
    let (_dir, clone) = clone_with("always");
    let start = Instant::now();
    let err = fetch_retrying(&clone, &["fetch", REMOTE]).unwrap_err();
    assert!(start.elapsed() >= Duration::from_secs(3));
    assert!(matches!(err, FetchError::Git(_)), "{err}");
    assert!(err.to_string().contains("cannot lock ref"), "{err}");
    assert_eq!(calls(&clone, "fetch"), 3);
    assert_eq!(calls(&clone, "ls-remote"), 1);
}

#[test]
fn an_unreachable_origin_is_offline() {
    // req:vcs/fetch-retries@f4f74e93
    let (dir, clone) = clone_with("");
    sh(
        &clone,
        &[
            "remote",
            "set-url",
            REMOTE,
            dir.path().join("missing").to_str().unwrap(),
        ],
    );
    let err = fetch_retrying(&clone, &["fetch", REMOTE]).unwrap_err();
    assert!(matches!(err, FetchError::Offline), "{err}");
}

#[test]
fn what_being_offline_says() {
    // req:vcs/fetch-retries@f4f74e93
    let (dir, clone) = clone_with("");
    sh(
        &clone,
        &[
            "remote",
            "set-url",
            REMOTE,
            dir.path().join("missing").to_str().unwrap(),
        ],
    );
    let err = fetch_retrying(&clone, &["fetch", REMOTE]).unwrap_err();
    assert_eq!(err.to_string(), "origin can't be reached");
}

#[test]
fn a_root_that_doesnt_exist_names_it_at_once() {
    // req:vcs/fetch-retries@f4f74e93
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("x");
    let start = Instant::now();
    let err = fetch_retrying(&missing, &["fetch", REMOTE]).unwrap_err();
    assert!(start.elapsed() < Duration::from_secs(1));
    assert!(matches!(err, FetchError::Git(_)), "{err}");
    let shown = err.to_string();
    assert!(shown.starts_with("git fetch origin in "), "{shown}");
    // ENOENT is 2 on Linux and macOS, the systems CLM runs on.
    let io = std::io::Error::from_raw_os_error(2);
    assert_eq!(
        shown,
        format!("git fetch origin in {}: {io}", missing.display())
    );
    assert!(!shown.contains("can't be reached"), "{shown}");
}

/// A branch `b` on the bare origin that the clone hasn't fetched.
fn push_branch_b(dir: &Path, clone: &Path) {
    let other = dir.join("other");
    sh(
        dir,
        &[
            "clone",
            "-q",
            dir.join("origin.git").to_str().unwrap(),
            other.to_str().unwrap(),
        ],
    );
    sh(&other, &["push", "-q", "origin", "HEAD:refs/heads/b"]);
    assert!(!clone.join(".git/refs/remotes/origin/b").exists());
}

#[test]
fn a_plain_fetch_that_loses_a_ref_race_goes_on() {
    // req:vcs/fetch-tries-again@3a1fd640
    let (dir, clone) = clone_with("once");
    push_branch_b(dir.path(), &clone);
    let start = Instant::now();
    clm_git::fetch(&clone).unwrap();
    assert!(start.elapsed() >= Duration::from_secs(1));
    sh(&clone, &["rev-parse", "--verify", "refs/remotes/origin/b"]);
    assert_eq!(calls(&clone, "fetch"), 2);
}

#[test]
fn a_plain_fetch_that_keeps_failing_returns_the_last_error() {
    // req:vcs/fetch-tries-again@3a1fd640
    let (_dir, clone) = clone_with("always");
    let start = Instant::now();
    let err = clm_git::fetch(&clone).unwrap_err();
    assert!(start.elapsed() >= Duration::from_secs(3));
    assert!(err.to_string().contains("cannot lock ref"), "{err}");
    assert_eq!(calls(&clone, "fetch"), 3);
    assert_eq!(calls(&clone, "ls-remote"), 0);
}
