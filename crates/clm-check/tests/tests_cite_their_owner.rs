//! A test a component's `tests` own cites only that component (`tests-cite-their-owner`).

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// Written so the marker literal never appears whole in this file.
fn cite(id: &str) -> String {
    format!("// {}:{id}\n", concat!("req", ""))
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A component's frontmatter, with its `tests` when given, and one requirement `y`.
fn component(paths: &str, tests: Option<&str>) -> String {
    let tests = tests.map(|t| format!("tests: {t}\n")).unwrap_or_default();
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n{tests}---\n## Contract\n\n## Requirements\n\n### Requirement: y\nIt SHALL.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n"
    )
}

/// A git repository of components `a` (tests `tests/a/**`) and `b`, with `files` tracked.
fn repo(a_tests: Option<&str>, files: &[(&str, String)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &component("[]", None));
    write(
        root,
        "specs/a/SPEC.md",
        &component("[\"src/a/**\"]", a_tests),
    );
    write(root, "specs/b/SPEC.md", &component("[\"src/b/**\"]", None));
    for (path, text) in files {
        write(root, path, text);
    }
    for args in [&["init", "-q"][..], &["add", "."]] {
        let status = Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap();
        assert!(status.success());
    }
    dir
}

fn found(root: &Path) -> Vec<String> {
    clm_check::run(root)
        .iter()
        .filter(|v| v.rule == "check/tests-cite-their-owner")
        .map(clm_check::format)
        .collect()
}

#[test]
fn another_components_requirement() {
    // req:check/tests-cite-their-owner@27661ae3
    let text = format!("// x\n// y\n// z\n{}", cite("b/y"));
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", text)]);
    assert_eq!(
        found(dir.path()),
        [
            "tests/a/t.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s test; move the test to `b`'s tests"
        ]
    );
}

#[test]
fn its_own_requirement() {
    // req:check/tests-cite-their-owner@27661ae3
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", cite("a/y"))]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_file_only_paths_own() {
    // req:check/tests-cite-their-owner@27661ae3
    let dir = repo(None, &[("src/a/t.rs", cite("b/y"))]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_marker_naming_nothing_is_left_to_dangling_markers() {
    // req:check/tests-cite-their-owner@27661ae3
    let dir = repo(
        Some("[\"tests/a/**\"]"),
        &[("tests/a/t.rs", cite("b/nope"))],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_capability_is_the_capability_components() {
    // req:check/tests-cite-their-owner@27661ae3
    let dir = repo(
        Some("[\"tests/a/**\"]"),
        &[("tests/a/t.rs", cite("capability/c"))],
    );
    fs::write(
        dir.path().join("CAPABILITIES.md"),
        "# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can.\n\n#### Why\nBecause.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n",
    )
    .unwrap();
    write(
        dir.path(),
        "specs/capability/SPEC.md",
        &component("[\"tests/cap/**\"]", None),
    );
    assert_eq!(
        found(dir.path()),
        [
            "tests/a/t.rs:1: check/tests-cite-their-owner: cites capability/c of `capability`, but the file is `a`'s test; move the test to `capability`'s tests"
        ]
    );
}

#[test]
fn run_listed_reports_it_too() {
    // req:check/tests-cite-their-owner@27661ae3
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", cite("b/y"))]);
    let files = ["specs/SPEC.md", "tests/a/t.rs"].map(std::path::PathBuf::from);
    let found: Vec<_> = clm_check::run_listed(dir.path(), &files)
        .into_iter()
        .filter(|v| v.rule == "check/tests-cite-their-owner")
        .collect();
    assert_eq!(found.len(), 1);
}
