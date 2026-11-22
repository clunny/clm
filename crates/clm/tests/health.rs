//! `clm health` as the `clm` binary routes it, with the settings it loads.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

#[path = "../../clm-ledger/tests/common/mod.rs"]
mod ledger_common;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn land_target_reaches_health() {
    // req:clm/health-land-target@a8db0736
    land_time_line(
        "host = \"git\"\n[land]\ncheck = \"true\"\ncheck_target_seconds = 45\n",
        "health/land-time: 1 of 1 timed lands over 45s",
    );
}

#[test]
fn land_target_defaults_when_unset() {
    // req:clm/health-land-target@a8db0736
    land_time_line(
        "host = \"git\"\n[land]\ncheck = \"true\"\n",
        "health/land-time: 0 of 1 timed lands over 120s",
    );
    land_time_line(
        "host = \"git\"\n",
        "health/land-time: 0 of 1 timed lands over 120s",
    );
}

/// Runs `clm health` in a clone holding `clm.toml` and one timed land of 50 seconds, and
/// asserts that its output has the `expected` line.
fn land_time_line(clm_toml: &str, expected: &str) {
    use clm_ledger::{Body, LandKind};
    let dir = TempDir::new().unwrap();
    let root = ledger_common::clone_with_origin(dir.path(), "repo", "Ann");
    write(
        &root,
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: active\nowners: []\n---\n",
    );
    write(&root, "clm.toml", clm_toml);
    let body = Body::landed(
        "c1".into(),
        "impl/a".into(),
        LandKind::Code,
        false,
        "f".into(),
    )
    .with_check_seconds(Some(50));
    clm_ledger::append(&root, body, &ledger_common::env(dir.path(), &[])).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["health", "--root"])
        .arg(&root)
        .env("HOME", dir.path())
        .env_remove("XDG_CONFIG_HOME")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(stdout.lines().any(|l| l == expected), "{stdout}");
}
