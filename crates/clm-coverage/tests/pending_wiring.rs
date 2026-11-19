//! `pending-report`: where an obsolete requirement of a component the graph holds is pending
//! once the root sets `wiring`, and what blocks each of its lines.

mod common;

use std::path::Path;

use clm_coverage::{Pending, pending};
use common::{graph, marker, spec, write};
use tempfile::TempDir;

/// The root's spec, with `wiring` set to `wiring` when given.
fn root(wiring: Option<&str>) -> String {
    let key = wiring.map_or(String::new(), |w| format!("wiring: {w}\n"));
    format!("---\ntitle: R\nstatus: active\nowners: []\n{key}---\n")
}

/// A root with `wiring`, components `b` and `c` owning `crates/b/**` and `crates/c/**`, and an
/// `applied` change that removed `b/gone`.
fn repo(wiring: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", &root(wiring));
    write(d, "specs/b/SPEC.md", &spec(r#"["crates/b/**"]"#, &["kept"]));
    write(d, "specs/c/SPEC.md", &spec(r#"["crates/c/**"]"#, &[]));
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/b.md",
        "---\ncomponent: b\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
    dir
}

/// The obsolete lines of `id`, each with the component it is pending on.
fn obsolete(root: &Path, id: &str, at: &[(&str, usize)]) -> Vec<(String, String)> {
    lines_of(&items(root, id, at), id)
}

fn items(root: &Path, id: &str, at: &[(&str, usize)]) -> Vec<Pending> {
    let markers: Vec<_> = at.iter().map(|(p, l)| marker(id, None, p, *l)).collect();
    pending(&graph(root), &markers)
}

fn lines_of(items: &[Pending], id: &str) -> Vec<(String, String)> {
    items
        .iter()
        .filter(|p| p.id == id)
        .map(|p| (p.node.clone(), p.to_string()))
        .collect()
}

fn pair(node: &str, line: &str) -> (String, String) {
    (node.to_string(), line.to_string())
}

#[test]
fn scenario_an_obsolete_marker_in_another_components_file() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some("[]"));
    let d = dir.path();
    assert_eq!(
        obsolete(d, "b/gone", &[("crates/b/t.rs", 3), ("crates/c/t.rs", 8)]),
        [
            pair("b", "b/gone obsolete crates/b/t.rs:3 blocked b/gone"),
            pair("c", "b/gone obsolete crates/c/t.rs:8"),
        ]
    );
    assert_eq!(
        obsolete(d, "b/gone", &[("crates/b/t.rs", 3)]),
        [pair("b", "b/gone obsolete crates/b/t.rs:3")]
    );
}

#[test]
fn the_own_line_alone_waits_on_its_own_id() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some("[]"));
    let found = items(
        dir.path(),
        "b/gone",
        &[("crates/b/t.rs", 3), ("crates/c/t.rs", 8)],
    );
    let waits: Vec<(&str, &[String])> = found
        .iter()
        .filter(|p| p.id == "b/gone")
        .map(|p| (p.node.as_str(), p.waits_on.as_slice()))
        .collect();
    assert_eq!(waits, [("b", &["b/gone".to_string()][..]), ("c", &[][..])]);
}

#[test]
fn scenario_only_another_components_file_cites_it() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some("[]"));
    assert_eq!(
        obsolete(dir.path(), "b/gone", &[("crates/c/t.rs", 8)]),
        [pair("c", "b/gone obsolete crates/c/t.rs:8")]
    );
}

#[test]
fn scenario_an_obsolete_marker_in_a_wiring_file() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some(r#"["crates/c/shared.rs"]"#));
    assert_eq!(
        obsolete(
            dir.path(),
            "b/gone",
            &[("crates/c/shared.rs", 2), ("old/t.rs", 4)]
        ),
        [pair("b", "b/gone obsolete crates/c/shared.rs:2 old/t.rs:4")]
    );
}

#[test]
fn scenario_an_obsolete_marker_with_no_wiring() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(None);
    assert_eq!(
        obsolete(
            dir.path(),
            "b/gone",
            &[("crates/b/t.rs", 3), ("crates/c/t.rs", 8)]
        ),
        [pair("b", "b/gone obsolete crates/b/t.rs:3 crates/c/t.rs:8")]
    );
}

#[test]
fn scenario_a_move_into_the_files_owner() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some("[]"));
    let d = dir.path();
    write(d, "specs/c/SPEC.md", &spec(r#"["crates/c/**"]"#, &["x"]));
    write(
        d,
        "changes/m/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(d, "changes/m/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/m/deltas/c.md",
        "---\ncomponent: c\n---\n## ADDED Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n",
    );
    let found = items(d, "g/x", &[("crates/c/t.rs", 8)]);
    let x = graph(d).requirement("c/x").unwrap().version().to_string();
    let on_c: Vec<String> = found
        .iter()
        .filter(|p| p.node == "c")
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        on_c,
        [
            format!("c/x {x} unbuilt"),
            "g/x obsolete crates/c/t.rs:8".to_string()
        ]
    );
    // On another component's line the move still blocks it (`moved-markers-wait`).
    let found = items(d, "g/x", &[("crates/b/t.rs", 8)]);
    assert_eq!(
        lines_of(&found, "g/x"),
        [pair("b", "g/x obsolete crates/b/t.rs:8 blocked c/x")]
    );
}

#[test]
fn scenario_a_question_holds_an_obsolete_line() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(None);
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n",
    );
    assert_eq!(
        obsolete(d, "b/gone", &[("crates/b/t.rs", 3)]),
        [pair(
            "b",
            "b/gone obsolete crates/b/t.rs:3 blocked backlog/q"
        )]
    );
}

#[test]
fn a_question_holds_each_line_of_an_obsolete_requirement() {
    // req:check.coverage/pending-report@ebf1b50f
    let dir = repo(Some("[]"));
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n",
    );
    assert_eq!(
        obsolete(d, "b/gone", &[("crates/b/t.rs", 3), ("crates/c/t.rs", 8)]),
        [
            pair(
                "b",
                "b/gone obsolete crates/b/t.rs:3 blocked backlog/q,b/gone"
            ),
            pair("c", "b/gone obsolete crates/c/t.rs:8 blocked backlog/q"),
        ]
    );
}
