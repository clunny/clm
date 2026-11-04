use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn clm(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn repo(change: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "notes.txt", "see a/x\n");
    write(d, "changes/r/change.md", change);
    let git = |args: &[&str]| {
        let ok = Command::new("git").current_dir(d).args(args).status();
        assert!(ok.unwrap().success());
    };
    git(&["init", "-q"]);
    git(&["add", "-A"]);
    dir
}

#[test]
// req:rename/finish@90976f4b
fn the_command_prints_what_it_did_and_exits_zero() {
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/x\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = clm(dir.path(), &["rename", "r"]);
    assert_eq!(out.status.code(), Some(0));
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        "renamed a/x -> a/y\nstill names a/x notes.txt:1\n"
    );
    assert!(dir.path().join("changes/archive/r/change.md").exists());
}

#[test]
fn a_refusal_exits_one_and_writes_nothing() {
    let dir = repo(
        "---\nstatus: proposed\nrenames:\n  - requirement: a/none\n    to: y\n---\n\n## Intent\nr\n",
    );
    let out = clm(dir.path(), &["rename", "r"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr).unwrap().contains("a/none"));
    assert!(dir.path().join("changes/r/change.md").exists());
    assert!(!dir.path().join("changes/archive").exists());
}
