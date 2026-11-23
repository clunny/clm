//! How `clm pending [<component>]` exits: 0 with its list, an empty one included, and 1 with
//! the violations, for an invalid graph or a component the graph doesn't hold
//! (`arguments-and-exit`).

mod common;

use std::fs;
use std::path::Path;

use common::clm;
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// `clm pending [<component>] --root <root>` through `clm-view-repo`'s call: its status and
/// its standard output.
fn clm_pending(root: &Path, node: Option<&str>) -> (Option<i32>, String) {
    let args: Vec<&str> = std::iter::once("pending").chain(node).collect();
    let out = clm(root, &args);
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// Component `a` with `a/x`, `a/y` and `a/z`, and an `applied` change `r` with no deltas.
fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(
        d,
        "specs/SPEC.md",
        "---\ntitle: R\nstatus: active\nowners: []\n---\n",
    );
    let mut spec =
        String::from("---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n");
    for slug in ["x", "y", "z"] {
        spec.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    write(d, "specs/a/SPEC.md", &spec);
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    dir
}

#[test]
fn violations_and_unknown_nodes_exit_1() {
    // req:view.repo/arguments-and-exit@c4b91292
    let dir = repo();
    let d = dir.path();
    let (code, out) = clm_pending(d, Some("nope"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("unknown component `nope`"), "{out}");
    write(d, "specs/b/SPEC.md", "---\ntitle: B\n---\n");
    let (code, out) = clm_pending(d, Some("a"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("model/valid-frontmatter"), "{out}");
    // The violations stand in place of the list: they are all it prints.
    let (_, violations) = clm_model::load(d);
    let expected: String = violations.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(out, expected);
}

#[test]
fn scenario_removed_node_from_the_cli() {
    // req:view.repo/arguments-and-exit@c4b91292
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "changes/r/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(
        d,
        "specs/v/SPEC.md",
        "---\ntitle: V\nstatus: active\nowners: []\npaths: [\"crates/v/**\"]\n---\n",
    );
    // Nothing pending on `v` yet: an empty list exits 0.
    assert_eq!(clm_pending(d, Some("v")), (Some(0), String::new()));
    write(
        d,
        "crates/v/tests/t.rs",
        &format!("#[test]\nfn t() {{\n    // {REQ}g/x\n}}\n"),
    );
    assert_eq!(
        clm_pending(d, Some("v")),
        (Some(0), "g/x obsolete crates/v/tests/t.rs:3\n".to_string())
    );
    // `g`, which the applied change removed, is no component the graph holds.
    let (code, out) = clm_pending(d, Some("g"));
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("unknown component `g`"), "{out}");
}
