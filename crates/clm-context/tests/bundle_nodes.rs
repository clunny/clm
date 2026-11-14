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
