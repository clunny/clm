//! `status-report`'s moves (`progress-follows-moves`): an applied change's requirement that
//! a later change moved to another node counts as the requirement it moved to.

mod common;

use std::fs;
use std::path::Path;

use clm_coverage::{Marker, change_progress, complete_changes, pending, status_report};
use clm_model::Graph;
use common::{ROOT, current, graph, marker, spec, write};
use tempfile::TempDir;

/// A change `id` with `status`.
fn change(root: &Path, id: &str, status: &str) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
}

/// `change`'s delta for `node` adding `slug`.
fn adds(root: &Path, change: &str, node: &str, slug: &str) {
    write(
        root,
        &format!("changes/{change}/deltas/{node}.md"),
        &format!(
            "---\ncomponent: {node}\n---\n## ADDED Requirements\n\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ),
    );
}

/// `change`'s delta removing node `node`, which held `slug`.
fn removes(root: &Path, change: &str, node: &str, slug: &str) {
    write(
        root,
        &format!("changes/{change}/deltas/{node}.md"),
        &format!(
            "---\ncomponent: {node}\nremove: true\n---\n## REMOVED Requirements\n\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n"
        ),
    );
}

/// `change` moves `<from>/x` to `<to>/x`, as a rename does.
fn moves(root: &Path, change: &str, from: &str, to: &str) {
    removes(root, change, from, "x");
    adds(root, change, to, "x");
}

fn archive(root: &Path, id: &str) {
    fs::create_dir_all(root.join("changes/archive")).unwrap();
    fs::rename(
        root.join(format!("changes/{id}")),
        root.join(format!("changes/archive/{id}")),
    )
    .unwrap();
}

/// The row `clm status` prints for `id`.
fn row(g: &Graph, markers: &[Marker], id: &str) -> Vec<String> {
    status_report(g, markers)
        .changes
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .find(|r| r[0] == id)
        .unwrap()
}

/// Change `20260101-c` adds `g/x`; `20260102-r` renames `g` to `v`.
fn renamed(d: &Path) {
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/v/SPEC.md", &spec("[]", &["x"]));
    change(d, "20260101-c", "applied");
    adds(d, "20260101-c", "g", "x");
    change(d, "20260102-r", "applied");
    moves(d, "20260102-r", "g", "v");
}

#[test]
fn scenario_node_renamed_later_not_yet_built() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    let g = graph(d);
    assert_eq!(
        row(&g, &[], "20260101-c"),
        ["20260101-c", "0/1 requirements"]
    );
    assert!(complete_changes(&g, &[]).is_empty());
}

#[test]
fn scenario_node_renamed_later_built() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    change(d, "20260102-r", "done");
    archive(d, "20260102-r");
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "1/1 requirements", "complete"]
    );
    assert_eq!(complete_changes(&g, &markers), ["20260101-c"]);
}

#[test]
fn scenario_renamed_twice() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/w/SPEC.md", &spec("[]", &["x"]));
    change(d, "20260101-c", "applied");
    adds(d, "20260101-c", "g", "x");
    change(d, "20260102-r", "applied");
    moves(d, "20260102-r", "g", "v");
    change(d, "20260103-s", "applied");
    moves(d, "20260103-s", "v", "w");
    let g = graph(d);
    let markers = [current(&g, "w/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "1/1 requirements", "complete"]
    );
}

#[test]
fn a_split_counts_as_each_node_it_went_to() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/v/SPEC.md", &spec("[]", &["x"]));
    write(d, "specs/w/SPEC.md", &spec("[]", &["x"]));
    change(d, "20260101-c", "implementing");
    adds(d, "20260101-c", "g", "x");
    change(d, "20260102-r", "applied");
    moves(d, "20260102-r", "g", "v");
    adds(d, "20260102-r", "w", "x");
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    let progress = change_progress(&g, &markers);
    assert_eq!((progress[0].current, progress[0].total), (1, 2));
    assert!(!progress[0].complete);
    let markers = [markers[0].clone(), current(&g, "w/x", "t.rs", 2)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "2/2 requirements", "complete"]
    );
}

#[test]
fn a_partial_split_counts_what_moved_and_what_stayed() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/g/SPEC.md", &spec("[]", &["y"]));
    write(d, "specs/v/SPEC.md", &spec("[]", &["x"]));
    change(d, "20260101-c", "applied");
    write(
        d,
        "changes/20260101-c/deltas/g.md",
        "---\ncomponent: g\n---\n## ADDED Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\ny.\n\n#### Scenario: s\n- ok\n",
    );
    // `g` keeps `y` and gives `x` to `v`.
    change(d, "20260102-r", "applied");
    write(
        d,
        "changes/20260102-r/deltas/g.md",
        "---\ncomponent: g\n---\n## REMOVED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n",
    );
    adds(d, "20260102-r", "v", "x");
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "1/2 requirements"]
    );
    let markers = [markers[0].clone(), current(&g, "g/y", "t.rs", 2)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "2/2 requirements", "complete"]
    );
}

