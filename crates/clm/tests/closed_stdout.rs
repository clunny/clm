//! A reader of stdout that has gone ends the command quietly (clm/closed-stdout).

use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

fn spec(title: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: []\ndepends_on: []\n---\n\n## Contract\nC\n\n## Design\nD\n\n## Requirements\n"
    )
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("ARCHITECTURE.md"), "ARCH\n").unwrap();
    fs::create_dir_all(dir.path().join("specs/a")).unwrap();
    fs::write(dir.path().join("specs/SPEC.md"), spec("Root")).unwrap();
    fs::write(dir.path().join("specs/a/SPEC.md"), spec("A")).unwrap();
    dir
}

/// Runs `clm <args>` in `root` with a stdout whose reading end is already closed.
fn with_closed_reader(root: &Path, args: &[&str]) -> Output {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(root)
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

#[test]
fn a_print_to_a_closed_pipe_exits_141_quietly() {
    // req:clm/closed-stdout@e1df9220
    let dir = repo();
    let out = with_closed_reader(dir.path(), &["context", "a"]);
    assert_eq!(out.status.code(), Some(141));
    assert_eq!(String::from_utf8_lossy(&out.stderr), "");
}

#[test]
fn a_view_to_a_closed_pipe_prints_no_panic() {
    // req:clm/closed-stdout@e1df9220
    let dir = repo();
    let out = with_closed_reader(dir.path(), &["view", "spec"]);
    assert_ne!(out.status.code(), Some(0));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"));
}

#[cfg(target_os = "linux")]
#[test]
fn a_full_disk_still_panics_with_its_reason() {
    // req:clm/closed-stdout@e1df9220
    let dir = repo();
    let full = fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["context", "a"])
        .current_dir(dir.path())
        .stdout(full)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let err = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(101), "{err}");
    assert!(err.contains("failed printing to stdout"), "{err}");
    assert!(err.contains("No space left on device"), "{err}");
}
