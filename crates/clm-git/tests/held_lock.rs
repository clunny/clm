mod common;

use std::path::{Path, PathBuf};

use clm_git::{Worktree, worktrees};
use common::{repo, sh};

/// A repository at `<dir>/repo` with a linked worktree at `<dir>/<name>` for each of
/// `reasons`, locked with that reason, or with none for `None`.
fn locked_worktrees(dir: &Path, reasons: &[Option<&str>]) -> (PathBuf, Vec<PathBuf>) {
    let root = dir.join("repo");
    std::fs::create_dir(&root).unwrap();
    repo(&root, &[]);
    let mut paths = Vec::new();
    for (i, reason) in reasons.iter().enumerate() {
        let path = dir.join(format!("w{i}"));
        let p = path.to_str().unwrap();
        sh(&root, &["worktree", "add", "-q", "--detach", p]);
        match reason {
            Some(reason) => sh(&root, &["worktree", "lock", "--reason", reason, p]),
            None => sh(&root, &["worktree", "lock", p]),
        };
        paths.push(path);
    }
    (root, paths)
}

/// The entry `list` holds for `path`, comparing with symbolic links resolved.
fn entry<'a>(list: &'a [Worktree], path: &Path) -> &'a Worktree {
    let path = path.canonicalize().unwrap();
    list.iter()
        .find(|w| w.path.canonicalize().unwrap() == path)
        .unwrap_or_else(|| panic!("{path:?} not in {list:?}"))
}

fn agent_reason(pid: impl std::fmt::Display) -> String {
    format!("claude agent agent-1 (pid {pid} start 1)")
}

#[test]
fn a_running_process_holds_its_lock() {
    // req:vcs/held-lock@c8d3683a
    let dir = tempfile::tempdir().unwrap();
    let reason = agent_reason(std::process::id());
    let (root, paths) = locked_worktrees(dir.path(), &[Some(&reason)]);
    let list = worktrees(&root).unwrap();
    let w = entry(&list, &paths[0]);
    assert!(w.locked && w.held, "{w:?}");
    assert!(!entry(&list, &root).held, "{list:?}");
}

/// Unix only: elsewhere a lock naming a process ID is held whether or not it runs.
#[cfg(unix)]
#[test]
fn an_ended_process_holds_no_lock() {
    // req:vcs/held-lock@c8d3683a
    let dir = tempfile::tempdir().unwrap();
    let (root, paths) = locked_worktrees(dir.path(), &[Some("placeholder")]);
    let p = paths[0].to_str().unwrap();
    sh(&root, &["worktree", "unlock", p]);
    // The child is reaped just before the listing, so its ID has little time to be reused.
    let mut child = std::process::Command::new("true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    sh(
        &root,
        &["worktree", "lock", "--reason", &agent_reason(pid), p],
    );
    let list = worktrees(&root).unwrap();
    let w = entry(&list, &paths[0]);
    assert!(w.locked && !w.held, "{w:?}");
}

#[cfg(unix)]
#[test]
fn another_users_process_holds_its_lock() {
    // req:vcs/held-lock@c8d3683a
    let dir = tempfile::tempdir().unwrap();
    let reason = agent_reason(1);
    let (root, paths) = locked_worktrees(dir.path(), &[Some(&reason)]);
    let list = worktrees(&root).unwrap();
    let w = entry(&list, &paths[0]);
    assert!(w.locked && w.held, "{w:?}");
}

#[test]
fn a_lock_naming_no_process_is_not_held() {
    // req:vcs/held-lock@c8d3683a
    let dir = tempfile::tempdir().unwrap();
    let reasons = [
        None,
        Some("kept by hand"),
        Some("rapid 1"),
        Some("pid 0"),
        Some("pid 99999999999999999999"),
        Some("pid x1"),
    ];
    let (root, paths) = locked_worktrees(dir.path(), &reasons);
    let list = worktrees(&root).unwrap();
    for (path, reason) in paths.iter().zip(reasons) {
        let w = entry(&list, path);
        assert!(w.locked && !w.held, "{reason:?}: {w:?}");
    }
}
