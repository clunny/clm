//! `pending-report`'s obsolete lines once the root sets `wiring`: an obsolete requirement is
//! pending on each component owning a file that cites it, its own component's line waiting on
//! its own ID while another is left; and `wait-lines`' `waits`, which resolves a line's
//! waits to lines.

mod common;

use std::path::Path;

use clm_coverage::{Marker, Pending, pending, waits};
use clm_model::Graph;
use common::{graph, marker, spec, write};
use tempfile::TempDir;

/// A root that sets `wiring` to `wiring` (none when `None`).
fn root(d: &Path, wiring: Option<&str>) {
    let wiring = wiring.map_or(String::new(), |w| format!("wiring: {w}\n"));
    write(
        d,
        "specs/SPEC.md",
        &format!("---\ntitle: R\nstatus: active\nowners: []\n{wiring}---\n"),
    );
}

/// Components `b` and `c` owning `crates/b/**` and `crates/c/**`, and an `applied` change that
/// removed `b/gone`.
fn repo(wiring: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    root(d, wiring);
    write(d, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", &["keep"]));
    write(d, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", &["keep"]));
    write(d, "changes/r/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/r/deltas/b.md",
        "---\ncomponent: b\n---\n## REMOVED Requirements\n\n### Requirement: gone <!-- base: 0123456789abcdef -->\n",
    );
    dir
}

/// Each line for `id`, as `(component, text)`, in the order `pending` returns them.
fn lines_for(g: &Graph, markers: &[Marker], id: &str) -> Vec<(String, String)> {
    pending(g, markers)
        .iter()
        .filter(|p| p.id == id)
        .map(|p| (p.node.clone(), p.to_string()))
        .collect()
}

fn pair(node: &str, text: &str) -> (String, String) {
    (node.to_string(), text.to_string())
}

#[test]
fn scenario_an_obsolete_marker_in_another_components_file() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(Some("[]"));
    let g = graph(dir.path());
    let both = [
        marker("b/gone", None, "crates/b/t.rs", 3),
        marker("b/gone", None, "crates/c/t.rs", 8),
    ];
    assert_eq!(
        lines_for(&g, &both, "b/gone"),
        [
            pair("b", "b/gone obsolete crates/b/t.rs:3 blocked b/gone"),
            pair("c", "b/gone obsolete crates/c/t.rs:8"),
        ]
    );
    let list = pending(&g, &both);
    let own = list.iter().find(|p| p.node == "b").unwrap();
    let other = list.iter().find(|p| p.node == "c").unwrap();
    assert_eq!(own.waits_on, ["b/gone"]);
    assert!(own.blocked_by.is_empty());
    assert!(other.waits_on.is_empty(), "{other:?}");
    // Once the marker in `c`'s file is gone, `b`'s line waits on nothing.
    assert_eq!(
        lines_for(&g, &both[..1], "b/gone"),
        [pair("b", "b/gone obsolete crates/b/t.rs:3")]
    );
}

#[test]
fn scenario_only_another_components_file_cites_it() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(Some("[]"));
    let g = graph(dir.path());
    let markers = [marker("b/gone", None, "crates/c/t.rs", 8)];
    assert_eq!(
        lines_for(&g, &markers, "b/gone"),
        [pair("c", "b/gone obsolete crates/c/t.rs:8")]
    );
}

#[test]
fn scenario_an_obsolete_marker_in_a_wiring_file() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(Some("[\"crates/c/shared.rs\"]"));
    let g = graph(dir.path());
    let markers = [
        marker("b/gone", None, "crates/c/shared.rs", 2),
        marker("b/gone", None, "old/t.rs", 4),
    ];
    assert_eq!(
        lines_for(&g, &markers, "b/gone"),
        [pair("b", "b/gone obsolete crates/c/shared.rs:2 old/t.rs:4")]
    );
}

#[test]
fn scenario_an_obsolete_marker_with_no_wiring() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(None);
    let g = graph(dir.path());
    let markers = [
        marker("b/gone", None, "crates/b/t.rs", 3),
        marker("b/gone", None, "crates/c/t.rs", 8),
    ];
    assert_eq!(
        lines_for(&g, &markers, "b/gone"),
        [pair("b", "b/gone obsolete crates/b/t.rs:3 crates/c/t.rs:8")]
    );
}

