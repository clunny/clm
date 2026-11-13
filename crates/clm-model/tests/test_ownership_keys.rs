//! The `tests` frontmatter key.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clm_model::{Violation, check_node_frontmatter, load};
use tempfile::TempDir;

const FM: &str = "title: T\nstatus: unbuilt\nowners: [\"@a\"]";

fn write(dir: &Path, id: &str, extra: &str) {
    let path = dir.join(format!("specs/{id}/SPEC.md"));
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, format!("---\n{FM}{extra}\n---\n")).unwrap();
}

/// A repo with a root and a node `a` with `FM` plus `a`.
fn repo(a: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::create_dir_all(dir.path().join("specs")).unwrap();
    fs::write(
        dir.path().join("specs/SPEC.md"),
        format!("---\n{FM}\n---\n"),
    )
    .unwrap();
    write(dir.path(), "a", a);
    dir
}

fn rules(violations: &[Violation]) -> Vec<&str> {
    violations.iter().map(|v| v.rule).collect()
}

#[test]
fn keys_parsed() {
    // req:model/test-ownership-keys@c678f739
    let (graph, violations) = load(repo("\ntests: [\"tests/a/**\", \"x.rs\"]").path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.nodes["a"].frontmatter.tests, ["tests/a/**", "x.rs"]);
}

#[test]
fn keys_absent() {
    // req:model/test-ownership-keys@c678f739
    let (graph, violations) = load(repo("").path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.nodes["a"].frontmatter.tests.is_empty());
}

#[test]
fn bad_shape() {
    // req:model/test-ownership-keys@c678f739
    for extra in [
        "\ntests: \"tests/**\"",
        "\ntests: [\"\"]",
        "\ntests: [1]",
        "\ntests:",
    ] {
        let (_, violations) = load(repo(extra).path());
        assert_eq!(rules(&violations), ["model/test-ownership-keys"], "{extra}");
        assert_eq!(violations[0].path, Path::new("specs/a/SPEC.md"), "{extra}");
        assert!(violations[0].message.contains("tests"), "{violations:?}");
        // The frontmatter check `apply` runs rejects the same shapes.
        let fm = format!("{FM}{extra}");
        let path = Path::new("specs/a/SPEC.md");
        let errs = check_node_frontmatter("a", path, &fm, &BTreeSet::new()).unwrap_err();
        assert_eq!(rules(&errs), ["model/test-ownership-keys"], "{extra}");
        assert!(errs[0].message.contains("tests"));
    }
}
