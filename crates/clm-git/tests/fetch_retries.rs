//! `fetch_retrying`: a fake `git` first on this process's `PATH` fails `fetch` as each
//! clone's `.git/fake-mode` says and counts the calls it sees in `.git/fake-calls`.

mod common;
mod exec;

use clm_git::{FetchError, REMOTE, fetch_retrying};
use common::sh;
use std::path::{Path, PathBuf};
use std::sync::Once;
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

fn install_fake() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let bin = Box::leak(Box::new(tempfile::tempdir().unwrap()));
        exec::write_exec(&bin.path().join("git"), FAKE);
        let real = std::env::var("PATH").unwrap();
        let path = format!("{}:{real}", bin.path().display());
        // SAFETY: every test here calls this first, and `Once` blocks the others until the
        // variables are set, before any of them starts a command.
        unsafe {
            std::env::set_var("FAKE_REAL_PATH", real);
            std::env::set_var("PATH", path);
        }
    });
}

/// A clone of a bare `origin`, with the fake's mode set.
fn clone_with(mode: &str) -> (tempfile::TempDir, PathBuf) {
    install_fake();
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
    std::fs::read_to_string(clone.join(".git/fake-calls"))
        .unwrap_or_default()
        .lines()
        .filter(|l| *l == what)
        .count()
}

#[test]
fn a_fetch_that_loses_a_ref_race_goes_on() {
    // req:vcs/fetch-retries@a74172a3
    let (_dir, clone) = clone_with("once");
    let start = Instant::now();
    fetch_retrying(&clone, &["fetch", REMOTE]).unwrap();
    assert!(start.elapsed() >= Duration::from_secs(1));
    // The first wait is 1 second, not the 2 before the second retry.
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(calls(&clone, "fetch"), 2);
}

#[test]
fn a_fetch_that_keeps_failing_with_origin_reachable_is_a_git_error() {
    // req:vcs/fetch-retries@a74172a3
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
    // req:vcs/fetch-retries@a74172a3
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
