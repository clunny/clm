//! The `tests` and `test_depends_on` frontmatter keys.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{Violation, check_graph, check_node_frontmatter, load};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: unbuilt\nowners: [\"@a\"]";

fn write(dir: &Path, id: &str, extra: &str) {
    let path = dir.join(format!("specs/{id}/SPEC.md"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("---\n{FM}{extra}\n---\n")).unwrap();
}

/// A repo with a root, a node `a` with `FM` plus `a`, and a node `b` with `FM` plus `b`.
fn repo(a: &str, b: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        format!("---\n{FM}\n---\n"),
    )
    .unwrap();
    write(dir.path(), "a", a);
    write(dir.path(), "b", b);
    dir
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

#[test]
fn keys_parsed() {
    // req:model/test-ownership-keys@baebdbff
    let dir = repo(
        "\ntests: [\"tests/a/**\", \"x.rs\"]\ntest_depends_on: [c, b]",
        "",
    );
    write(dir.path(), "c", "");
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(check_graph(&graph).is_empty());
    let a = &graph.nodes["a"].frontmatter;
    assert_eq!(a.tests, ["tests/a/**", "x.rs"]);
    assert_eq!(a.test_depends_on, ["c", "b"]);
}

#[test]
fn keys_absent() {
    // req:model/test-ownership-keys@baebdbff
    let (graph, violations) = load(repo("", "").path());
    assert!(violations.is_empty(), "{violations:?}");
    let a = &graph.nodes["a"].frontmatter;
    assert!(a.tests.is_empty() && a.test_depends_on.is_empty());
}

#[test]
fn bad_shape() {
    // req:model/test-ownership-keys@baebdbff
    for (extra, key) in [
        ("\ntests: \"tests/**\"", "tests"),
        ("\ntests: [\"\"]", "tests"),
        ("\ntests: [1]", "tests"),
        ("\ntests:", "tests"),
        ("\ntest_depends_on:", "test_depends_on"),
        ("\ntest_depends_on: b", "test_depends_on"),
        ("\ntest_depends_on: [1]", "test_depends_on"),
        ("\ntest_depends_on: [b, ~]", "test_depends_on"),
    ] {
        let (_, violations) = load(repo(extra, "").path());
        assert_eq!(rules(&violations), ["model/test-ownership-keys"], "{extra}");
        assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"), "{extra}");
        assert!(violations[0].message.contains(key), "{violations:?}");
        // The frontmatter check `apply` runs rejects the same shapes.
        let fm = format!("{FM}{extra}");
        let path = Path::new("specs/a/SPEC.md");
        let errs = check_node_frontmatter("a", path, &fm, &BTreeSet::new()).unwrap_err();
        assert_eq!(rules(&errs), ["model/test-ownership-keys"], "{extra}");
        assert!(errs[0].message.contains(key));
    }
}

#[test]
fn unknown_or_own_component() {
    // req:model/test-ownership-keys@baebdbff
    for id in ["ghost", "a"] {
        let dir = repo(&format!("\ntest_depends_on: [{id}]"), "");
        let (graph, violations) = load(dir.path());
        let checked = check_graph(&graph);
        assert_eq!(violations, checked);
        assert_eq!(rules(&checked), ["model/test-ownership-keys"], "{id}");
        assert_eq!(checked[0].path, Path::new("specs/a/SPEC.md"));
        let m = &checked[0].message;
        assert!(m.contains("test_depends_on") && m.contains(id), "{m}");
        // The component stays loaded.
        assert!(graph.nodes.contains_key("a"));
    }
}

#[test]
fn tests_import_a_dependant() {
    // req:model/test-ownership-keys@baebdbff
    let dir = repo("\ntest_depends_on: [b]", "\ndepends_on: [a]");
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(check_graph(&graph).is_empty());
}
