//! `check/untraced`, through `clm_check::run`.

use std::fs;
use std::path::Path;

use clm_check::run;
use clm_model::Violation;
use tempfile::TempDir;

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";
const RULE: &str = "check/untraced";
const CAPABILITY: &str = "# Capabilities\n\n## Capabilities\n\n### Capability: c\nA person can.\n\n#### Why\nBecause.\n\n#### Scenario: s\n- ok\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Node `a`, with `serves` in its frontmatter when given, holding `rules`.
fn node_a(serves: Option<&str>, rules: &str) -> String {
    let serves = serves
        .map(|s| format!("serves: [{s}]\n"))
        .unwrap_or_default();
    format!("---\ntitle: A\nstatus: active\nowners: []\n{serves}---\n\n## Requirements\n\n{rules}")
}

const X: &str = "### Requirement: x\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n";
const Y_SERVES: &str = "### Requirement: y\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n\n#### Serves\n- capability/c\n\n";

/// A repository with the capability entry `c` and node `a` as given.
fn repo(capabilities: Option<&str>, a: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", a);
    if let Some(text) = capabilities {
        write(d, "CAPABILITIES.md", text);
    }
    dir
}

fn untraced(dir: &TempDir) -> Vec<Violation> {
    run(dir.path())
        .into_iter()
        .filter(|v| v.rule == RULE)
        .collect()
}

#[test]
fn rule_without_serves_is_untraced() {
    // req:check/untraced@47d8df4f
    let dir = repo(Some(CAPABILITY), &node_a(None, X));
    let found = untraced(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].path, Path::new("specs/a/SPEC.md"));
    assert_eq!(
        found[0].message,
        "a/x serves nothing; give component a a serves list or the rule a #### Serves block"
    );
}

#[test]
fn rule_block_traces_it() {
    // req:check/untraced@47d8df4f
    let dir = repo(Some(CAPABILITY), &node_a(None, &format!("{X}{Y_SERVES}")));
    let found = untraced(&dir);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.starts_with("a/x "), "{found:?}");
}

#[test]
fn node_serves_traces_its_rules() {
    // req:check/untraced@47d8df4f
    let dir = repo(Some(CAPABILITY), &node_a(Some("capability/c"), X));
    assert_eq!(untraced(&dir), []);
}

#[test]
fn no_entries_no_untraced() {
    // req:check/untraced@47d8df4f
    let dir = repo(None, &node_a(None, X));
    write(
        dir.path(),
        "ARCHITECTURE.md",
        "# Architecture\n\nFree-form prose.\n",
    );
    assert_eq!(untraced(&dir), []);
}

#[test]
fn entries_are_never_untraced() {
    // req:check/untraced@47d8df4f
    let dir = repo(Some(CAPABILITY), &node_a(Some("capability/c"), X));
    write(
        dir.path(),
        "ARCHITECTURE.md",
        "# Architecture\n\n## Constraints\n\n### Constraint: unserved\nIt SHALL hold.\n\n#### Scenario: s\n- ok\n",
    );
    assert_eq!(untraced(&dir), []);
}
