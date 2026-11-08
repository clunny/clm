//! `keep_remote` on a clone the environment names (host/none-keeps-a-remote). Its own test
//! binary, since it sets `GIT_DIR` and `GIT_WORK_TREE` for the whole process.

use std::path::Path;
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

#[test]
fn a_clone_the_environment_names() {
    // req:host/none-keeps-a-remote@7c60711b
    let tmp = tempfile::tempdir().unwrap();
    let base = tmp.path().canonicalize().unwrap();
    let w = base.join("w");
    let g = base.join("g");
    std::fs::create_dir_all(&w).unwrap();
    git(
        &base,
        &[
            "init",
            "-q",
            "-b",
            "main",
            "--separate-git-dir",
            g.to_str().unwrap(),
            w.to_str().unwrap(),
        ],
    );
    std::fs::write(w.join("a.txt"), "a").unwrap();
    git(&w, &["add", "a.txt"]);
    git(&w, &["commit", "-q", "-m", "first"]);

    // SAFETY: this binary holds one test, so nothing else reads the environment meanwhile.
    unsafe {
        std::env::set_var("GIT_DIR", &g);
        std::env::set_var("GIT_WORK_TREE", &w);
    }
    let got = keep_remote(&w, Host::None).unwrap();
    unsafe {
        std::env::remove_var("GIT_DIR");
        std::env::remove_var("GIT_WORK_TREE");
    }

    let bare = g.join("clm").join("remote.git");
    assert_eq!(got, Some(bare.clone()));
    assert_eq!(git(&bare, &["rev-parse", "--is-bare-repository"]), "true");
    assert_eq!(
        git(&w, &["remote", "get-url", "origin"]),
        bare.to_str().unwrap()
    );
}
