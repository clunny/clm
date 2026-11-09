//! The starter `scripts/clm-check.sh` for Rust keeps a land's cargo build in the place `clm land`
//! hands it (`starter-check-land-dir`).

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;
use std::sync::Mutex;

use clm_init::{Options, init};
use tempfile::TempDir;

/// Writing a stub and running it must not overlap another test's fork (`ETXTBSY`).
static FORKS: Mutex<()> = Mutex::new(());

/// A Rust repository `clm init --lang rust` set up, holding a `Cargo.toml`.
fn rust_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let out = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let opts = Options {
        langs: vec!["rust".into()],
        ..Options::default()
    };
    init(dir.path(), &opts).unwrap();
    fs::write(dir.path().join("Cargo.toml"), "").unwrap();
    dir
}

/// Runs the repository's check script with a `cargo` that records the two variables, with
/// `CLM_LAND_DIR` and `CARGO_TARGET_DIR` as given and `CARGO_INCREMENTAL` unset. Returns the
/// stub's record.
fn run_script(dir: &Path, land_dir: Option<&str>, target_dir: Option<&str>) -> String {
    let _forks = FORKS.lock().unwrap_or_else(|e| e.into_inner());
    let stubs = TempDir::new().unwrap();
    let log = stubs.path().join("log");
    let cargo = stubs.path().join("cargo");
    fs::write(
        &cargo,
        format!(
            "#!/bin/sh\necho \"target=${{CARGO_TARGET_DIR-unset}} incremental=${{CARGO_INCREMENTAL-unset}}\" >> '{}'\n",
            log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&cargo, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!(
        "{}:{}",
        stubs.path().display(),
        std::env::var("PATH").unwrap()
    );
    let mut cmd = Command::new("bash");
    cmd.arg(dir.join("scripts/clm-check.sh"))
        .current_dir(dir)
        .env("PATH", path)
        .env_remove("CLM_LAND_DIR")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_INCREMENTAL");
    if let Some(land) = land_dir {
        cmd.env("CLM_LAND_DIR", land);
    }
    if let Some(target) = target_dir {
        cmd.env("CARGO_TARGET_DIR", target);
    }
    let out = cmd.output().unwrap();
    assert!(out.status.success(), "{out:?}");
    fs::read_to_string(log).unwrap()
}

#[test]
fn a_land_hands_the_place() {
    // req:init/starter-check-land-dir@3e5ee659
    let dir = rust_repo();
    let seen = run_script(dir.path(), Some("/tmp/x/land"), None);
    assert_eq!(seen, "target=/tmp/x/land incremental=0\n");
}

#[test]
fn no_place_handed_leaves_the_callers_variables() {
    // req:init/starter-check-land-dir@3e5ee659
    let dir = rust_repo();
    let seen = run_script(dir.path(), None, Some("/tmp/y"));
    assert_eq!(seen, "target=/tmp/y incremental=unset\n");
    // An empty place counts as none handed.
    let seen = run_script(dir.path(), Some(""), Some("/tmp/y"));
    assert_eq!(seen, "target=/tmp/y incremental=unset\n");
}

#[test]
fn a_repository_without_cargo_toml_still_passes() {
    // req:init/starter-check-land-dir@3e5ee659
    let dir = rust_repo();
    fs::remove_file(dir.path().join("Cargo.toml")).unwrap();
    let _forks = FORKS.lock().unwrap_or_else(|e| e.into_inner());
    let out = Command::new("bash")
        .arg(dir.path().join("scripts/clm-check.sh"))
        .current_dir(dir.path())
        .env("CLM_LAND_DIR", "/tmp/x/land")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
}
