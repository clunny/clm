#![cfg(unix)]
// The only test in this binary, so setting the process's environment disturbs no other.

mod common;

use clm_git::{git, git_isolated};
use common::sh;

#[test]
fn bare_repository_beside_a_named_clone() {
    // req:vcs/git-isolated@5e69c07c
    let dir = tempfile::tempdir().unwrap();
    let w = dir.path().join("w");
    let g = dir.path().join("g");
    let b = dir.path().join("b");
    sh(
        dir.path(),
        &[
            "init",
            "-q",
            "--separate-git-dir",
            g.to_str().unwrap(),
            w.to_str().unwrap(),
        ],
    );
    let before = std::fs::read_dir(&g).unwrap().count();
    // SAFETY: no other thread of this binary reads or writes the environment.
    unsafe {
        std::env::set_var("GIT_DIR", &g);
        std::env::set_var("GIT_WORK_TREE", &w);
    }
    let args = ["init", "--quiet", "--bare", b.to_str().unwrap()];
    let err = git(&w, &args).unwrap_err();
    assert!(
        err.to_string()
            .contains("not allowed without specifying GIT_DIR"),
        "{err}"
    );
    git_isolated(&w, &args).unwrap();
    unsafe {
        std::env::remove_var("GIT_DIR");
        std::env::remove_var("GIT_WORK_TREE");
    }
    assert_eq!(
        sh(
            dir.path(),
            &[
                "--git-dir",
                b.to_str().unwrap(),
                "rev-parse",
                "--is-bare-repository"
            ]
        ),
        "true"
    );
    assert_eq!(std::fs::read_dir(&g).unwrap().count(), before);
}
