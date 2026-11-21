//! A test a component's `tests` own, or any file a component owns once the root names its
//! `wiring`, cites only its owner (`tests-cite-their-owner`).

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// A test citing `id`, written so the marker literal never appears whole in this file.
fn cite(id: &str) -> String {
    format!("// {}:{id}\n#[test]\nfn t() {{}}\n", concat!("req", ""))
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
    // req:check/tests-cite-their-owner@edb5a1b0
    let text = format!("// x\n// y\n// z\n{}", cite("b/y"));
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", text)]);
    assert_eq!(
        found(dir.path()),
        [
            "tests/a/t.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files"
        ]
    );
}

#[test]
fn its_own_requirement() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", cite("a/y"))]);
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

/// The scenarios' repository: `a` with paths `["tests/**"]` and no `tests`, and `tests/t.rs`
/// citing `b/y`.
fn paths_only() -> TempDir {
    let dir = repo(None, &[("tests/t.rs", cite("b/y"))]);
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &component("[\"tests/**\"]", None),
    );
    dir
}

#[test]
fn a_file_only_paths_own() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = paths_only();
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_marker_naming_nothing_is_left_to_dangling_markers() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(
        Some("[\"tests/a/**\"]"),
        &[("tests/a/t.rs", cite("b/nope"))],
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_capability_is_the_capability_components() {
    // req:check/tests-cite-their-owner@edb5a1b0
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
            "tests/a/t.rs:1: check/tests-cite-their-owner: cites capability/c of `capability`, but the file is `a`'s; move the test to `capability`'s files"
        ]
    );
}

#[test]
fn run_listed_reports_it_too() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(Some("[\"tests/a/**\"]"), &[("tests/a/t.rs", cite("b/y"))]);
    let files = ["specs/SPEC.md", "tests/a/t.rs"].map(std::path::PathBuf::from);
    let found: Vec<_> = clm_check::run_listed(dir.path(), &files)
        .into_iter()
        .filter(|v| v.rule == "check/tests-cite-their-owner")
        .collect();
    assert_eq!(found.len(), 1);
}

#[test]
fn exempt_until_split() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let text = format!("// x\n// y\n// z\n{}", cite("b/y"));
    let dir = repo(
        Some("[\"tests/a/**\"]"),
        &[("tests/a/t.rs", text.clone()), ("tests/a/u.rs", text)],
    );
    let spec = dir.path().join("specs/a/SPEC.md");
    let original = fs::read_to_string(&spec).unwrap();
    // Only the file the exemption names is excused, and only from this rule.
    let with = |rule: &str, file: &str| {
        let entry = format!(
            "exemptions: [{{rule: {rule}, files: [\"{file}\"], until: backlog/x}}]\n---\n## Contract"
        );
        fs::write(&spec, original.replacen("---\n## Contract", &entry, 1)).unwrap();
        found(dir.path())
    };
    let line = |f: &str| {
        format!(
            "{f}:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files"
        )
    };
    assert_eq!(
        with("tests-cite-their-owner", "tests/a/t.rs"),
        [line("tests/a/u.rs")]
    );
    assert_eq!(
        with("skipped-tests", "tests/a/**"),
        [line("tests/a/t.rs"), line("tests/a/u.rs")]
    );
}

/// Gives the repository's root spec `wiring: []`, naming its wiring.
fn wire(root: &Path) {
    wire_with(root, "[]", "[]");
}

/// Gives the repository's root spec the `paths` and `wiring` given.
fn wire_with(root: &Path, paths: &str, wiring: &str) {
    let spec = root.join("specs/SPEC.md");
    let text = fs::read_to_string(&spec).unwrap();
    let wired = text.replacen(
        "paths: []\n",
        &format!("paths: {paths}\nwiring: {wiring}\n"),
        1,
    );
    assert_ne!(wired, text, "the root's frontmatter changed shape");
    fs::write(spec, wired).unwrap();
}

#[test]
fn a_file_only_paths_own_once_the_root_names_its_wiring() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = paths_only();
    wire(dir.path());
    assert_eq!(
        found(dir.path()),
        [
            "tests/t.rs:1: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files"
        ]
    );
}

#[test]
fn a_file_no_component_owns_is_not_judged_under_wiring() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(None, &[("elsewhere/t.rs", cite("b/y"))]);
    wire(dir.path());
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_capability_test_under_wiring() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(None, &[("tests/capability/see.rs", cite("capability/see"))]);
    wire(dir.path());
    fs::write(
        dir.path().join("CAPABILITIES.md"),
        "# Capabilities\n\n## Capabilities\n\n### Capability: see\nA person can.\n\n#### Why\nBecause.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n",
    )
    .unwrap();
    write(
        dir.path(),
        "specs/capability/SPEC.md",
        &component("[\"tests/capability/**\"]", None),
    );
    assert_eq!(found(dir.path()), Vec::<String>::new());
}

#[test]
fn a_wiring_file() {
    // req:check/tests-cite-their-owner@edb5a1b0
    // Every build may edit a wiring file, but a test there is still its owner's, the root's.
    let dir = repo(None, &[("src/main.rs", cite("b/y"))]);
    wire_with(dir.path(), "[\"src/main.rs\"]", "[\"src/main.rs\"]");
    assert_eq!(
        found(dir.path()),
        [
            "src/main.rs:1: check/tests-cite-their-owner: cites b/y of `b`, but the file is `clm`'s; move the test to `b`'s files"
        ]
    );
}

#[test]
fn a_marker_outside_a_test_is_not_judged() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let outside = format!("fn f() {{\n    // {}:b/y\n}}\n", concat!("req", ""));
    let dir = repo(None, &[("src/a/x.rs", outside.clone())]);
    wire(dir.path());
    assert_eq!(found(dir.path()), Vec::<String>::new());
    // The same marker in a test is.
    let dir = repo(None, &[("src/a/x.rs", format!("{outside}{}", cite("b/y")))]);
    wire(dir.path());
    assert_eq!(
        found(dir.path()),
        [
            "src/a/x.rs:4: check/tests-cite-their-owner: cites b/y of `b`, but the file is `a`'s; move the test to `b`'s files"
        ]
    );
}

#[test]
fn an_architecture_entrys_marker_is_not_judged() {
    // req:check/tests-cite-their-owner@edb5a1b0
    let dir = repo(None, &[("src/a/x.rs", cite("architecture/git-is-truth"))]);
    wire(dir.path());
    fs::write(
        dir.path().join("ARCHITECTURE.md"),
        "# Architecture\n\n## Constraints\n\n### Constraint: git-is-truth\n**Checked.** Git SHALL be the truth.\n\n#### Scenario: s\n- GIVEN g\n- WHEN w\n- THEN t\n\n#### Enforced by\n- a/y\n",
    )
    .unwrap();
    let (graph, _) = clm_model::load(dir.path());
    assert!(graph.requirement("architecture/git-is-truth").is_some());
    assert_eq!(found(dir.path()), Vec::<String>::new());
}
