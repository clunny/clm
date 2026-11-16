//! `worktrees`: a fake `git` first on this process's `PATH` fails `worktree list` as each
//! repository's `.git/fake-mode` says, as git does while another process is adding a
//! worktree, and counts the listings it sees in `.git/fake-calls`.

mod common;
mod exec;

use std::path::{Path, PathBuf};
use std::sync::Once;
use std::time::{Duration, Instant};

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
if [ "$1" = worktree ] && [ "$2" = list ]; then
  dir=$("$real" rev-parse --git-common-dir)
  echo list >> "$dir/fake-calls"
  mode=$(cat "$dir/fake-mode" 2>/dev/null)
  n=$(grep -c '^list$' "$dir/fake-calls")
  if [ "$mode" = always ] || { [ "$mode" = once ] && [ "$n" = 1 ]; }; then
    echo "fatal: failed to read .git/worktrees/half/commondir: Success" >&2
    exit 128
  fi
fi
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

/// A repository with a linked worktree on branch `impl/a`, with the fake's mode set.
fn repo_with(mode: &str) -> (tempfile::TempDir, PathBuf, PathBuf) {
    install_fake();
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
    std::fs::read_to_string(root.join(".git/fake-calls"))
        .unwrap_or_default()
        .lines()
        .filter(|l| *l == "list")
        .count()
}

#[test]
fn a_listing_that_meets_a_worktree_being_added_goes_on() {
    // req:vcs/worktrees-tries-again@17533c62
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
    // req:vcs/worktrees-tries-again@17533c62
    let (_dir, root, _linked) = repo_with("always");
    let start = Instant::now();
    let err = clm_git::worktrees(&root).unwrap_err();
    assert!(start.elapsed() >= Duration::from_millis(600));
    assert!(err.to_string().contains("commondir"), "{err}");
    assert_eq!(listings(&root), 3);
}
