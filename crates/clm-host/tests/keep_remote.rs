//! `keep_remote` (host/none-keeps-a-remote).

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_config::Host;
use clm_host::keep_remote;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@example.com")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@example.com")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// A clone with a commit on `main` and no remote, in a temporary directory.
fn clone() -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("clone");
    std::fs::create_dir_all(&dir).unwrap();
    let dir = dir.canonicalize().unwrap();
    git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("a.txt"), "a").unwrap();
    git(&dir, &["add", "a.txt"]);
    git(&dir, &["commit", "-q", "-m", "first"]);
    (tmp, dir)
}

fn kept(dir: &Path) -> PathBuf {
    PathBuf::from(git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ))
    .join("clm")
    .join("remote.git")
}

#[test]
fn no_remote() {
    // req:host/none-keeps-a-remote@85ff1b8b
    let (_tmp, dir) = clone();
    let status_before = git(&dir, &["status", "--porcelain"]);
    let got = keep_remote(&dir, Host::None).unwrap();
    let bare = kept(&dir);
    assert_eq!(got, Some(bare.clone()));
    assert_eq!(git(&bare, &["rev-parse", "--is-bare-repository"]), "true");
    assert_eq!(
        git(&dir, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
    assert_eq!(git(&bare, &["for-each-ref"]), "");
    assert_eq!(git(&dir, &["status", "--porcelain"]), status_before);
}

#[test]
fn from_a_linked_worktree() {
    // req:host/none-keeps-a-remote@85ff1b8b
    let (tmp, dir) = clone();
    let linked = tmp.path().join("linked");
    git(
        &dir,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "other",
            linked.to_str().unwrap(),
        ],
    );
    let got = keep_remote(&linked, Host::None).unwrap();
    let bare = kept(&dir);
    assert_eq!(got, Some(bare.clone()));
    assert!(bare.join("HEAD").exists());
    assert_eq!(
        git(&linked, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
    assert_eq!(
        git(&dir, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
    assert_eq!(git(&linked, &["status", "--porcelain"]), "");
}

#[test]
fn already_kept() {
    // req:host/none-keeps-a-remote@85ff1b8b
    let (_tmp, dir) = clone();
    keep_remote(&dir, Host::None).unwrap();
    let bare = kept(&dir);
    git(&dir, &["push", "-q", "origin", "main"]);
    git(&dir, &["remote", "remove", "origin"]);
    let got = keep_remote(&dir, Host::None).unwrap();
    assert_eq!(got, Some(bare.clone()));
    assert_eq!(
        git(&dir, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
    assert_eq!(
        git(&bare, &["rev-parse", "main"]),
        git(&dir, &["rev-parse", "main"])
    );
}

#[test]
fn nothing_to_do() {
    // req:host/none-keeps-a-remote@85ff1b8b
    let (tmp, dir) = clone();
    let other = tmp.path().join("other.git");
    git(
        tmp.path(),
        &["init", "-q", "--bare", other.to_str().unwrap()],
    );
    git(&dir, &["remote", "add", "origin", other.to_str().unwrap()]);
    assert_eq!(keep_remote(&dir, Host::None).unwrap(), None);
    assert_eq!(
        git(&dir, &["remote", "get-url", "origin"]),
        other.to_str().unwrap()
    );
    assert!(!kept(&dir).exists());

    for host in [Host::Git, Host::Github, Host::Azure] {
        let (_tmp2, dir2) = clone();
        assert_eq!(keep_remote(&dir2, host).unwrap(), None);
        assert_eq!(git(&dir2, &["remote"]), "");
        assert!(!kept(&dir2).exists());
    }
}
