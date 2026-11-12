//! `clm machine` as the binary dispatches it (`machine-command`).

use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("CLM_MAX_BUILDS")
        .output()
        .unwrap()
}

/// A repository `clm init` has just set up.
fn initialised() -> TempDir {
    let dir = TempDir::new().unwrap();
    let git = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(git.status.success(), "{git:?}");
    let init = clm(
        dir.path(),
        &["init", "--lang", "rust", "--no-detectors", "--host", "none"],
    );
    assert!(init.status.success(), "{init:?}");
    dir
}

// req:clm/machine-command@9c8df2a2
#[test]
fn dispatch_passes_max_builds_from_a_subdirectory() {
    let dir = initialised();
    std::fs::write(dir.path().join("clm.local.toml"), "max_builds = 2\n").unwrap();
    let sub = dir.path().join("specs");
    let out = clm(&sub, &["machine", "--format", "json"]);
    assert!(out.status.success(), "{out:?}");
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["disk"]["free_places"], 2, "{v}");
}

// req:clm/machine-command@9c8df2a2
#[test]
fn help_lists_machine() {
    let dir = initialised();
    let out = clm(dir.path(), &["--help"]);
    assert!(out.status.success(), "{out:?}");
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(help.lines().any(|l| l.trim_start().starts_with("machine ")));
}

// req:clm/machine-command@9c8df2a2
#[test]
fn text_format_prints_the_three_readings() {
    let dir = initialised();
    let out = clm(dir.path(), &["machine", "--format", "text"]);
    assert!(out.status.success(), "{out:?}");
    let text = String::from_utf8_lossy(&out.stdout);
    for word in ["cpu", "mem", "disk"] {
        assert!(text.contains(word), "{text}");
    }
}
