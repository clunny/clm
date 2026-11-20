//! `worktrees`: a fake `git` first on this process's `PATH` fails `worktree list` as each
//! repository's `.git/fake-mode` says, as git does while another process is adding a
//! worktree, and counts the listings it sees in `.git/fake-calls`. Each failing run also
//! prints its number on a second line, so a test can tell which run's error it got.

mod common;
mod exec;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
if [ "$1" = worktree ] && [ "$2" = list ]; then
  dir=$("$real" rev-parse --path-format=absolute --git-common-dir)
  echo list >> "$dir/fake-calls"
  mode=$(cat "$dir/fake-mode" 2>/dev/null)
  n=$(grep -c '^list$' "$dir/fake-calls")
  if [ "$mode" = always ] || { [ "$mode" = once ] && [ "$n" = 1 ]; }; then
    echo "fatal: failed to read .git/worktrees/half/commondir: Success" >&2
    echo "run $n" >&2
    exit 128
  fi
fi
exec "$real" "$@"
"#;

/// A repository with a linked worktree on branch `impl/a`, with the fake's mode set.
fn repo_with(mode: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    common::install_fake_git(FAKE, exec::write_exec);
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    std::fs::create_dir(&root).unwrap();
    common::repo(&root, &[("a", "a")]);
    let linked = dir.path().join("linked");
    common::sh(
        &root,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            linked.to_str().unwrap(),
        ],
    );
    std::fs::write(root.join(".git/fake-mode"), mode).unwrap();
    (dir, root, linked)
}

fn listings(root: &Path) -> usize {
    common::count_calls(&root.join(".git/fake-calls"), "list")
}

#[test]
fn a_listing_that_meets_a_worktree_being_added_goes_on() {
    // req:vcs/worktrees-tries-again@c983d508
    let (_dir, root, linked) = repo_with("once");
    let start = Instant::now();
    let list = clm_git::worktrees(&root).unwrap();
    assert!(start.elapsed() >= Duration::from_millis(100));
    assert_eq!(list.len(), 2, "{list:?}");
    let linked = linked.canonicalize().unwrap();
    let entry = list
        .iter()
        .find(|w| w.path.canonicalize().ok().as_ref() == Some(&linked))
        .expect("the linked worktree is listed");
    assert_eq!(entry.branch.as_deref(), Some("impl/a"));
    assert_eq!(listings(&root), 2);
}

#[test]
fn a_listing_that_keeps_failing_returns_the_last_error() {
    // req:vcs/worktrees-tries-again@c983d508
    let (_dir, root, _linked) = repo_with("always");
    let start = Instant::now();
    let err = clm_git::worktrees(&root).unwrap_err().to_string();
    assert!(start.elapsed() >= Duration::from_millis(600));
    assert!(err.contains("commondir"), "{err}");
    assert!(err.ends_with("run 3"), "the last run's error: {err}");
    assert_eq!(listings(&root), 3);
}
