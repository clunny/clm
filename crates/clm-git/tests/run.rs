mod common;

use clm_git::{Error, Options, git, run};
use common::{repo, sh};
use std::ffi::OsStr;

#[test]
fn check_attr_reads_stdin_and_keeps_nul_separated_output() {
    // req:vcs/raw-runs@9264ba25
    let dir = tempfile::tempdir().unwrap();
    repo(
        dir.path(),
        &[(".gitattributes", "a.lock linguist-generated\n")],
    );
    let out = run(
        dir.path(),
        "git",
        &["check-attr", "--stdin", "-z", "linguist-generated"],
        Options {
            stdin: Some(b"a.lock\0"),
            ..Options::default()
        },
    )
    .unwrap();
    assert_eq!(out, b"a.lock\0linguist-generated\0set\0");
}

#[test]
fn run_returns_stdout_untrimmed() {
    // req:vcs/raw-runs@9264ba25
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    let out = run(
        dir.path(),
        "git",
        &["rev-parse", "--abbrev-ref", "HEAD"],
        Options::default(),
    );
    assert_eq!(out.unwrap(), b"main\n");
}

#[test]
fn run_sets_the_environment() {
    // req:vcs/raw-runs@9264ba25
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[("a.txt", "a\n")]);
    std::fs::write(dir.path().join("b.txt"), "b\n").unwrap();
    sh(dir.path(), &["add", "b.txt"]);
    let index_before = std::fs::read(dir.path().join(".git/index")).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let idx = scratch.path().join("idx");
    run(
        dir.path(),
        "git",
        &["read-tree", "HEAD"],
        Options {
            env: &[("GIT_INDEX_FILE", idx.as_os_str())],
            ..Options::default()
        },
    )
    .unwrap();
    assert!(idx.exists());
    assert_eq!(
        std::fs::read(dir.path().join(".git/index")).unwrap(),
        index_before
    );
}

#[test]
fn run_fails_like_git() {
    // req:vcs/raw-runs@9264ba25
    let dir = tempfile::tempdir().unwrap();
    let raw = run(dir.path(), "git", &["status"], Options::default()).unwrap_err();
    let wrapped = git(dir.path(), &["status"]).unwrap_err();
    assert_eq!(raw.to_string(), wrapped.to_string());
}

#[test]
fn git_returns_trimmed_stdout() {
    // req:vcs/run-git@32c20165
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    assert_eq!(
        git(dir.path(), &["rev-parse", "--abbrev-ref", "HEAD"]).unwrap(),
        "main"
    );
}

#[test]
fn failure_names_the_command_and_holds_stderr() {
    // req:vcs/run-git@32c20165
    let dir = tempfile::tempdir().unwrap();
    let err = git(dir.path(), &["status"]).unwrap_err();
    assert!(matches!(err, Error::Failed { .. }));
    let shown = err.to_string();
    assert!(shown.starts_with("git status: "), "{shown}");
    assert!(shown.contains("not a git repository"), "{shown}");
    assert!(!shown.ends_with('\n'), "{shown:?}");
}

#[test]
fn failure_without_stderr_shows_the_status() {
    // req:vcs/run-git@32c20165
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), &[]);
    // `diff --quiet` with a change exits 1 and prints nothing.
    std::fs::write(dir.path().join("x"), "x").unwrap();
    sh(dir.path(), &["add", "x"]);
    let err = git(dir.path(), &["diff", "--cached", "--quiet"]).unwrap_err();
    let shown = err.to_string();
    assert!(shown.starts_with("git diff --cached --quiet: "), "{shown}");
    assert!(shown.contains("exit status: 1"), "{shown}");
}

#[test]
fn a_program_that_cannot_start_is_a_spawn_error() {
    // req:vcs/run-git@32c20165
    let dir = tempfile::tempdir().unwrap();
    let err = run(
        dir.path(),
        "git",
        &["status"],
        Options {
            env: &[("PATH", OsStr::new(""))],
            ..Options::default()
        },
    )
    .unwrap_err();
    assert!(matches!(err, Error::Spawn { .. }), "{err:?}");
    assert!(err.to_string().starts_with("git status: "), "{err}");
}
