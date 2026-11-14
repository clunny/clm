//! The root's `wiring` globs: the files every build may edit.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{Violation, check_node_frontmatter, load};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: unbuilt\nowners: [\"@a\"]";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A repo whose root has frontmatter `FM` plus `root`, and, when `a` is given, a
/// component `a` with `FM` plus `a`.
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

#[test]
fn globs_on_the_root() {
    // req:model/wiring-globs@85896262
    let dir = repo("\nwiring: [\"Cargo.toml\", \"src/main.rs\"]", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.wiring(),
        Some(&["Cargo.toml".to_string(), "src/main.rs".to_string()][..])
    );
    assert_eq!(graph.outside(), None);
}

#[test]
fn empty_list() {
    // req:model/wiring-globs@85896262
    let dir = repo("\nwiring: []", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.wiring(), Some(&[][..]));
}

#[test]
fn no_key() {
    // req:model/wiring-globs@85896262
    let dir = repo("", None);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.wiring(), None);
}

#[test]
fn on_another_component() {
    // req:model/wiring-globs@85896262
    let dir = repo("", Some("\nwiring: [\"src/**\"]"));
    let (_, violations) = load(dir.path());
    assert_eq!(rules(&violations), ["model/wiring-globs"]);
    assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"));
    assert!(violations[0].message.contains("wiring"), "{violations:?}");
}

#[test]
fn bad_value() {
    // req:model/wiring-globs@85896262
    for value in [
        " \"src/**\"",
        " [\"\"]",
        "",
        " [1]",
        " { a: b }",
        " [\"a\", ~]",
    ] {
        let dir = repo(&format!("\nwiring:{value}"), None);
        let (graph, violations) = load(dir.path());
        assert_eq!(rules(&violations), ["model/wiring-globs"], "{value}");
        assert_eq!(violations[0].path, Path::new("specs/SPEC.md"));
        assert!(violations[0].message.contains("wiring"), "{violations:?}");
        assert_eq!(graph.wiring(), None);
    }
}

#[test]
fn the_frontmatter_check() {
    // req:model/wiring-globs@85896262
    let none = BTreeSet::new();
    let path = Path::new("specs/a/SPEC.md");
    let root = Path::new("specs/SPEC.md");
    let globs = format!("{FM}\nwiring: [\"Cargo.toml\"]");
    // A wrong shape on any component, the root included, fails alone.
    let bad = format!("{FM}\nwiring: [\"\"]");
    for (id, path) in [("a", path), ("clm", root)] {
        let errors = check_node_frontmatter(id, path, &bad, &none).unwrap_err();
        assert_eq!(rules(&errors), ["model/wiring-globs"], "{id}");
        assert_eq!(errors[0].path, path);
    }
    // The key off the root keeps the frontmatter parsed.
    let (_, violations) = check_node_frontmatter("a", path, &globs, &none).unwrap();
    assert_eq!(rules(&violations), ["model/wiring-globs"]);
    let (fm, violations) = check_node_frontmatter("clm", root, &globs, &none).unwrap();
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(fm.wiring, Some(vec!["Cargo.toml".to_string()]));
}
