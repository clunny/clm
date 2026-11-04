//! `run_within`: each test puts its fake `gh` first on the `PATH` it hands the command.

mod exec;

use clm_git::{Error, Options, git, run_within};
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;
use std::time::{Duration, Instant};

/// A directory holding an executable `gh` running `script`, and the `PATH` that finds it first.
fn fake_gh(script: &str) -> (tempfile::TempDir, OsString) {
    let bin = tempfile::tempdir().unwrap();
    exec::write_exec(&bin.path().join("gh"), &format!("#!/bin/sh\n{script}"));
    let mut path = OsString::from(bin.path());
    path.push(":");
    path.push(std::env::var_os("PATH").unwrap());
    (bin, path)
}

fn gh_within(
    root: &Path,
    path: &OsString,
    args: &[&str],
    limit: Duration,
) -> Result<Vec<u8>, Error> {
    let env = [("PATH", path.as_os_str())];
    let options = Options {
        env: &env,
        ..Options::default()
    };
    run_within(root, "gh", args, options, limit)
}

#[test]
fn a_command_that_finishes_in_time() {
    // req:vcs/bounded-runs@9e2c536e
    let (_bin, path) = fake_gh("echo '{\"state\":\"MERGED\"}'\n");
    let root = tempfile::tempdir().unwrap();
    let out = gh_within(
        root.path(),
        &path,
        &["pr", "view", "1"],
        Duration::from_secs(5),
    );
    assert_eq!(out.unwrap(), b"{\"state\":\"MERGED\"}\n");
}

#[test]
fn stdin_is_fed_within_a_limit() {
    // req:vcs/bounded-runs@9e2c536e
    let (_bin, path) = fake_gh("cat\n");
    let root = tempfile::tempdir().unwrap();
    let env = [("PATH", path.as_os_str())];
    let input = vec![b'x'; 1 << 20];
    let options = Options {
        stdin: Some(&input),
        env: &env,
    };
    let out = run_within(root.path(), "gh", &["x"], options, Duration::from_secs(5));
    assert_eq!(out.unwrap(), input);
}

#[test]
fn a_command_that_stalls() {
    // req:vcs/bounded-runs@9e2c536e
    let root = tempfile::tempdir().unwrap();
    let pid_file = root.path().join("pid");
    let (_bin, path) = fake_gh(&format!(
        "echo $$ > '{}'\nexec sleep 30\n",
        pid_file.display()
    ));
    let start = Instant::now();
    let err = gh_within(root.path(), &path, &["api", "x"], Duration::from_secs(1)).unwrap_err();
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "{:?}",
        start.elapsed()
    );
    assert!(matches!(err, Error::TimedOut { .. }), "{err:?}");
    assert_eq!(err.to_string(), "gh api x: timed out after 1s");
    let pid = std::fs::read_to_string(&pid_file).unwrap();
    let alive = Command::new("kill")
        .args(["-0", pid.trim()])
        .output()
        .unwrap();
    assert!(
        !alive.status.success(),
        "process {} still exists",
        pid.trim()
    );
}

#[test]
fn a_child_keeps_the_pipes_open() {
    // req:vcs/bounded-runs@9e2c536e
    let (_bin, path) = fake_gh("sleep 30 &\nsleep 30\n");
    let root = tempfile::tempdir().unwrap();
    let start = Instant::now();
    let err = gh_within(
        root.path(),
        &path,
        &["api", "x"],
        Duration::from_millis(200),
    )
    .unwrap_err();
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(err.to_string(), "gh api x: timed out after 200ms");
}

#[test]
fn a_command_that_exits_but_leaves_the_pipes_open() {
    // req:vcs/bounded-runs@9e2c536e
    let (_bin, path) = fake_gh("sleep 30 &\necho started\n");
    let root = tempfile::tempdir().unwrap();
    let start = Instant::now();
    let err = gh_within(
        root.path(),
        &path,
        &["api", "x"],
        Duration::from_millis(300),
    )
    .unwrap_err();
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "{:?}",
        start.elapsed()
    );
    assert_eq!(err.to_string(), "gh api x: timed out after 300ms");
}

#[test]
fn no_time_left() {
    // req:vcs/bounded-runs@9e2c536e
    let root = tempfile::tempdir().unwrap();
    let ran = root.path().join("ran");
    let (_bin, path) = fake_gh(&format!("touch '{}'\n", ran.display()));
    let err = gh_within(root.path(), &path, &["api", "x"], Duration::ZERO).unwrap_err();
    assert_eq!(err.to_string(), "gh api x: timed out after 0ns");
    assert!(!ran.exists());
}

#[test]
fn failures_within_the_limit() {
    // req:vcs/bounded-runs@9e2c536e
    let dir = tempfile::tempdir().unwrap();
    let err = run_within(
        dir.path(),
        "git",
        &["status"],
        Options::default(),
        Duration::from_secs(5),
    )
    .unwrap_err();
    assert!(matches!(err, Error::Failed { .. }), "{err:?}");
    let shown = err.to_string();
    assert!(shown.contains("git status"), "{shown}");
    assert!(shown.contains("not a git repository"), "{shown}");
    assert_eq!(shown, git(dir.path(), &["status"]).unwrap_err().to_string());
}
