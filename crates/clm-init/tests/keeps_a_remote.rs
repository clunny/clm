//! `clm init` gives a repository on host `none` a remote of its own (`init/init-keeps-a-remote`).

use std::path::{Path, PathBuf};
use std::process::Command;

use clm_config::Host;
use clm_init::{Options, init};
use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    dir
}

fn bare_path(dir: &Path) -> PathBuf {
    let common = git(
        dir,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    Path::new(&common).join("clm").join("remote.git")
}

#[test]
fn empty_repository_with_no_remote() {
    // req:init/init-keeps-a-remote@c228f421
    let dir = repo();
    let out = init(dir.path(), &Options::default()).unwrap();
    let toml = std::fs::read_to_string(dir.path().join("clm.toml")).unwrap();
    assert!(toml.contains("host = \"none\""), "{toml}");
    let bare = bare_path(dir.path());
    let url = git(dir.path(), &["remote", "get-url", "origin"]);
    assert_eq!(Path::new(&url), bare);
    assert!(git(&bare, &["branch", "--list"]).is_empty());
    assert!(out.contains(bare.to_str().unwrap()), "{out}");
}

#[test]
fn a_remote_already_there() {
    // req:init/init-keeps-a-remote@c228f421
    let dir = repo();
    git(
        dir.path(),
        &["remote", "add", "origin", "https://github.com/o/r.git"],
    );
    init(dir.path(), &Options::default()).unwrap();
    assert_eq!(
        git(dir.path(), &["remote", "get-url", "origin"]),
        "https://github.com/o/r.git"
    );
    assert!(!bare_path(dir.path()).exists());

    let other = repo();
    let opts = Options {
        host: Some(Host::Git),
        ..Options::default()
    };
    init(other.path(), &opts).unwrap();
    assert!(git(other.path(), &["remote"]).is_empty());
    assert!(!bare_path(other.path()).exists());
}