#[test]
fn an_item_blocks_every_line_and_the_own_wait_follows_it() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(Some("[]"));
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n",
    );
    let g = graph(d);
    let both = [
        marker("b/gone", None, "crates/b/t.rs", 3),
        marker("b/gone", None, "crates/c/t.rs", 8),
    ];
    assert_eq!(
        lines_for(&g, &both, "b/gone"),
        [
            pair(
                "b",
                "b/gone obsolete crates/b/t.rs:3 blocked backlog/q,b/gone"
            ),
            pair("c", "b/gone obsolete crates/c/t.rs:8 blocked backlog/q"),
        ]
    );
}

#[test]
fn scenario_a_question_holds_an_obsolete_line() {
    // req:check.coverage/pending-report@362c927d
    let dir = repo(None);
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [b/gone]\n---\n",
    );
    let g = graph(d);
    let markers = [marker("b/gone", None, "crates/b/t.rs", 3)];
    assert_eq!(
        lines_for(&g, &markers, "b/gone"),
        [pair(
            "b",
            "b/gone obsolete crates/b/t.rs:3 blocked backlog/q"
        )]
    );
}

/// No component `g`; an `applied` change removed `g/x` and added `c/x` as its move; `c`
/// owns `crates/c/**`.
fn moved_into_c() -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    root(d, Some("[]"));
    write(d, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", &["x"]));
    write(d, "changes/m/change.md", "---\nstatus: applied\n---\n");
    write(
        d,
        "changes/m/deltas/g.md",
        "---\ncomponent: g\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    write(
        d,
        "changes/m/deltas/c.md",
        "---\ncomponent: c\n---\n## ADDED Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n",
    );
    dir
}

fn on(g: &Graph, markers: &[Marker], node: &str) -> Vec<String> {
    pending(g, markers)
        .iter()
        .filter(|p| p.node == node)
        .map(ToString::to_string)
        .collect()
}

#[test]
fn scenario_a_move_into_the_files_owner() {
    // req:check.coverage/pending-report@362c927d
    let dir = moved_into_c();
    let g = graph(dir.path());
    let x = g.requirement("c/x").unwrap().version();
    let markers = [marker("g/x", None, "crates/c/t.rs", 8)];
    assert_eq!(
        on(&g, &markers, "c"),
        [
            format!("c/x {x} unbuilt"),
            "g/x obsolete crates/c/t.rs:8".into()
        ]
    );
}

#[test]
fn scenario_a_move_into_the_files_owner_the_target_blocked() {
    // req:check.coverage/pending-report@362c927d
    let dir = moved_into_c();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [c/x]\n---\n",
    );
    let g = graph(d);
    let x = g.requirement("c/x").unwrap().version();
    let markers = [marker("g/x", None, "crates/c/t.rs", 8)];
    assert_eq!(
        on(&g, &markers, "c"),
        [
            format!("c/x {x} unbuilt blocked backlog/q"),
            "g/x obsolete crates/c/t.rs:8 blocked c/x".into()
        ]
    );
}

#[test]
fn a_move_target_on_another_component_still_blocks() {
    // req:check.coverage/pending-report@362c927d
    let dir = moved_into_c();
    let d = dir.path();
    write(d, "specs/v/SPEC.md", &spec("[\"crates/v/**\"]", &["keep"]));
    let g = graph(d);
    let markers = [marker("g/x", None, "crates/v/t.rs", 8)];
    assert_eq!(
        on(&g, &markers, "v")
            .into_iter()
            .filter(|l| l.starts_with("g/x "))
            .collect::<Vec<_>>(),
        ["g/x obsolete crates/v/t.rs:8 blocked c/x"]
    );
}

