#![cfg(unix)]

mod common;

use clm_git::git_isolated;
use common::repo;

#[test]
fn same_as_git_otherwise() {
    // req:vcs/git-isolated@5e69c07c
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("a", "a\n")]);
    assert_eq!(
        git_isolated(dir.path(), &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap(),
        "main"
    );
    let err = git_isolated(dir.path(), &["no-such-command"]).unwrap_err();
    assert!(err.to_string().contains("git no-such-command"), "{err}");
}
