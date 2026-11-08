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

/// A set-up repository with `[verify]` on, so each flag reaches its own mode.
fn verifying() -> TempDir {
    let dir = initialised();
    let toml = dir.path().join("clm.toml");
    let mut text = std::fs::read_to_string(&toml).unwrap();
    text.push_str("\n[verify]\ncheck = \"true\"\nmode = \"local\"\ntriggers = [\"work-done\"]\n");
    std::fs::write(&toml, text).unwrap();
    git(dir.path(), &["add", "-A"]);
    git(
        dir.path(),
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@e",
            "commit",
            "-qm",
            "i",
        ],
    );
    dir
}

fn verify_as_writer(dir: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("CLM_WRITER", "t")
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

// req:clm/verify-command@f591ec16
#[test]
fn each_flag_reaches_its_own_mode() {
    let dir = verifying();
    let run = |args: &[&str]| verify_as_writer(dir.path(), args);
    let (code, out, _) = run(&["verify", "--due"]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "verify not-due no trigger holds for 1 lands\n")
    );
    let (code, out, _) = run(&["verify", "--due", "--event", "work-done", "--dry-run"]);
    assert!(out.starts_with("verify due"), "{out}");
    assert_eq!(code, Some(0));
    let (code, out, _) = run(&["verify", "--due", "--dry-run"]);
    assert_eq!((code, out.starts_with("verify not-due")), (Some(3), true));
    let (code, out, _) = run(&["verify", "--import"]);
    assert_eq!(
        (code, out.as_str()),
        (Some(0), "verify nothing to import\n")
    );
    let (code, out, _) = run(&["verify", "--commit", "HEAD"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.starts_with("verify passed "), "{out}");
    let (code, _, err) = run(&["verify", "--bisect", "HEAD"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("nothing to bisect"), "{err}");
}

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
