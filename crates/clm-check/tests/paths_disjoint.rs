//! A file belongs to one node (`paths-disjoint`).

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn node(paths: &str) -> String {
    format!("---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n---\n## Contract\n")
}

/// A git repository with the root node, `nodes` (`(spec path, paths)`) and tracked `files`.
fn repo(nodes: &[(&str, &str)], files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]"));
    for (spec, paths) in nodes {
        write(root, spec, &node(paths));
    }
    for f in files {
        write(root, f, "x\n");
    }
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    let status = Command::new("git")
        .args(["add", "."])
        .current_dir(root)
        .status()
        .unwrap();
    assert!(status.success());
    dir
}

fn found(root: &Path) -> Vec<String> {
    clm_check::run(root)
        .iter()
        .filter(|v| v.rule == "check/paths-disjoint")
        .map(clm_check::format)
        .collect()
}

#[test]
fn two_owners() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(
        &[
            ("specs/a/SPEC.md", "[\"src/**\"]"),
            ("specs/b/SPEC.md", "[\"src/b/**\"]"),
        ],
        &["src/b/x.rs"],
    );
    assert_eq!(
        found(dir.path()),
        ["src/b/x.rs: check/paths-disjoint: owned by `a`, `b`"]
    );
}

#[test]
fn parent_and_child() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(
        &[
            ("specs/a/SPEC.md", "[\"src/**\"]"),
            ("specs/a/c/SPEC.md", "[\"src/c/**\"]"),
        ],
        &["src/c/y.rs", "src/z.rs"],
    );
    assert_eq!(
        found(dir.path()),
        ["src/c/y.rs: check/paths-disjoint: owned by `a`, `a.c`"]
    );
}

#[test]
fn disjoint() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(
        &[
            ("specs/a/SPEC.md", "[\"src/a/**\"]"),
            ("specs/b/SPEC.md", "[\"src/b/**\"]"),
        ],
        &["src/a/x.rs", "src/b/y.rs"],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn markdown_and_untracked_files() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(
        &[
            ("specs/a/SPEC.md", "[\"src/**\"]"),
            ("specs/b/SPEC.md", "[\"src/b/**\"]"),
        ],
        &["src/b/README.md"],
    );
    write(dir.path(), "src/b/new.rs", "x\n");
    assert_eq!(
        found(dir.path()),
        ["src/b/README.md: check/paths-disjoint: owned by `a`, `b`"]
    );
}

#[test]
fn outside_a_work_tree_none() {
    // req:check/paths-disjoint@b3d269ba
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("[]"));
    write(dir.path(), "specs/a/SPEC.md", &node("[\"src/**\"]"));
    write(dir.path(), "specs/b/SPEC.md", &node("[\"src/**\"]"));
    write(dir.path(), "src/x.rs", "x\n");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

/// A component's frontmatter with `paths` and `tests`.
fn with_tests(paths: &str, tests: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ntests: {tests}\n---\n## Contract\n"
    )
}

#[test]
fn tests_inside_another_components_paths() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(
        &[("specs/harness/SPEC.md", "[\"tests/**\"]")],
        &["tests/a/t.py"],
    );
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &with_tests("[\"src/a/**\"]", "[\"tests/a/**\"]"),
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn two_components_tests() {
    // req:check/paths-disjoint@b3d269ba
    let dir = repo(&[], &["tests/b/t.py"]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &with_tests("[\"src/a/**\"]", "[\"tests/**\"]"),
    );
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &with_tests("[\"src/b/**\"]", "[\"tests/b/**\"]"),
    );
    assert_eq!(
        found(dir.path()),
        ["tests/b/t.py: check/paths-disjoint: owned by `a`, `b`"]
    );
}
