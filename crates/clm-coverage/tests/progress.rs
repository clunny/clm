mod common;

use clm_coverage::{ChangeProgress, change_progress, complete_changes, status_report};
use common::{ROOT, change, current, graph, marker, spec, write};
use tempfile::TempDir;

fn progress(
    change: &str,
    current: usize,
    total: usize,
    obsolete: &[&str],
    complete: bool,
) -> ChangeProgress {
    ChangeProgress {
        change: change.into(),
        current,
        total,
        obsolete: obsolete.iter().map(|s| s.to_string()).collect(),
        complete,
    }
}

#[test]
fn scenario_same_counts_as_status() {
    // req:check.coverage/change-progress@dac448cb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec("[]", &["x", "y", "v", "w"]));
    change(d, "20260101-c", "applied", &["x", "y"], &[]);
    change(d, "20260101-d", "applied", &["v", "w"], &[]);
    let g = graph(d);
    let markers = [
        current(&g, "a/x", "t.rs", 1),
        marker("a/y", Some("00000000"), "t.rs", 2),
        current(&g, "a/v", "t.rs", 3),
        current(&g, "a/w", "t.rs", 4),
    ];
    assert_eq!(
        change_progress(&g, &markers),
        [
            progress("20260101-c", 1, 2, &[], false),
            progress("20260101-d", 2, 2, &[], true),
        ]
    );
    let report = status_report(&g, &markers);
    assert_eq!(
        report.changes,
        [(
            "applied".to_string(),
            vec![
                vec!["20260101-c".to_string(), "1/2 requirements".to_string()],
                vec![
                    "20260101-d".to_string(),
                    "2/2 requirements".to_string(),
                    "complete".to_string()
                ],
            ]
        )]
    );
    assert_eq!(complete_changes(&g, &markers), ["20260101-d"]);
}

#[test]
fn scenario_removed_requirement_still_cited() {
    // req:check.coverage/change-progress@dac448cb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec("[]", &["y"]));
    change(d, "20260101-c", "applied", &[], &["x"]);
    let g = graph(d);
    let markers = [marker("a/x", None, "t.rs", 3)];
    assert_eq!(
        change_progress(&g, &markers),
        [progress("20260101-c", 0, 0, &["t.rs:3"], false)]
    );
    assert!(complete_changes(&g, &markers).is_empty());
}

#[test]
fn every_live_change_whatever_its_status_and_none_archived() {
    // req:check.coverage/change-progress@dac448cb
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &spec("[]", &["x"]));
    // Not applied yet: `a/z` isn't in the spec, but it counts, and it's never complete.
    change(d, "20260101-p", "proposed", &["z"], &[]);
    // Not applied either, though what it names is current: its text may still change.
    change(d, "20260101-q", "approved", &["x"], &[]);
    change(d, "20260101-a", "applied", &["x"], &[]);
    write(
        d,
        "changes/archive/20250101-old/change.md",
        "---\nstatus: done\n---\n",
    );
    let g = graph(d);
    let markers = [current(&g, "a/x", "t.rs", 1)];
    assert_eq!(
        change_progress(&g, &markers),
        [
            progress("20260101-a", 1, 1, &[], true),
            progress("20260101-p", 0, 1, &[], false),
            progress("20260101-q", 1, 1, &[], false),
        ]
    );
}
