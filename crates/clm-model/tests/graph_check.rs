//! The node parser and the graph check that loading runs, exposed so a caller holding a
//! graph checks it by the same rules.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{Violation, check_graph, load, parse_node};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: specd\nowners: [\"@a\"]";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn node(extra: &str, body: &str) -> String {
    format!("---\n{FM}{extra}\n---\n\n{body}")
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

#[test]
fn graph_check_on_a_loaded_graph() {
    // req:model/graph-check@e0dbe7bc
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("", ""));
    write(dir.path(), "specs/a/SPEC.md", &node("\noptional: true", ""));
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &node("\ndepends_on: [a, gone]", ""),
    );
    write(
        dir.path(),
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\nblocks: [a/none]\n---\n",
    );
    let (graph, loaded) = load(dir.path());
    let checked = check_graph(&graph);
    let mut got = rules(&checked);
    got.sort();
    assert_eq!(
        got,
        [
            "model/dependency-dag",
            "model/optional-nodes",
            "model/parse-backlog"
        ],
        "{checked:?}"
    );
    for v in &checked {
        assert!(loaded.contains(v), "{v:?} not in {loaded:?}");
    }
    assert_eq!(
        checked
            .iter()
            .find(|v| v.rule == "model/parse-backlog")
            .unwrap()
            .path,
        Path::new("backlog/x.md")
    );
    // Loading reports what the graph check does, and nothing else here.
    assert_eq!(loaded.len(), checked.len(), "{loaded:?}");
}

#[test]
fn graph_check_covers_needs_and_backlog_references() {
    // req:model/graph-check@e0dbe7bc
    let req = |slug: &str, needs: &str| {
        format!("### Requirement: {slug}\nx\n#### Scenario: s\n- y\n{needs}\n")
    };
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("", ""));
    write(
        dir.path(),
        "specs/a/SPEC.md",
        &node(
            "",
            &format!("## Requirements\n\n{}", req("x", "#### Needs\n- b/ghost")),
        ),
    );
    write(dir.path(), "specs/b/SPEC.md", &node("", ""));
    write(
        dir.path(),
        "backlog/x.md",
        "---\ntitle: X\nkind: idea\ncomponents: [ghost]\norigin: nope\n---\n",
    );
    let (graph, loaded) = load(dir.path());
    let checked = check_graph(&graph);
    let mut got = rules(&checked);
    got.sort();
    assert_eq!(
        got,
        [
            "model/parse-backlog",
            "model/parse-backlog",
            "model/requirement-needs"
        ],
        "{checked:?}"
    );
    assert_eq!(loaded.len(), checked.len(), "{loaded:?}");
    for v in &checked {
        assert!(loaded.contains(v), "{v:?} not in {loaded:?}");
    }
}

#[test]
fn node_parser_as_loading_parses() {
    // req:model/graph-check@e0dbe7bc
    let body = "## Requirements\n\n### Requirement: foo\nx\n#### Scenario: s\n- y\n\n### Requirement: foo\nx\n#### Scenario: s\n- y\n";
    let text = node("", body);
    let path = Path::new("specs/a/SPEC.md");
    let (parsed, parsed_violations) = parse_node("a", path, &text, &BTreeSet::new());
    assert_eq!(rules(&parsed_violations), ["model/unique-requirement-ids"]);
    assert!(parsed_violations[0].message.contains("a/foo"));

    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("", ""));
    write(dir.path(), "specs/a/SPEC.md", &text);
    let (graph, loaded) = load(dir.path());
    assert_eq!(loaded, parsed_violations);
    assert_eq!(parsed.as_ref(), graph.nodes.get("a"));
}

#[test]
fn node_parser_without_frontmatter_or_with_a_root_rule() {
    // req:model/graph-check@e0dbe7bc
    let path = Path::new("specs/a/SPEC.md");
    let (parsed, violations) = parse_node("a", path, "no frontmatter", &BTreeSet::new());
    assert!(parsed.is_none());
    assert_eq!(rules(&violations), ["model/parse-node-specs"]);
    // A root rule keeps the node, as loading does.
    let path = Path::new("specs/SPEC.md");
    let (parsed, violations) =
        parse_node("clm", path, &node("\noptional: true", ""), &BTreeSet::new());
    assert_eq!(rules(&violations), ["model/optional-nodes"]);
    assert_eq!(parsed.unwrap().parent, None);
}

#[test]
fn graph_check_on_an_edited_graph() {
    // req:model/graph-check@e0dbe7bc
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("", ""));
    write(dir.path(), "specs/a/SPEC.md", &node("", ""));
    write(
        dir.path(),
        "specs/b/SPEC.md",
        &node("\ndepends_on: [a]", ""),
    );
    let (mut graph, loaded) = load(dir.path());
    assert!(loaded.is_empty(), "{loaded:?}");
    assert!(check_graph(&graph).is_empty());
    graph.nodes.remove("a");
    let checked = check_graph(&graph);
    assert_eq!(rules(&checked), ["model/dependency-dag"]);
    assert_eq!(checked[0].path, Path::new("specs/b/SPEC.md"));
    assert!(checked[0].message.contains("`a`"), "{checked:?}");
}
