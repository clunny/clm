//! `bundle_nodes`: the nodes whose specs make up a bundle.

use std::fs;
use std::path::Path;

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn node(title: &str, deps: &str) -> String {
    format!(
        "---\ntitle: {title}\nstatus: specd\nowners: []\ndepends_on: [{deps}]\n---\n\n\
         ## Contract\nc {title}\n\n## Design\nd\n"
    )
}

#[test]
fn spine_then_dependencies_off_the_spine() {
    let dir = tempfile::TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("Root", ""));
    write(dir.path(), "specs/z/SPEC.md", &node("Z", ""));
    write(dir.path(), "specs/a/SPEC.md", &node("A", ""));
    // `b` depends on its parent (already on the spine) and on `z` and `a`.
    write(dir.path(), "specs/p/SPEC.md", &node("P", ""));
    write(dir.path(), "specs/p/b/SPEC.md", &node("B", "p, z, a, z"));
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");

    let root = graph
        .nodes
        .values()
        .find(|n| n.parent.is_none())
        .unwrap()
        .id
        .clone();
    assert_eq!(
        clm_context::bundle_nodes(&graph, "p.b"),
        [root.as_str(), "p", "p.b", "a", "z"]
    );
    assert!(clm_context::bundle_nodes(&graph, "missing").is_empty());
}

#[test]
fn test_dependency_is_not_in_the_bundle() {
    // req:context/bundle-contents@1db0ecd6
    let dir = tempfile::TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", &node("Root", ""));
    write(dir.path(), "specs/app/SPEC.md", &node("App", ""));
    let feat = node("Feat", "").replace("depends_on", "test_depends_on");
    write(
        dir.path(),
        "specs/feat/SPEC.md",
        &feat.replace("[]", "[app]"),
    );
    let (graph, violations) = clm_model::load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(!clm_context::bundle_nodes(&graph, "feat").contains(&"app".to_string()));
    let out = clm_context::bundle_from(&graph, None, "feat").unwrap();
    assert!(!out.contains("c App"), "{out}");
}
