//! `overlap`: what two selections can both break, and `affected_between`, the selection
//! between two commits it is applied to.

use std::fs;
use std::path::Path;
use std::process::Command;

use clm_affected::{Selection, affected_between, overlap};
use tempfile::TempDir;

fn nodes(nodes: &[&str], files: &[&str]) -> Selection {
    Selection::Nodes {
        nodes: nodes.iter().map(|s| s.to_string()).collect(),
        files: files.iter().map(|s| s.to_string()).collect(),
    }
}

fn all(path: &str) -> Selection {
    Selection::All(path.to_string())
}

#[test]
fn both_everything_gives_the_first() {
    // req:check.affected/overlap-selection@8fd7fe1a
    assert_eq!(overlap(all("a"), all("b")), all("a"));
}

#[test]
fn one_everything_gives_the_other_unchanged() {
    // req:check.affected/overlap-selection@8fd7fe1a
    let other = nodes(&["c"], &["f"]);
    assert_eq!(overlap(all("a"), other.clone()), other);
    assert_eq!(overlap(other.clone(), all("a")), other);
}

#[test]
fn otherwise_the_components_and_files_both_hold() {
    // req:check.affected/overlap-selection@8fd7fe1a
    let a = nodes(&["a", "c", "d"], &["f1", "f2", "f4"]);
    let b = nodes(&["b", "c", "d"], &["f2", "f3", "f4"]);
    assert_eq!(overlap(a, b), nodes(&["c", "d"], &["f2", "f4"]));
    assert_eq!(
        overlap(nodes(&["a"], &["x"]), nodes(&["b"], &[])),
        nodes(&[], &[])
    );
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", message]);
}

fn spec(paths: &str, depends_on: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
    )
}

#[test]
fn affected_between_gives_the_selection_between_two_commits() {
    // req:check.affected/overlap-selection@8fd7fe1a
    // `a`, `b` on `a`, and `c` on nothing; `main`, then `x` changing `a`'s file and `y`
    // changing `b`'s, each from `main`.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "clm.toml", "");
    write(root, "specs/SPEC.md", &spec("[\"clm.toml\"]", "[]"));
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
    write(root, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", "[a]"));
    write(root, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", "[]"));
    for id in ["a", "b", "c"] {
        write(root, &format!("crates/{id}/src/lib.rs"), "fn f() {}\n");
    }
    commit_all(root, "start");
    git(root, &["switch", "-q", "-c", "x"]);
    write(root, "crates/a/src/lib.rs", "fn changed() {}\n");
    commit_all(root, "x");
    git(root, &["switch", "-q", "-c", "y", "main"]);
    write(root, "crates/b/src/lib.rs", "fn changed() {}\n");
    commit_all(root, "y");
    // The working tree, `y` checked out with `c` changed and a file untracked, is not read.
    write(root, "crates/c/src/lib.rs", "fn uncommitted() {}\n");
    write(root, "crates/c/src/new.rs", "fn untracked() {}\n");

    let x = affected_between(root, "main", "x", &[]).unwrap();
    // `a` changed, and `b` depends on it; each owns its file.
    assert_eq!(
        x,
        nodes(&["a", "b"], &["crates/a/src/lib.rs", "crates/b/src/lib.rs"])
    );
    let y = affected_between(root, "main", "y", &[]).unwrap();
    assert_eq!(y, nodes(&["b"], &["crates/b/src/lib.rs"]));
    // What both can break.
    assert_eq!(overlap(x, y), nodes(&["b"], &["crates/b/src/lib.rs"]));
    // The `[affected] all` globs given select everything for a change they match.
    assert_eq!(
        affected_between(root, "main", "x", &["crates/a/**".to_string()]).unwrap(),
        all("crates/a/src/lib.rs")
    );
}
