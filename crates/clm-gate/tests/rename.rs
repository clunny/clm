use std::fs;
use std::path::Path;

use clm_gate::{Kind, evaluate_at};
use clm_rename::rename;
use tempfile::TempDir;

fn git(d: &Path, args: &[&str]) -> String {
    clm_git::git(d, args).unwrap()
}

fn write(d: &Path, rel: &str, content: &str) {
    let path = d.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn commit(d: &Path, message: &str) {
    git(d, &["add", "-A"]);
    git(
        d,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            message,
        ],
    );
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: specd\nowners: [\"@a\"]\n---\n\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";
const ARCH: &str = "# Arch\n\n## Requirements\n\n### Requirement: law\nA law SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Enforced by\n- a/x\n";

/// A repository on `main` with node `a`, then a branch `pr` holding the committed result of
/// `clm rename r` for a change whose `renames` hold `renames` and whose other frontmatter
/// is `extra`.
fn renamed(renames: &str, extra: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "ARCHITECTURE.md", ARCH);
    write(d, "crates/a/src/lib.rs", "pub fn a() {}\n");
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    let change =
        format!("---\nstatus: proposed\n{extra}renames:\n{renames}---\n\n## Intent\nRename.\n");
    write(d, "changes/r/change.md", &change);
    git(d, &["add", "-A"]);
    rename(d, "r").unwrap();
    commit(d, "rename");
    dir
}

const ID_RENAME: &str = "  - requirement: a/x\n    to: y\n";
const TERM_RENAME: &str = "  - term: spine\n    to: trunk\n";

#[test]
fn a_rename_branch_is_a_spec_branch() {
    // req:gate/classify@17132c50
    let dir = renamed(ID_RENAME, "");
    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Spec, "{:?}", out.violations);
    assert!(!out.review);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn a_rename_branch_with_a_code_edit_is_mixed() {
    // req:gate/classify@17132c50
    let dir = renamed(ID_RENAME, "");
    write(dir.path(), "crates/a/src/lib.rs", "pub fn b() {}\n");
    commit(dir.path(), "extra");
    let out = evaluate_at(dir.path(), "main", "HEAD", true).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
    assert!(out.violations.iter().any(|v| v.rule == "gate/classify"));
}

#[test]
fn an_id_rename_through_the_human_layers_needs_no_review() {
    // req:gate/needs-review@3276bfca
    let dir = renamed(ID_RENAME, "");
    let arch = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    assert!(arch.contains("- a/y"), "the rename rewrote ARCHITECTURE.md");
    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Spec);
    assert!(!out.review);
    assert!(out.violations.is_empty(), "{:?}", out.violations);
}

#[test]
fn a_term_rename_needs_review() {
    // req:gate/needs-review@3276bfca
    let dir = renamed(TERM_RENAME, "");
    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Spec, "{:?}", out.violations);
    assert!(out.review);
    assert!(out.violations.iter().any(|v| v.rule == "gate/approval"));
}

#[test]
fn a_rename_that_declares_burdens_needs_review() {
    // req:gate/needs-review@3276bfca
    let dir = renamed(ID_RENAME, "human_burdens: [a paid run]\n");
    let out = evaluate_at(dir.path(), "main", "HEAD", false).unwrap();
    assert_eq!(out.kind, Kind::Spec, "{:?}", out.violations);
    assert!(out.review);
}
