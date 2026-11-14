//! `removed-capabilities`: a capability a live change lists under `removes_capabilities`,
//! and `CAPABILITIES.md` no longer holds, reads as obsolete wherever it is cited.

mod common;

use std::fs;
use std::path::Path;

use clm_coverage::{
    Marker, change_progress, complete_changes, pending, removed, report, status_report,
};
use clm_model::{Graph, Priority};
use common::{ROOT, graph, marker, spec, write};
use tempfile::TempDir;

const KEPT: &str = "## Capabilities\n\n### Capability: kept\nOne can keep.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
const KEPT_AND_GONE: &str = "## Capabilities\n\n### Capability: gone\nOne can go.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n\n### Capability: kept\nOne can keep.\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";

fn change(root: &Path, id: &str, status: &str, removes: &str) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\nremoves_capabilities: [{removes}]\n---\n"),
    );
}

fn repo(capabilities: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "CAPABILITIES.md", capabilities);
    dir
}

fn with_capability_component(d: &Path) {
    write(
        d,
        "specs/capability/SPEC.md",
        &spec("[\"crates/clm/tests/capability/**\"]", &[]),
    );
}

fn lines(graph: &Graph, markers: &[Marker]) -> Vec<String> {
    report(graph, markers)
        .iter()
        .map(ToString::to_string)
        .collect()
}

fn pending_lines(graph: &Graph, markers: &[Marker], node: &str) -> Vec<String> {
    pending(graph, markers)
        .into_iter()
        .filter(|p| p.node == node && p.id == "capability/gone")
        .map(|p| p.to_string())
        .collect()
}

fn cited() -> [Marker; 2] {
    [
        marker("capability/gone", None, "capabilities/web/gone.spec.ts", 5),
        marker(
            "capability/gone",
            None,
            "crates/clm/tests/capability/gone.rs",
            3,
        ),
    ]
}

#[test]
fn scenario_removed_capability_cited() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT);
    let d = dir.path();
    with_capability_component(d);
    change(d, "20260101-x", "applied", "gone");
    let g = graph(d);
    let markers = cited();
    assert_eq!(removed(&g)["capability/gone"], ["20260101-x"]);
    let expected = "capability/gone obsolete capabilities/web/gone.spec.ts:5 crates/clm/tests/capability/gone.rs:3";
    assert_eq!(lines(&g, &markers).last().unwrap(), expected);
    assert_eq!(pending_lines(&g, &markers, "capability"), [expected]);
    assert!(pending_lines(&g, &markers, "clm").is_empty());
}

#[test]
fn removal_is_listed_whatever_the_status_and_gives_every_change() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT);
    let d = dir.path();
    change(d, "20260101-b", "proposed", "gone");
    change(d, "20260101-a", "implementing", "gone");
    let g = graph(d);
    assert_eq!(removed(&g)["capability/gone"], ["20260101-a", "20260101-b"]);
}

#[test]
fn the_priority_is_the_highest_of_the_changes_listing_it() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT);
    let d = dir.path();
    for (id, p) in [
        ("20260101-a", "P3"),
        ("20260101-b", "P1"),
        ("20260101-c", "P4"),
    ] {
        write(
            d,
            &format!("changes/{id}/change.md"),
            &format!("---\nstatus: applied\npriority: {p}\nremoves_capabilities: [gone]\n---\n"),
        );
    }
    let g = graph(d);
    let found: Vec<_> = pending(&g, &cited())
        .into_iter()
        .filter(|p| p.id == "capability/gone")
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].priority, Priority::P1);
    assert!(found[0].blocked_by.is_empty() && found[0].waits_on.is_empty());
}

#[test]
fn scenario_listed_but_still_held() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT_AND_GONE);
    let d = dir.path();
    change(d, "20260101-x", "proposed", "gone");
    let g = graph(d);
    let version = g
        .requirement("capability/gone")
        .unwrap()
        .version()
        .to_string();
    let markers = [marker("capability/gone", Some(&version), "t.rs", 1)];
    assert!(removed(&g).is_empty());
    let all = lines(&g, &markers);
    assert!(
        all.contains(&"capability/gone current t.rs:1".to_string()),
        "{all:?}"
    );
    assert!(all.iter().all(|l| !l.contains("obsolete")));
}

#[test]
fn scenario_change_archived() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT);
    let d = dir.path();
    with_capability_component(d);
    change(d, "20260101-x", "applied", "gone");
    fs::create_dir_all(d.join("changes/archive")).unwrap();
    fs::rename(
        d.join("changes/20260101-x"),
        d.join("changes/archive/20260101-x"),
    )
    .unwrap();
    let g = graph(d);
    assert!(removed(&g).is_empty());
    assert!(
        pending(&g, &cited())
            .iter()
            .all(|p| p.id != "capability/gone")
    );
}

#[test]
fn scenario_no_capability_component() {
    // req:check.coverage/removed-capabilities@ec1473a4
    let dir = repo(KEPT);
    let d = dir.path();
    write(d, "specs/v/SPEC.md", &spec("[\"crates/v/**\"]", &[]));
    change(d, "20260101-x", "applied", "gone");
    let g = graph(d);
    let markers = [
        marker("capability/gone", None, "crates/v/t.rs", 4),
        marker("capability/gone", None, "old/t.rs", 2),
    ];
    assert_eq!(
        pending_lines(&g, &markers, "clm"),
        ["capability/gone obsolete old/t.rs:2"]
    );
    assert_eq!(
        pending_lines(&g, &markers, "v"),
        ["capability/gone obsolete crates/v/t.rs:4"]
    );
}

#[test]
fn scenario_removed_capability_still_cited_in_progress_and_status() {
    // req:check.coverage/change-progress@e4335c5a
    // req:check.coverage/status-report@ba0eb7f0
    let dir = repo(KEPT);
    let d = dir.path();
    change(d, "20260101-c", "applied", "gone");
    let g = graph(d);
    let markers = [marker("capability/gone", None, "t.rs", 3)];
    let got = change_progress(&g, &markers);
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].current, got[0].total), (0, 0));
    assert_eq!(got[0].obsolete, ["t.rs:3"]);
    assert!(!got[0].complete);
    let row = |markers: &[Marker]| status_report(&g, markers).changes[0].1[0].clone();
    assert_eq!(row(&markers), ["20260101-c", "0/0 requirements"]);
    assert_eq!(row(&[]), ["20260101-c", "0/0 requirements", "complete"]);
    assert!(complete_changes(&g, &markers).is_empty());
    assert_eq!(complete_changes(&g, &[]), ["20260101-c"]);
}
