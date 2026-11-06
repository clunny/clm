//! `clm init` stamps a `clm.toml` it writes with `upgraded`, and never one it keeps.

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_init::{Options, init};
use tempfile::TempDir;

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let out = Command::new("git")
        .args(["init", "-q", "-b", "main"])
        .current_dir(dir.path())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    dir
}

/// The value of the top-level `upgraded` key of `dir`'s `clm.toml`, when set.
fn upgraded(dir: &Path) -> Option<String> {
    let text = fs::read_to_string(dir.join("clm.toml")).unwrap();
    let table: toml::Table = text.parse().unwrap();
    table
        .get("upgraded")
        .map(|v| v.as_str().expect("upgraded is a string").to_owned())
}

#[test]
fn new_repository_is_stamped_with_the_newest_migration() {
    // req:init/init-stamps-upgraded@37a0167c
    let dir = repo();
    init(dir.path(), &Options::default()).unwrap();
    assert_eq!(
        upgraded(dir.path()).as_deref(),
        clm_upgrade::latest(),
        "upgraded must be upgrade's latest()"
    );
}

#[test]
fn kept_clm_toml_is_not_stamped() {
    // req:init/init-stamps-upgraded@37a0167c
    let dir = repo();
    fs::write(dir.path().join("clm.toml"), "approval = \"any\"\n").unwrap();
    init(dir.path(), &Options::default()).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join("clm.toml")).unwrap(),
        "approval = \"any\"\n"
    );
}
