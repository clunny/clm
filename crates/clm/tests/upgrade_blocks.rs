//! `upgrade-blocks-commands` as the binary runs it.

use std::fs;
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

fn project(config: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir(dir.path().join(".git")).unwrap();
    fs::create_dir(dir.path().join("specs")).unwrap();
    fs::write(dir.path().join("specs/SPEC.md"), "").unwrap();
    fs::write(dir.path().join("clm.toml"), config).unwrap();
    dir
}

// req:clm/upgrade-blocks-commands@8787b1c6
#[test]
fn a_project_upgraded_by_a_newer_clm_stops_land_but_not_status() {
    let dir = project("upgraded = \"99990101-from-the-future\"\n");
    let land = clm(dir.path(), &["land", "--check-only"]);
    assert_eq!(land.status.code(), Some(4));
    let err = String::from_utf8_lossy(&land.stderr);
    assert!(err.contains("newer `clm`"), "{err}");
    assert!(land.stdout.is_empty());
    let status = clm(dir.path(), &["status"]);
    assert_ne!(status.status.code(), Some(4));
}

// req:clm/upgrade-blocks-commands@8787b1c6
#[test]
fn a_project_with_nothing_pending_runs_commands_and_a_non_project_is_not_judged() {
    let ok = project("defects = \"ask\"\n");
    assert_ne!(clm(ok.path(), &["check"]).status.code(), Some(4));
    let none = TempDir::new().unwrap();
    fs::create_dir(none.path().join(".git")).unwrap();
    fs::write(none.path().join("clm.toml"), "upgraded = \"zzz\"\n").unwrap();
    assert_ne!(clm(none.path(), &["check"]).status.code(), Some(4));
}
