//! `clm unreferenced`: public items of a workspace's libraries that nothing else uses, as a
//! command and as a `clm check` detector.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

/// A git repository holding `files`, all added to the index.
fn repo(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    git(dir.path(), &["init", "-q", "-b", "main"]);
    for (rel, content) in files {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }
    git(dir.path(), &["add", "-A"]);
    dir
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap()
}

const A_MANIFEST: &str = "[package]\nname = \"a\"\nversion = \"0.1.0\"\n";
const TAIL: &str = "is public but nothing outside its library uses it";

#[test]
fn scenario_from_a_subdirectory_of_a_project_without_clm() {
    // req:clm/root-from-cwd@7274c252
    let dir = repo(&[
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(&dir.path().join("crates/a/src"), &["unreferenced"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("crates/a/src/lib.rs:1: a::f {TAIL}\n")
    );
}

#[test]
fn scenario_named_as_a_detector() {
    // req:clm/check-across-components@45a113b3
    let clm_toml = format!(
        "[[detectors]]\nname = \"unreferenced-public-items\"\ncommand = [{:?}, \"unreferenced\"]\n",
        env!("CARGO_BIN_EXE_clm")
    );
    let dir = repo(&[
        (
            "specs/SPEC.md",
            "---\ntitle: R\nstatus: active\nowners: []\n---\n",
        ),
        ("clm.toml", &clm_toml),
        ("crates/a/Cargo.toml", A_MANIFEST),
        ("crates/a/src/lib.rs", "pub fn f() {}\n"),
    ]);
    let out = clm(dir.path(), &["check"]);
    let stdout = String::from_utf8(out.stdout).unwrap();
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert_eq!(out.status.code(), Some(1), "{stdout}{stderr}");
    assert!(
        stdout
            .lines()
            .any(|l| l == "clm.toml: check/detectors: unreferenced-public-items failed (exit 1)"),
        "{stdout}"
    );
    assert!(
        stderr.contains(&format!("crates/a/src/lib.rs:1: a::f {TAIL}")),
        "{stderr}"
    );
}
