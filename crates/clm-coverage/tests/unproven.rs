//! A requirement a backlog item's `unproven` names at its current version reads `unbuilt`
//! whatever cites it (`coverage-report`).

mod common;

use clm_coverage::report;
use common::{ROOT, current, graph, marker, spec, write};
use tempfile::TempDir;

const ITEM: &str = "---\ntitle: T\nkind: bug\nunproven: [n/x@VERSION]\n---\n";

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/n/SPEC.md", &spec("[\"src/**\"]", &["x"]));
    dir
}

fn line(dir: &TempDir, marker_at: Option<&str>) -> String {
    let graph = graph(dir.path());
    let markers = match marker_at {
        Some(v) => vec![marker("n/x", Some(v), "t.rs", 2)],
        None => vec![current(&graph, "n/x", "t.rs", 2)],
    };
    report(&graph, &markers)
        .into_iter()
        .find(|c| c.id == "n/x")
        .unwrap()
        .to_string()
}

fn version(dir: &TempDir) -> String {
    graph(dir.path())
        .requirement("n/x")
        .unwrap()
        .version()
        .to_string()
}

#[test]
fn named_unproven() {
    // req:check.coverage/coverage-report@72845fef
    let dir = repo();
    let v = version(&dir);
    write(dir.path(), "backlog/u.md", &ITEM.replace("VERSION", &v));
    assert_eq!(line(&dir, None), "n/x unbuilt");
    std::fs::remove_file(dir.path().join("backlog/u.md")).unwrap();
    assert_eq!(line(&dir, None), "n/x current t.rs:2");
}

#[test]
fn unproven_at_an_older_version() {
    // req:check.coverage/coverage-report@72845fef
    let dir = repo();
    write(
        dir.path(),
        "backlog/u.md",
        &ITEM.replace("VERSION", "aaaaaaaa"),
    );
    assert_ne!(version(&dir), "aaaaaaaa");
    assert_eq!(line(&dir, None), "n/x current t.rs:2");
}

#[test]
fn unproven_beside_a_stale_marker() {
    // req:check.coverage/coverage-report@72845fef
    let dir = repo();
    let v = version(&dir);
    write(dir.path(), "backlog/u.md", &ITEM.replace("VERSION", &v));
    assert_eq!(line(&dir, Some("bbbbbbbb")), "n/x unbuilt");
}
