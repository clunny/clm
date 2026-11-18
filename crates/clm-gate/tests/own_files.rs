use std::fs;
use std::path::Path;

use clm_gate::{Kind, evaluate_at, foreign_files};
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

fn root(extra: &str) -> String {
    format!(
        "---\ntitle: Root\nstatus: active\nowners: []\npaths: [\"Cargo.lock\"]\n{extra}---\n\n## Contract\nroot\n\n## Design\nroot design\n\n## Requirements\n"
    )
}

fn component(title: &str, extra: &str, requirements: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: [\"@a\"]\n{extra}---\n\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n{requirements}"
    )
}

const X_SEE: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n\n#### Serves\n- capability/see\n";
const X: &str = "\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

/// Components `a`, `b` and `capability`, on `main`; `a/x` serves `capability/see`.
fn base(wiring: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    let wiring = wiring.map_or(String::new(), |w| format!("wiring: {w}\n"));
    write(d, "specs/SPEC.md", &root(&wiring));
    write(
        d,
        "specs/a/SPEC.md",
        &component("A", "paths: [\"crates/a/**\"]\n", X_SEE),
    );
    write(
        d,
        "specs/b/SPEC.md",
        &component("B", "paths: [\"crates/b/**\"]\n", X),
    );
    write(
        d,
        "specs/capability/SPEC.md",
        &component("Capability", "paths: [\"tests/capability/**\"]\n", X),
    );
    write(d, "Cargo.lock", "# lock\n");
    write(d, "crates/a/src/lib.rs", "pub fn a() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b() {}\n");
    write(d, "crates/b/tests/old.rs", &cite("a/y", "fn old() {}\n"));
    write(d, "crates/b/tests/cli.rs", "fn cli() {}\n");
    write(
        d,
        "tests/capability/see.rs",
        &cite("capability/see", "fn see() {}\n"),
    );
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "impl/a"]);
    dir
}

/// Text citing `id`, built so this file holds no marker of its own.
fn cite(id: &str, body: &str) -> String {
    format!("// {}{id}@00000000\n{body}", concat!("req", ":"))
}

fn foreign(d: &Path, component: &str) -> Vec<(String, String)> {
    foreign_files(d, "main", "HEAD", component)
        .unwrap()
        .into_iter()
        .map(|f| (f.path, f.owner))
        .collect()
}

fn pair(path: &str, owner: &str) -> (String, String) {
    (path.to_string(), owner.to_string())
}

// req:gate/own-files@343514b7
#[test]
fn another_components_source_is_foreign_and_the_gate_ignores_it() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "crates/a/src/lib.rs", "pub fn a2() {}\n");
    write(d, "crates/b/src/lib.rs", "pub fn b2() {}\n");
    commit(d, "change");
    assert_eq!(foreign(d, "a"), vec![pair("crates/b/src/lib.rs", "b")]);
    let outcome = evaluate_at(d, "main", "HEAD", false).unwrap();
    assert_eq!(outcome.kind, Kind::Code);
    assert!(outcome.violations.is_empty());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_gate::command(["--base", "main"], d, &|_| None, &mut out, &mut err);
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&err));
    let out = String::from_utf8(out).unwrap();
    assert_eq!(out.lines().next(), Some("code"), "{out}");
}

// req:gate/own-files@343514b7
#[test]
fn a_requirement_without_serves_of_its_own_serves_what_its_component_does() {
    let dir = base(Some("[]"));
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(
        d,
        "specs/b/SPEC.md",
        &component(
            "B",
            "paths: [\"crates/b/**\"]\nserves: [\"capability/see\"]\n",
            X,
        ),
    );
    commit(d, "b serves see");
    git(d, &["checkout", "-q", "impl/a"]);
    git(d, &["merge", "-q", "main"]);
    write(
        d,
        "tests/capability/see.rs",
        &cite("capability/see", "fn see() { /* moved */ }\n"),
    );
    commit(d, "edit");
    assert_eq!(foreign(d, "b"), Vec::new());
}

// req:gate/own-files@343514b7
#[test]
fn wiring_and_unowned_files_are_the_builds_to_change() {
    let dir = base(Some("[\"Cargo.lock\"]"));
    let d = dir.path();
    write(d, "crates/a/src/lib.rs", "pub fn a2() {}\n");
    write(d, "Cargo.lock", "# lock 2\n");
    write(d, "scripts/unowned.sh", "echo\n");
    commit(d, "change");
    assert_eq!(foreign(d, "a"), Vec::new());
}

// req:gate/own-files@343514b7
#[test]
fn a_root_file_no_wiring_glob_matches_is_foreign() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "Cargo.lock", "# lock 2\n");
    commit(d, "change");
    assert_eq!(foreign(d, "a"), vec![pair("Cargo.lock", "clm")]);
}

