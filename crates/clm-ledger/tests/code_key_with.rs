mod common;

use std::path::Path;

use clm_ledger::{code_key, code_key_with};
use common::{commit, repo, sh};

/// A repository whose first commit holds `clm.toml` among neighbours that sort before and
/// after it and a file under a directory, and that commit.
fn setup() -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().unwrap();
    repo(dir.path(), "T");
    let a = commit(
        dir.path(),
        &[
            ("aaa.toml", "a\n"),
            ("clm.toml", "x = 1\n"),
            ("crates/a/src/lib.rs", "fn a() {}\n"),
            ("zzz.toml", "z\n"),
        ],
        "a",
    );
    (dir, a)
}

/// A commit on `rev` without `paths`.
fn without(root: &Path, rev: &str, paths: &[&str]) -> String {
    sh(root, &["checkout", "-q", "--detach", rev]);
    for path in paths {
        sh(root, &["rm", "-q", path]);
    }
    sh(root, &["commit", "-q", "--allow-empty", "-m", "without"]);
    sh(root, &["rev-parse", "HEAD"])
}

#[test]
fn one_file_taken_from_the_base() {
    // req:ledger/code-key-with@52cbbafd
    let (dir, a) = setup();
    let root = dir.path();
    let b = commit(root, &[("clm.toml", "x = 2\n"), ("docs/x.md", "x\n")], "b");
    let with = code_key_with(root, &b, "clm.toml", &a).unwrap();
    assert_eq!(with, code_key(root, &a).unwrap());
    assert_ne!(with, code_key(root, &b).unwrap());
    // A doc-only path is no part of the key: `rev`'s own comes back.
    assert_eq!(
        code_key_with(root, &b, "docs/x.md", &a).unwrap(),
        code_key(root, &b).unwrap()
    );
}

#[test]
fn absent_in_from() {
    // req:ledger/code-key-with@52cbbafd
    let (dir, a) = setup();
    let root = dir.path();
    let b = commit(root, &[("tool.toml", "t\n")], "b");
    assert_eq!(
        code_key_with(root, &b, "tool.toml", &a).unwrap(),
        code_key(root, &a).unwrap()
    );
}

#[test]
fn absent_in_rev() {
    // req:ledger/code-key-with@52cbbafd
    let (dir, a) = setup();
    let root = dir.path();
    // Restoring an entry at the start, the middle and the end of the listing.
    for path in ["aaa.toml", "clm.toml", "crates/a/src/lib.rs", "zzz.toml"] {
        let b = without(root, &a, &[path]);
        assert_ne!(code_key(root, &b).unwrap(), code_key(root, &a).unwrap());
        assert_eq!(
            code_key_with(root, &b, path, &a).unwrap(),
            code_key(root, &a).unwrap(),
            "{path}"
        );
    }
}

#[test]
fn another_file_differs_too() {
    // req:ledger/code-key-with@52cbbafd
    let (dir, a) = setup();
    let root = dir.path();
    let b = commit(
        root,
        &[
            ("clm.toml", "x = 2\n"),
            ("crates/a/src/lib.rs", "fn b() {}\n"),
        ],
        "b",
    );
    assert_ne!(
        code_key_with(root, &b, "clm.toml", &a).unwrap(),
        code_key(root, &a).unwrap()
    );
}

#[test]
fn mode_taken_with_the_entry() {
    // req:ledger/code-key-with@52cbbafd
    use std::os::unix::fs::PermissionsExt;
    let (dir, a) = setup();
    let root = dir.path();
    let mode = std::fs::Permissions::from_mode(0o755);
    std::fs::set_permissions(root.join("clm.toml"), mode).unwrap();
    let b = commit(root, &[], "mode");
    assert_ne!(code_key(root, &b).unwrap(), code_key(root, &a).unwrap());
    assert_eq!(
        code_key_with(root, &b, "clm.toml", &a).unwrap(),
        code_key(root, &a).unwrap()
    );
    assert_eq!(
        code_key_with(root, &a, "clm.toml", &b).unwrap(),
        code_key(root, &b).unwrap()
    );
}

#[test]
fn path_matched_exactly() {
    // req:ledger/code-key-with@52cbbafd
    let (dir, a) = setup();
    let root = dir.path();
    let b = commit(root, &[("crates/a/src/lib.rs", "fn b() {}\n")], "b");
    // A directory, a prefix, a glob or pathspec magic names no one file: `rev`'s key stays.
    for path in [
        "crates",
        "crates/a",
        "crates/a/src/lib",
        "crates/*",
        ":(glob)crates/**",
    ] {
        assert_eq!(
            code_key_with(root, &b, path, &a).unwrap(),
            code_key(root, &b).unwrap(),
            "{path}"
        );
    }
}
