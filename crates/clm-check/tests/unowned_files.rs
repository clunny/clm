//! Every code file is in some node's `paths` or the root's `outside` (`unowned-files`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

const MESSAGE: &str = "in no component's paths; add it to a component's paths, or to outside in specs/SPEC.md until a change adopts it";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn node(paths: &str, outside: Option<&str>) -> String {
    let outside = outside
        .map(|o| format!("outside: {o}\n"))
        .unwrap_or_default();
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n{outside}---\n## Contract\n"
    )
}

/// A directory with the root node (setting `outside` when given), `nodes`
/// (`(spec path, paths)`) and `files`.
fn tree(outside: Option<&str>, nodes: &[(&str, &str)], files: &[&str]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &node("[]", outside));
    for (spec, paths) in nodes {
        write(root, spec, &node(paths, None));
    }
    for f in files {
        write(root, f, "x\n");
    }
    dir
}

/// The same as a git repository with everything in the index.
fn repo(outside: Option<&str>, nodes: &[(&str, &str)], files: &[&str]) -> TempDir {
    let dir = tree(outside, nodes, files);
    for args in [&["init", "-q"][..], &["add", "."]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert!(status.success());
    }
    dir
}

fn found(root: &Path) -> Vec<String> {
    clm_check::run(root)
        .iter()
        .filter(|v| v.rule == "check/unowned-files")
        .map(clm_check::format)
        .collect()
}

const A: (&str, &str) = ("specs/a/SPEC.md", "[\"crates/a/**\"]");

#[test]
fn file_in_no_node() {
    // req:check/unowned-files@788ce525
    let dir = repo(Some("[]"), &[A], &["crates/a/lib.rs", "crates/b/lib.rs"]);
    assert_eq!(
        found(dir.path()),
        [format!("crates/b/lib.rs: check/unowned-files: {MESSAGE}")]
    );
}

#[test]
fn not_opted_in() {
    // req:check/unowned-files@788ce525
    let dir = repo(None, &[A], &["crates/a/lib.rs", "crates/b/lib.rs"]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn declared_not_yet_adopted() {
    // req:check/unowned-files@788ce525
    let dir = repo(
        Some("[\"crates/b/**\"]"),
        &[A],
        &["crates/a/lib.rs", "crates/b/lib.rs"],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn part_of_an_outside_directory_adopted() {
    // req:check/unowned-files@788ce525
    let dir = repo(
        Some("[\"src/**\"]"),
        &[("specs/api/SPEC.md", "[\"src/api/**\"]")],
        &["src/api/x.rs", "src/web/y.rs"],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn outside_glob_left_behind() {
    // req:check/unowned-files@788ce525
    let dir = repo(
        Some("[\"crates/b/**\"]"),
        &[A, ("specs/b/SPEC.md", "[\"crates/b/**\"]")],
        &["crates/a/lib.rs", "crates/b/lib.rs"],
    );
    assert_eq!(
        found(dir.path()),
        [
            "specs/SPEC.md: check/unowned-files: outside glob crates/b/** matches no file outside every component"
        ]
    );
}

#[test]
fn spec_paths_and_untracked_files() {
    // req:check/unowned-files@788ce525
    let dir = repo(
        Some("[]"),
        &[A],
        &[
            "docs/guide/x.md",
            "backlog/y.md",
            "changes/c/change.md.txt",
            "ARCHITECTURE.md",
            "CAPABILITIES.md",
            "CLAUDE.md",
            "AGENTS.md",
            "crates/a/lib.rs",
        ],
    );
    write(dir.path(), "notes/todo.txt", "x\n");
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn whatever_the_node_status() {
    // req:check/unowned-files@788ce525
    let dir = repo(Some("[]"), &[], &["crates/a/lib.rs"]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        "---\ntitle: N\nstatus: specd\nowners: []\npaths: [\"crates/a/**\"]\n---\n## Contract\n",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[cfg(unix)]
#[test]
fn symlinks_and_markdown_count() {
    // req:check/unowned-files@788ce525
    let dir = tree(Some("[]"), &[A], &["README.md"]);
    std::os::unix::fs::symlink("README.md", dir.path().join("link")).unwrap();
    for args in [&["init", "-q"][..], &["add", "."]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(dir.path())
            .status()
            .unwrap();
        assert!(status.success());
    }
    assert_eq!(
        found(dir.path()),
        [
            format!("README.md: check/unowned-files: {MESSAGE}"),
            format!("link: check/unowned-files: {MESSAGE}"),
        ]
    );
}

#[test]
fn extracted_tree() {
    // req:check/unowned-files@788ce525
    let dir = tree(
        Some("[\".ci/**\"]"),
        &[A],
        &["crates/b/lib.rs", ".ci/build.yml"],
    );
    let files = ["specs/SPEC.md", "crates/b/lib.rs", ".ci/build.yml"].map(PathBuf::from);
    let found: Vec<String> = clm_check::run_listed(dir.path(), &files)
        .iter()
        .filter(|v| v.rule == "check/unowned-files")
        .map(clm_check::format)
        .collect();
    assert_eq!(
        found,
        [format!("crates/b/lib.rs: check/unowned-files: {MESSAGE}")]
    );
}

#[test]
fn no_work_tree_no_listing() {
    // req:check/unowned-files@788ce525
    let dir = tree(
        Some("[\".ci/**\"]"),
        &[A],
        &["crates/b/lib.rs", ".ci/build.yml"],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn submodules_count() {
    // req:check/unowned-files@788ce525
    let dir = repo(Some("[]"), &[A], &["crates/a/lib.rs"]);
    let status = Command::new("git")
        .args([
            "update-index",
            "--add",
            "--cacheinfo",
            "160000,1111111111111111111111111111111111111111,vendor/sub",
        ])
        .current_dir(dir.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        found(dir.path()),
        [format!("vendor/sub: check/unowned-files: {MESSAGE}")]
    );
}

#[test]
fn owned_by_tests() {
    // req:check/unowned-files@788ce525
    let dir = repo(Some("[]"), &[], &["crates/a/lib.rs", "tests/a/t.rs"]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\ntests: [\"tests/a/**\"]\n---\n## Contract\n",
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}
