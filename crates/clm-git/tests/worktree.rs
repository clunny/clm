#![cfg(unix)]

mod common;

use clm_git::{Error, remove_worktree};
use common::{repo, sh};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

fn setup() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("r");
    std::fs::create_dir(&root).unwrap();
    repo(&root, &[("a.txt", "a\n"), (".gitignore", "cache/\nout\n")]);
    let w = dir.path().join("w");
    sh(
        &root,
        &["worktree", "add", "-q", "-b", "wb", w.to_str().unwrap()],
    );
    (dir, w)
}

fn mode(p: &Path) -> u32 {
    std::fs::metadata(p).unwrap().permissions().mode() & 0o777
}

#[test]
fn unwritable_directory_removed() {
    // req:vcs/remove-worktree@ff9f14fb
    let (dir, w) = setup();
    let root = dir.path().join("r");
    sh(&root, &["worktree", "lock", w.to_str().unwrap()]);
    std::fs::create_dir_all(w.join("cache/deep")).unwrap();
    std::fs::write(w.join("cache/deep/f"), "x").unwrap();
    // No access at all: only a chmod made before the entries are read can reach `f`.
    std::fs::set_permissions(w.join("cache/deep"), std::fs::Permissions::from_mode(0o000)).unwrap();
    std::fs::set_permissions(w.join("cache"), std::fs::Permissions::from_mode(0o500)).unwrap();
    remove_worktree(&root, &w).unwrap();
    assert!(!w.exists());
    assert!(!sh(&root, &["worktree", "list"]).contains("/w "));
}

#[test]
fn symbolic_link_not_followed() {
    // req:vcs/remove-worktree@ff9f14fb
    let (dir, w) = setup();
    let root = dir.path().join("r");
    let d = dir.path().join("d");
    std::fs::create_dir(&d).unwrap();
    std::fs::write(d.join("f"), "x").unwrap();
    std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o500)).unwrap();
    std::os::unix::fs::symlink(&d, w.join("out")).unwrap();
    remove_worktree(&root, &w).unwrap();
    assert!(!w.exists());
    assert_eq!(mode(&d), 0o500);
    std::fs::set_permissions(&d, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(d.join("f").exists());
}

#[test]
fn git_refuses() {
    // req:vcs/remove-worktree@ff9f14fb
    let (dir, w) = setup();
    let root = dir.path().join("r");
    std::fs::write(w.join("untracked.txt"), "u").unwrap();
    let err = remove_worktree(&root, &w).unwrap_err();
    assert!(err.to_string().contains("git worktree remove"), "{err}");
    assert!(w.join("untracked.txt").exists());
}

#[test]
fn still_on_disk_displays_path() {
    // req:vcs/remove-worktree@ff9f14fb
    let e = Error::StillOnDisk {
        path: "/x/w".into(),
    };
    assert_eq!(e.to_string(), "/x/w: still on disk");
}
