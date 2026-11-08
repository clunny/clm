//! `clm verify` as the binary dispatches it (`verify-command`).

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
}

/// A repository `clm init` has just set up.
fn initialised() -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    let init = clm(
        dir.path(),
        &["init", "--lang", "rust", "--no-detectors", "--host", "none"],
    );
    assert!(init.status.success(), "{init:?}");
    dir
}

const OFF: &str = "verify off: no [verify] in clm.toml\n";

// req:clm/verify-command@f591ec16
#[test]
fn verify_due_in_a_subdirectory_of_a_fresh_project_is_off() {
    let dir = initialised();
    let out = clm(&dir.path().join("specs"), &["verify", "--due"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), OFF);
}

// req:clm/verify-command@f591ec16
#[test]
fn every_flag_form_is_dispatched() {
    let dir = initialised();
    for args in [
        &["verify", "--due", "--event", "work-done", "--dry-run"][..],
        &["verify", "--commit", "abc"],
        &["verify", "--import"],
        &["verify", "--bisect", "abc"],
    ] {
        let out = clm(dir.path(), args);
        assert_eq!(out.status.code(), Some(0), "{args:?}: {out:?}");
        assert_eq!(String::from_utf8_lossy(&out.stdout), OFF, "{args:?}");
    }
}

// req:clm/verify-command@f591ec16
#[test]
fn a_mode_is_required_and_help_lists_the_command() {
    let dir = initialised();
    let none = clm(dir.path(), &["verify"]);
    assert_eq!(none.status.code(), Some(2), "{none:?}");
    let help = clm(dir.path(), &["--help"]);
    let text = String::from_utf8_lossy(&help.stdout);
    assert!(
        text.lines().any(|l| l.trim_start().starts_with("verify ")),
        "{text}"
    );
}