// req:gate/own-files@343514b7
#[test]
fn a_test_that_cites_the_component_in_anothers_file_is_foreign() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "crates/b/tests/cli.rs", &cite("a/x", "fn cli() {}\n"));
    fs::remove_file(d.join("crates/b/tests/old.rs")).unwrap();
    commit(d, "change");
    assert_eq!(
        foreign(d, "a"),
        vec![
            pair("crates/b/tests/cli.rs", "b"),
            pair("crates/b/tests/old.rs", "b"),
        ]
    );
}

// req:gate/own-files@343514b7
#[test]
fn a_capability_test_may_be_edited_never_deleted_by_a_serving_build() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(
        d,
        "tests/capability/see.rs",
        &cite("capability/see", "fn see() { /* moved */ }\n"),
    );
    commit(d, "edit");
    assert_eq!(foreign(d, "a"), Vec::new());
    // `b` serves nothing, so the same edit is foreign to it.
    assert_eq!(
        foreign(d, "b"),
        vec![pair("tests/capability/see.rs", "capability")]
    );
    git(d, &["reset", "-q", "--hard", "main"]);
    fs::remove_file(d.join("tests/capability/see.rs")).unwrap();
    commit(d, "delete");
    assert_eq!(
        foreign(d, "a"),
        vec![pair("tests/capability/see.rs", "capability")]
    );
}

// req:gate/own-files@343514b7
#[test]
fn a_capability_test_citing_a_longer_slug_is_foreign() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(
        d,
        "tests/capability/seeing.rs",
        &cite("capability/seeing", "fn seeing() {}\n"),
    );
    commit(d, "add");
    git(d, &["checkout", "-q", "main"]);
    git(d, &["merge", "-q", "impl/a"]);
    git(d, &["checkout", "-q", "impl/a"]);
    write(
        d,
        "tests/capability/seeing.rs",
        &cite("capability/seeing", "fn seeing() { /* moved */ }\n"),
    );
    commit(d, "edit");
    assert_eq!(
        foreign(d, "a"),
        vec![pair("tests/capability/seeing.rs", "capability")]
    );
}

// req:gate/own-files@343514b7
#[test]
fn a_capability_marker_counts_whatever_version_it_cites_and_wherever_it_sits() {
    for marker in ["capability/see", "capability/see@1234abcd"] {
        let dir = base(Some("[]"));
        let d = dir.path();
        git(d, &["checkout", "-q", "main"]);
        // Mid-line, in a string, after the code it observes.
        let text = format!(
            "fn see() {{}}\nconst NOTE: &str = \"proves {}{marker} here\";\n",
            concat!("req", ":")
        );
        write(d, "tests/capability/see.rs", &text);
        commit(d, "cite elsewhere");
        git(d, &["checkout", "-q", "impl/a"]);
        git(d, &["merge", "-q", "main"]);
        write(d, "tests/capability/see.rs", &format!("{text}// moved\n"));
        commit(d, "edit");
        assert_eq!(foreign(d, "a"), Vec::new(), "{marker}");
    }
}

// req:gate/own-files@343514b7
#[test]
fn a_capability_citation_is_read_at_the_merge_base() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "tests/capability/see.rs", "fn see() {}\n");
    commit(d, "drop the citation");
    assert_eq!(foreign(d, "a"), Vec::new());
}

// req:gate/own-files@343514b7
#[test]
fn a_capability_test_is_never_added_by_a_serving_build() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(
        d,
        "tests/capability/new.rs",
        &cite("capability/see", "fn new() {}\n"),
    );
    commit(d, "add");
    assert_eq!(
        foreign(d, "a"),
        vec![pair("tests/capability/new.rs", "capability")]
    );
}

// req:gate/own-files@343514b7
#[test]
fn nothing_is_foreign_until_the_root_sets_wiring() {
    let dir = base(None);
    let d = dir.path();
    write(d, "crates/b/src/lib.rs", "pub fn b2() {}\n");
    commit(d, "change");
    assert_eq!(foreign(d, "a"), Vec::new());
}

// req:gate/own-files@343514b7
#[test]
fn nothing_is_foreign_to_an_unknown_component() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "crates/b/src/lib.rs", "pub fn b2() {}\n");
    commit(d, "change");
    assert_eq!(foreign(d, "z"), Vec::new());
}

// req:gate/own-files@343514b7
#[test]
fn results_are_sorted_by_path_and_read_from_the_revision_not_the_checkout() {
    let dir = base(Some("[]"));
    let d = dir.path();
    write(d, "crates/b/src/z.rs", "z\n");
    write(d, "Cargo.lock", "# lock 2\n");
    write(d, "crates/b/src/a.rs", "a\n");
    commit(d, "change");
    let head = git(d, &["rev-parse", "HEAD"]);
    git(d, &["checkout", "-q", "main"]);
    let found: Vec<String> = foreign_files(d, "main", &head, "a")
        .unwrap()
        .into_iter()
        .map(|f| f.path)
        .collect();
    assert_eq!(
        found,
        ["Cargo.lock", "crates/b/src/a.rs", "crates/b/src/z.rs"]
    );
}
