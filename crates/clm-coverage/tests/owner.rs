mod common;

use std::path::Path;

use clm_coverage::{owner, pending};
use common::{ROOT, graph, marker, spec, write};
use tempfile::TempDir;

#[test]
fn scenario_owned_and_unowned() {
    // req:check.coverage/owner@50c03c13
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/a/**"]"#, &[]));
    write(d, "specs/a/b/SPEC.md", &spec(r#"["crates/b/**"]"#, &[]));
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("crates/a/src/lib.rs")), Some("a"));
    assert_eq!(owner(&g, Path::new("crates/b/x.rs")), Some("a.b"));
    assert_eq!(owner(&g, Path::new("tools/x.sh")), None);
}

#[test]
fn the_deepest_node_then_the_first_by_id() {
    // req:check.coverage/owner@50c03c13
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/**"]"#, &[]));
    write(d, "specs/a/b/SPEC.md", &spec(r#"["crates/x/**"]"#, &[]));
    write(d, "specs/a/c/SPEC.md", &spec(r#"["crates/x/**"]"#, &[]));
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("crates/x/t.rs")), Some("a.b"));
    assert_eq!(owner(&g, Path::new("crates/y/t.rs")), Some("a"));
}

#[test]
fn pending_puts_an_unowned_file_on_the_root() {
    // req:check.coverage/owner@50c03c13
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec(r#"["crates/a/**"]"#, &[]));
    // `g/x` is gone with its node: its markers move to the node owning their file.
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    let g = graph(d);
    let found: Vec<(String, String)> = pending(
        &g,
        &[
            marker("g/x", None, "crates/a/t.rs", 1),
            marker("g/x", None, "old/t.rs", 2),
        ],
    )
    .into_iter()
    .map(|p| (p.node.clone(), p.to_string()))
    .collect();
    assert_eq!(
        found,
        [
            ("a".to_string(), "g/x obsolete crates/a/t.rs:1".to_string()),
            ("clm".to_string(), "g/x obsolete old/t.rs:2".to_string()),
        ]
    );
}

#[test]
fn scenario_tests_inside_another_components_paths() {
    // req:check.coverage/owner@50c03c13
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/harness/SPEC.md", &spec(r#"["tests/**"]"#, &[]));
    write(
        d,
        "specs/a/SPEC.md",
        &spec(r#"["crates/a/**"]"#, &[]).replace(
            "paths: [\"crates/a/**\"]",
            "paths: [\"crates/a/**\"]\ntests: [\"tests/a/**\"]",
        ),
    );
    let g = graph(d);
    assert_eq!(owner(&g, Path::new("tests/a/conftest.py")), Some("a"));
    assert_eq!(owner(&g, Path::new("tests/helpers.py")), Some("harness"));
}
