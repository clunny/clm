//! The frontmatter rules that depend on the node's ID: `optional` and `outside` are the
//! root's business, and the root's `outside` globs.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{Violation, check_node_frontmatter, load};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: specd\nowners: [\"@a\"]";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A repo whose root node has frontmatter `FM` plus `root`, and, when `a` is given, a
/// node `a` with `FM` plus `a`.
fn repo(root: &str, a: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        &format!("---\n{FM}{root}\n---\n"),
    );
    if let Some(a) = a {
        write(
            dir.path(),
            "specs/a/SPEC.md",
            &format!("---\n{FM}{a}\n---\n"),
        );
    }
    dir
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

fn none() -> BTreeSet<String> {
    BTreeSet::new()
}

#[test]
fn root_rule_in_the_exposed_check() {
    // req:model/optional-nodes@37ade64d
    let fm = format!("{FM}\noptional: true");
    let path = Path::new("specs/SPEC.md");
    let (parsed, violations) = check_node_frontmatter("clm", path, &fm, &none()).unwrap();
    assert_eq!(rules(&violations), ["model/optional-nodes"]);
    assert_eq!(violations[0].path, path);
    assert!(violations[0].message.contains("optional"), "{violations:?}");
    assert!(parsed.optional);
    let path = Path::new("specs/a/SPEC.md");
    let (parsed, violations) = check_node_frontmatter("a", path, &fm, &none()).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
    assert!(parsed.optional);
}

#[test]
fn other_errors_are_reported_alone() {
    // req:model/optional-nodes@37ade64d
    let path = Path::new("specs/SPEC.md");
    for (extra, rule) in [
        ("\noptional: maybe", "model/optional-nodes"),
        (
            "\noptional: true\nsize_budget: { lines: 1 }",
            "model/size-budget",
        ),
        ("\noptional: true\ncolour: blue", "model/valid-frontmatter"),
        (
            "\noutside: [\"x\"]\ncolour: blue",
            "model/valid-frontmatter",
        ),
    ] {
        let fm = format!("{FM}{extra}");
        let errors = check_node_frontmatter("clm", path, &fm, &none()).unwrap_err();
        assert_eq!(rules(&errors), [rule], "{extra}: {errors:?}");
    }
}

#[test]
fn globs_on_the_root() {
    // req:model/outside-globs@013b7044
    let dir = repo("\noutside: [\"src/web/**\", \"README.md\"]", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.outside(),
        Some(&["src/web/**", "README.md"].map(String::from)[..])
    );
}

#[test]
fn empty_list() {
    // req:model/outside-globs@013b7044
    let dir = repo("\noutside: []", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.outside(), Some(&[][..]));
}

#[test]
fn no_key() {
    // req:model/outside-globs@013b7044
    let dir = repo("", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.outside(), None);
}

#[test]
fn on_another_node() {
    // req:model/outside-globs@013b7044
    for value in ["[\"src/**\"]", "[]"] {
        let dir = repo("", Some(&format!("\noutside: {value}")));
        let (graph, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/outside-globs"], "{value}");
        assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
        assert!(violations[0].message.contains("outside"), "{violations:?}");
        // The node stays loaded, as for the root's `optional`.
        assert!(graph.nodes.contains_key("a"));
        assert_eq!(graph.outside(), None);
    }
}

#[test]
fn bad_value() {
    // req:model/outside-globs@013b7044
    for value in [
        " \"src/**\"",
        " [\"\"]",
        "",
        " [1]",
        " { a: b }",
        " [\"a\", ~]",
    ] {
        let dir = repo(&format!("\noutside:{value}"), None);
        let (_, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/outside-globs"], "{value}");
        assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
        assert!(violations[0].message.contains("outside"), "{violations:?}");
    }
}

#[test]
fn the_frontmatter_check() {
    // req:model/outside-globs@013b7044
    let path = Path::new("specs/a/SPEC.md");
    let root = Path::new("specs/SPEC.md");
    let globs = format!("{FM}\noutside: [\"src/**\"]");
    // A wrong shape on any node, the root included, fails alone.
    let bad = format!("{FM}\noutside: [\"\"]");
    for (id, path) in [("a", path), ("clm", root)] {
        let errors = check_node_frontmatter(id, path, &bad, &none()).unwrap_err();
        assert_eq!(rules(&errors), ["model/outside-globs"], "{id}");
        assert_eq!(errors[0].path, path);
    }
    // The key off the root keeps the frontmatter parsed.
    let (_, violations) = check_node_frontmatter("a", path, &globs, &none()).unwrap();
    assert_eq!(rules(&violations), ["model/outside-globs"]);
    let (fm, violations) =
        check_node_frontmatter("clm", Path::new("specs/SPEC.md"), &globs, &none()).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(fm.outside, Some(vec!["src/**".to_string()]));
}
