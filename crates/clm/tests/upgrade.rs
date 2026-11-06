//! `clm upgrade` as the binary dispatches it (`upgrade-command`).

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

// req:clm/upgrade-command@86c8b58e
#[test]
fn upgrade_in_a_subdirectory_of_a_fresh_project_is_up_to_date() {
    let dir = initialised();
    let sub = dir.path().join("specs");
    let out = clm(&sub, &["upgrade"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "up to date\n");
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn the_flags_are_dispatched_and_exclusive() {
    let dir = initialised();
    let before = fs::read_to_string(dir.path().join("clm.toml")).unwrap();
    let apply = clm(dir.path(), &["upgrade", "--apply"]);
    assert_eq!(apply.status.code(), Some(0), "{apply:?}");
    assert!(
        apply.stdout.is_empty(),
        "apply is not the listing: {apply:?}"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("clm.toml")).unwrap(),
        before,
        "nothing to change writes nothing"
    );
    let unknown = clm(dir.path(), &["upgrade", "--note", "no-such-migration"]);
    assert_eq!(unknown.status.code(), Some(1), "{unknown:?}");
    let decline = clm(dir.path(), &["upgrade", "--decline", "no-such-migration"]);
    assert_eq!(decline.status.code(), Some(1), "{decline:?}");
    let both = clm(dir.path(), &["upgrade", "--apply", "--note", "x"]);
    assert_eq!(both.status.code(), Some(2), "{both:?}");
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn lock_runs_on_the_upgrade_crate_and_refuses_a_too_new_project() {
    let dir = initialised();
    // The binary holds no migration to lock, so the lock is left unwritten.
    let out = clm(dir.path(), &["upgrade", "--lock", "0.1.0"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        !dir.path()
            .join("crates/clm-upgrade/migrations.lock")
            .exists()
    );

    let toml = dir.path().join("clm.toml");
    let text = fs::read_to_string(&toml).unwrap();
    fs::write(&toml, format!("upgraded = \"99990101-future\"\n{text}")).unwrap();
    let too_new = clm(dir.path(), &["upgrade", "--lock", "0.1.0"]);
    assert_eq!(too_new.status.code(), Some(1), "{too_new:?}");
    assert!(String::from_utf8_lossy(&too_new.stderr).contains("newer `clm`"));
}

// req:clm/upgrade-command@86c8b58e
#[test]
fn upgrade_from_a_subdirectory_reads_the_project_root() {
    let dir = initialised();
    let toml = dir.path().join("clm.toml");
    let text = fs::read_to_string(&toml).unwrap();
    fs::write(&toml, format!("upgraded = \"99990101-future\"\n{text}")).unwrap();
    let out = clm(&dir.path().join("specs"), &["upgrade"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("newer `clm`"));
}
