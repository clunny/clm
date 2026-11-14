use std::fs;
use std::path::Path;

use clm_gate::{Kind, evaluate_at};
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
const BODY: &str = "\n## Contract\nc\n\n## Design\nthe spine\n\n## Requirements\n\n### Requirement: x\nX SHALL hold.\n\n#### Scenario: holds\n- ok\n";

fn component(exemptions: &str) -> String {
    format!(
        "---\ntitle: A\nstatus: unbuilt\nowners: [\"@a\"]\npaths: [\"src/a/**\"]\n{exemptions}---\n{BODY}"
    )
}

const EXEMPTION: &str =
    "exemptions:\n  - rule: skipped-tests\n    files: [\"src/a/**\"]\n    until: backlog/x\n";

fn base() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    git(d, &["init", "-q", "-b", "main"]);
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &component(""));
    commit(d, "base");
    git(d, &["checkout", "-q", "-b", "pr"]);
    dir
}

const ITEM: &str = "---\nkind: deferred\ncomponents: [a]\n---\nEnds the exemption.\n";

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn an_exemption_whose_item_is_missing_blocks() {
    let dir = base();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    commit(d, "exempt");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    assert_eq!(out.kind, Kind::Spec);
    let lines: Vec<String> = out
        .violations
        .iter()
        .filter(|v| v.rule == "gate/exemption-names-its-item")
        .map(|v| format!("{}: {}: {}", v.path.display(), v.rule, v.message))
        .collect();
    assert_eq!(
        lines,
        [
            "specs/a/SPEC.md: gate/exemption-names-its-item: exemption from skipped-tests until backlog/x names no backlog item; add it"
        ]
    );
}

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn an_item_added_with_it_satisfies() {
    let dir = base();
    let d = dir.path();
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    write(d, "backlog/x.md", ITEM);
    commit(d, "exempt with its item");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    assert!(
        out.violations
            .iter()
            .all(|v| v.rule != "gate/exemption-names-its-item"),
        "{:?}",
        out.violations
    );
}

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn a_widened_exemption_is_tested_and_an_old_one_is_not() {
    let dir = base();
    let d = dir.path();
    // main already holds the exemption, its item since deleted: not the branch's to answer for
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    commit(d, "exempt on main");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    // the branch rewords the component, leaving its exemption as it was
    write(
        d,
        "specs/a/SPEC.md",
        &format!("{}\nMore.\n", component(EXEMPTION)),
    );
    commit(d, "reword");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    assert!(
        out.violations
            .iter()
            .all(|v| v.rule != "gate/exemption-names-its-item"),
        "{:?}",
        out.violations
    );
    // widened: another glob under the same rule and item
    let widened = EXEMPTION.replace("[\"src/a/**\"]", "[\"src/a/**\", \"tests/**\"]");
    write(d, "specs/a/SPEC.md", &component(&widened));
    commit(d, "widen");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    assert_eq!(
        out.violations
            .iter()
            .filter(|v| v.rule == "gate/exemption-names-its-item")
            .count(),
        1
    );
}

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn another_item_or_a_new_component_is_tested() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    commit(d, "exempt on main");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    // the same rule and files, now until another item
    let moved = EXEMPTION.replace("backlog/x", "backlog/y");
    write(d, "specs/a/SPEC.md", &component(&moved));
    commit(d, "move");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    let rules: Vec<&str> = out
        .violations
        .iter()
        .map(|v| v.rule)
        .filter(|r| *r == "gate/exemption-names-its-item")
        .collect();
    assert_eq!(rules, ["gate/exemption-names-its-item"]);
    // a component new on the branch counts every exemption
    write(d, "backlog/y.md", ITEM);
    write(d, "specs/b/SPEC.md", &component(EXEMPTION));
    commit(d, "new component");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    let paths: Vec<String> = out
        .violations
        .iter()
        .filter(|v| v.rule == "gate/exemption-names-its-item")
        .map(|v| v.path.display().to_string())
        .collect();
    assert_eq!(paths, ["specs/b/SPEC.md"]);
}

fn named(out: &clm_gate::Outcome) -> usize {
    out.violations
        .iter()
        .filter(|v| v.rule == "gate/exemption-names-its-item")
        .count()
}

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn narrowing_a_new_rule_and_each_offender() {
    let dir = base();
    let d = dir.path();
    let two = EXEMPTION.replace("[\"src/a/**\"]", "[\"src/a/**\", \"tests/**\"]");
    git(d, &["checkout", "-q", "main"]);
    write(d, "specs/a/SPEC.md", &component(&two));
    commit(d, "exempt on main");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    // narrowed, and only its `why` changed: never blocks
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    commit(d, "narrow");
    assert_eq!(named(&evaluate_at(d, "main", "pr", false).unwrap()), 0);
    let why = format!("{two}    why: because\n");
    write(d, "specs/a/SPEC.md", &component(&why));
    commit(d, "why");
    assert_eq!(named(&evaluate_at(d, "main", "pr", false).unwrap()), 0);
    // another rule over the same files and item is an added exemption
    let other = two.replace("skipped-tests", "undeclared-imports");
    write(
        d,
        "specs/a/SPEC.md",
        &component(&format!("{two}{}", &other["exemptions:\n".len()..])),
    );
    commit(d, "rule");
    assert_eq!(named(&evaluate_at(d, "main", "pr", false).unwrap()), 1);
    // each offender is reported, in every file
    write(d, "specs/b/SPEC.md", &component(&other));
    commit(d, "b");
    assert_eq!(named(&evaluate_at(d, "main", "pr", false).unwrap()), 2);
}

// req:gate/exemption-names-its-item@9eb8dcde
#[test]
fn an_item_on_main_counts_and_only_a_spec_branch_is_judged() {
    let dir = base();
    let d = dir.path();
    git(d, &["checkout", "-q", "main"]);
    write(d, "backlog/x.md", ITEM);
    commit(d, "item on main");
    git(d, &["checkout", "-q", "pr"]);
    git(d, &["merge", "-q", "--ff-only", "main"]);
    write(d, "specs/a/SPEC.md", &component(EXEMPTION));
    commit(d, "exempt");
    assert_eq!(named(&evaluate_at(d, "main", "pr", false).unwrap()), 0);
    // a code branch is no spec branch: its exemption is the check's, not the gate's
    git(d, &["rm", "-q", "backlog/x.md"]);
    write(d, "src/a/lib.rs", "pub fn a() {}\n");
    commit(d, "code, and the item gone");
    let out = evaluate_at(d, "main", "pr", false).unwrap();
    assert_eq!(out.kind, Kind::Mixed);
    assert_eq!(named(&out), 0);
}