#[test]
fn a_move_target_that_waits_on_a_need_stays_on_the_line() {
    // req:check.coverage/pending-report@362c927d
    let dir = moved_into_c();
    let d = dir.path();
    write(
        d,
        "specs/c/SPEC.md",
        &spec("[\"crates/c/**\"]", &["x"]).replace("x.\n", "x.\n\n#### Needs\n- e/z\n"),
    );
    write(d, "specs/e/SPEC.md", &spec("[\"crates/e/**\"]", &["z"]));
    let g = graph(d);
    let markers = [marker("g/x", None, "crates/c/t.rs", 8)];
    assert_eq!(
        on(&g, &markers, "c")
            .into_iter()
            .filter(|l| l.starts_with("g/x "))
            .collect::<Vec<_>>(),
        ["g/x obsolete crates/c/t.rs:8 blocked c/x"]
    );
}

// --- wait-lines ---

/// The lines of `lines` that `line` waits on, as `(component, id)`.
fn waited(lines: &[Pending], line: &Pending) -> Vec<(String, String)> {
    waits(lines, line)
        .into_iter()
        .map(|p| (p.node.clone(), p.id.clone()))
        .collect()
}

#[test]
fn scenario_an_obsolete_requirements_other_lines() {
    // req:check.coverage/wait-lines@11db2e3a
    let dir = repo(Some("[]"));
    let d = dir.path();
    write(d, "specs/d/SPEC.md", &spec("[\"crates/d/**\"]", &["keep"]));
    let g = graph(d);
    let markers = [
        marker("b/gone", None, "crates/b/t.rs", 3),
        marker("b/gone", None, "crates/c/t.rs", 8),
        marker("b/gone", None, "crates/d/t.rs", 1),
    ];
    let all = pending(&g, &markers);
    let own = all
        .iter()
        .find(|p| p.id == "b/gone" && p.node == "b")
        .unwrap();
    assert_eq!(
        waited(&all, own),
        [pair("c", "b/gone"), pair("d", "b/gone")]
    );
    let narrowed: Vec<Pending> = all.iter().filter(|p| p.node == "b").cloned().collect();
    assert!(waits(&narrowed, own).is_empty());
}

#[test]
fn scenario_a_need() {
    // req:check.coverage/wait-lines@11db2e3a
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    root(d, None);
    write(
        d,
        "specs/a/SPEC.md",
        &spec("[]", &["y"]).replace("y.\n", "y.\n\n#### Needs\n- b/z\n"),
    );
    write(d, "specs/b/SPEC.md", &spec("[]", &["z"]));
    let g = graph(d);
    let all = pending(&g, &[]);
    let y = all.iter().find(|p| p.id == "a/y").unwrap();
    assert_eq!(waited(&all, y), [pair("b", "b/z")]);
    let z = all.iter().find(|p| p.id == "b/z").unwrap();
    assert!(waits(&all, z).is_empty());
}

#[test]
fn scenario_a_wait_on_the_lines_own_component() {
    // req:check.coverage/wait-lines@11db2e3a
    let dir = moved_into_c();
    let d = dir.path();
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [c/x]\n---\n",
    );
    let g = graph(d);
    let all = pending(&g, &[marker("g/x", None, "crates/c/t.rs", 8)]);
    let gx = all.iter().find(|p| p.id == "g/x").unwrap();
    assert_eq!(gx.node, "c");
    assert_eq!(waited(&all, gx), [pair("c", "c/x")]);
}

#[test]
fn waits_reads_waits_on_in_order_and_skips_ids_no_line_holds() {
    // req:check.coverage/wait-lines@11db2e3a
    let line = |id: &str, node: &str, waits_on: &[&str]| Pending {
        id: id.into(),
        node: node.into(),
        version: None,
        state: clm_coverage::State::Unbuilt,
        cited: vec![],
        markers: vec![],
        priority: clm_model::Priority::P5,
        blocked_by: vec![],
        waits_on: waits_on.iter().map(|s| s.to_string()).collect(),
    };
    let lines = [
        line("a/y", "a", &["c/q", "gone/w", "b/z"]),
        line("b/z", "b", &[]),
        line("c/q", "c", &[]),
        line("c/q", "d", &[]),
    ];
    assert_eq!(
        waited(&lines, &lines[0]),
        [pair("c", "c/q"), pair("d", "c/q"), pair("b", "b/z")]
    );
}