#[test]
fn progress_status_and_complete_changes_agree_on_a_rename() {
    // req:check.coverage/change-progress@dac448cb
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    let g = graph(d);
    for (markers, complete) in [(vec![], false), (vec![current(&g, "v/x", "t.rs", 1)], true)] {
        let progress = change_progress(&g, &markers);
        let c = progress.iter().find(|p| p.change == "20260101-c").unwrap();
        assert_eq!(
            (c.current, c.total, c.complete),
            (usize::from(complete), 1, complete)
        );
        let mut expected = vec![
            "20260101-c".to_string(),
            format!("{}/1 requirements", usize::from(complete)),
        ];
        if complete {
            expected.push("complete".into());
        }
        assert_eq!(row(&g, &markers, "20260101-c"), expected);
        let completes = complete_changes(&g, &markers);
        assert_eq!(completes.contains(&"20260101-c".to_string()), complete);
        let marked: Vec<String> = progress
            .into_iter()
            .filter(|p| p.complete)
            .map(|p| p.change)
            .collect();
        assert_eq!(completes, marked);
    }
}

#[test]
fn a_move_that_comes_back_is_followed_once() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec("[]", &["y"]));
    change(d, "20260101-c", "applied");
    adds(d, "20260101-c", "g", "x");
    // `g/x` and `v/x` move to each other, and neither is held: nothing is counted.
    change(d, "20260102-r", "applied");
    moves(d, "20260102-r", "g", "v");
    change(d, "20260103-s", "applied");
    moves(d, "20260103-s", "v", "g");
    let g = graph(d);
    assert_eq!(
        row(&g, &[], "20260101-c"),
        ["20260101-c", "0/0 requirements", "complete"]
    );
}

#[test]
fn only_applied_implementing_or_done_changes_move() {
    // req:check.coverage/status-report@47ccc387
    for status in ["proposed", "approved", "abandoned"] {
        let dir = TempDir::new().unwrap();
        let d = dir.path();
        renamed(d);
        change(d, "20260102-r", status);
        let g = graph(d);
        let markers = [current(&g, "v/x", "t.rs", 1)];
        assert_eq!(
            row(&g, &markers, "20260101-c"),
            ["20260101-c", "0/0 requirements", "complete"],
            "{status}"
        );
    }
}

#[test]
fn an_unapplied_change_follows_no_moves() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    change(d, "20260101-c", "proposed");
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "0/1 requirements"]
    );
}

#[test]
fn an_id_the_spec_holds_is_not_followed_further() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    // `g` came back holding `x`: the move from `g/x` no longer applies.
    write(d, "specs/g/SPEC.md", &spec("[]", &["x"]));
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "0/1 requirements"]
    );
}

#[test]
fn a_modified_requirement_makes_no_move() {
    // req:check.coverage/status-report@47ccc387
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    renamed(d);
    write(
        d,
        "changes/20260102-r/deltas/v.md",
        "---\ncomponent: v\n---\n## MODIFIED Requirements\n\n### Requirement: x <!-- base: 0123456789abcdef -->\nx.\n\n#### Scenario: s\n- ok\n",
    );
    let g = graph(d);
    let markers = [current(&g, "v/x", "t.rs", 1)];
    assert_eq!(
        row(&g, &markers, "20260101-c"),
        ["20260101-c", "0/0 requirements", "complete"]
    );
}

/// Nodes `a` and `a.b`; an applied change moves `a/x` to `a.b/x`; a marker at `t.rs:4`
/// cites `a/x`.
fn moved(d: &Path) -> Graph {
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec("[]", &["keep"]));
    write(d, "specs/a/b/SPEC.md", &spec("[]", &["x"]));
    change(d, "20260101-m", "applied");
    removes(d, "20260101-m", "a", "x");
    write(
        d,
        "changes/20260101-m/deltas/a.b.md",
        "---\ncomponent: a.b\n---\n## ADDED Requirements\n\n### Requirement: x\nx.\n\n#### Scenario: s\n- ok\n",
    );
    graph(d)
}

#[test]
fn moved_requirement_waits_for_the_move() {
    // req:check.coverage/pending-report@0ca7a745
    let dir = TempDir::new().unwrap();
    let g = moved(dir.path());
    let markers = [marker("a/x", None, "t.rs", 4)];
    let lines: Vec<(String, String)> = pending(&g, &markers)
        .iter()
        .filter(|p| p.id.ends_with("/x"))
        .map(|p| (p.node.clone(), p.to_string()))
        .collect();
    let version = g.requirement("a.b/x").unwrap().version();
    assert_eq!(
        lines,
        [
            ("a".into(), "a/x obsolete t.rs:4 blocked a.b/x".into()),
            ("a.b".into(), format!("a.b/x {version} unbuilt")),
        ]
    );
}

#[test]
fn move_landed_marker_left_behind() {
    // req:check.coverage/pending-report@0ca7a745
    let dir = TempDir::new().unwrap();
    let g = moved(dir.path());
    let markers = [
        marker("a/x", None, "t.rs", 4),
        current(&g, "a.b/x", "u.rs", 1),
    ];
    let a: Vec<String> = pending(&g, &markers)
        .iter()
        .filter(|p| p.node == "a" && p.id == "a/x")
        .map(|p| p.to_string())
        .collect();
    assert_eq!(a, ["a/x obsolete t.rs:4"]);
}
