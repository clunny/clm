use std::fs;
use std::path::Path;

use clm_model::Graph;
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");
const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";
const A: &str = "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n\n### Requirement: y\nY.\n\n#### Scenario: s\n- ok\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", A);
    dir
}

fn graph(root: &Path) -> Graph {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn change(root: &Path, id: &str, status: &str, adds: &[&str]) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    let mut delta = String::from("---\ncomponent: a\n---\n## ADDED Requirements\n");
    for slug in adds {
        delta.push_str(&format!(
            "### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n\n"
        ));
    }
    write(root, &format!("changes/{id}/deltas/a.md"), &delta);
}

fn cite_x(root: &Path) {
    let g = graph(root);
    let v = g.requirement("a/x").unwrap().version().to_string();
    write(root, "t.rs", &format!("// {REQ}a/x@{v}\n"));
}

#[test]
fn scenario_one_pending_change() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    cite_x(d);
    let report = clm_check::status(d).unwrap();
    assert_eq!(report.nodes[0], ["a", "active", "1/2"]);
    assert_eq!(report.nodes[1], ["clm", "active", "0/0"]);
    assert_eq!(report.changes.len(), 1);
    assert_eq!(report.changes[0].0, "applied");
    assert_eq!(report.changes[0].1, [["20260101-c", "1/2 requirements"]]);
}

#[test]
fn a_requirement_cited_only_at_an_old_version_is_not_current() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    write(d, "t.rs", &format!("// {REQ}a/x@00000000\n// {REQ}a/y\n"));
    let report = clm_check::status(d).unwrap();
    assert_eq!(report.nodes[0], ["a", "active", "0/2"]);
    assert_eq!(report.changes[0].1, [["20260101-c", "0/2 requirements"]]);
}

#[test]
fn rows_within_a_group_are_sorted_by_id() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260102-b", "applied", &["y"]);
    change(d, "20260101-z", "applied", &["x"]);
    change(d, "20260101-a", "applied", &[]);
    let report = clm_check::status(d).unwrap();
    let ids: Vec<&str> = report.changes[0].1.iter().map(|r| r[0].as_str()).collect();
    assert_eq!(ids, ["20260101-a", "20260101-z", "20260102-b"]);
}

#[test]
fn scenario_complete_change() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    let g = graph(d);
    let cite = |id: &str| format!("// {REQ}{id}@{}\n", g.requirement(id).unwrap().version());
    write(d, "t.rs", &(cite("a/x") + &cite("a/y")));
    let report = clm_check::status(d).unwrap();
    assert_eq!(
        report.changes[0].1,
        [["20260101-c", "2/2 requirements", "complete"]]
    );
}

/// Adds a REMOVED section for `a/<slug>` to the delta `change` wrote.
fn removes(root: &Path, id: &str, slug: &str) {
    let path = root.join(format!("changes/{id}/deltas/a.md"));
    let delta = fs::read_to_string(&path).unwrap();
    fs::write(
        path,
        format!(
            "{delta}## REMOVED Requirements\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n"
        ),
    )
    .unwrap();
}

#[test]
fn scenario_requirement_removed_later() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    write(
        d,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Requirements\n\n### Requirement: x\nX.\n\n#### Scenario: s\n- ok\n",
    );
    cite_x(d);
    for status in ["applied", "implementing"] {
        change(d, "20260101-c", status, &["x", "y"]);
        let report = clm_check::status(d).unwrap();
        assert_eq!(
            report.changes[0].1,
            [["20260101-c", "1/1 requirements", "complete"]],
            "{status}"
        );
    }
}

#[test]
fn scenario_requirement_added_back_later() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &[]);
    removes(d, "20260101-c", "x");
    cite_x(d);
    let report = clm_check::status(d).unwrap();
    assert_eq!(
        report.changes[0].1,
        [["20260101-c", "0/0 requirements", "complete"]]
    );
}

#[test]
fn a_requirement_added_back_cited_at_an_old_version_does_not_hold_the_change_back() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &[]);
    removes(d, "20260101-c", "x");
    write(d, "t.rs", &format!("// {REQ}a/x@00000000\n"));
    let g = graph(d);
    let markers = clm_check::markers_in(
        Path::new("t.rs"),
        &fs::read_to_string(d.join("t.rs")).unwrap(),
    );
    assert_eq!(clm_check::complete_changes(&g, &markers), ["20260101-c"]);
}

#[test]
fn scenario_unapplied_change() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    for status in ["proposed", "approved"] {
        change(d, "20260101-p", status, &["z"]);
        let report = clm_check::status(d).unwrap();
        assert_eq!(
            report.changes[0].1,
            [["20260101-p", "0/1 requirements"]],
            "{status}"
        );
    }
}

#[test]
fn scenario_removed_requirement_still_cited() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x"]);
    removes(d, "20260101-c", "gone");
    let g = graph(d);
    let x = g.requirement("a/x").unwrap().version().to_string();
    write(d, "t.rs", &format!("// {REQ}a/x@{x}\n// {REQ}a/gone\n"));
    let report = clm_check::status(d).unwrap();
    assert_eq!(report.changes[0].1, [["20260101-c", "1/1 requirements"]]);
}

#[test]
fn an_unapplied_change_is_never_complete() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    for status in ["proposed", "approved"] {
        write(
            d,
            "changes/20260101-c/change.md",
            &format!("---\nstatus: {status}\n---\n"),
        );
        write(
            d,
            "changes/20260101-c/deltas/a.md",
            "---\ncomponent: a\n---\n## MODIFIED Requirements\n### Requirement: x <!-- base: 0000000000000000 -->\nX2.\n\n#### Scenario: s\n- ok\n",
        );
        let g = graph(d);
        let x = g.requirement("a/x").unwrap().version().to_string();
        write(d, "t.rs", &format!("// {REQ}a/x@{x}\n"));
        let report = clm_check::status(d).unwrap();
        assert_eq!(
            report.changes[0].1,
            [["20260101-c", "1/1 requirements"]],
            "{status}"
        );
    }
}

#[test]
fn scenario_groups_in_order() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-a", "applied", &["x"]);
    change(d, "20260101-z", "implementing", &["y"]);
    change(d, "20260101-p", "proposed", &[]);
    change(d, "20260101-q", "approved", &[]);
    change(d, "20260101-b", "abandoned", &[]);
    fs::create_dir_all(d.join("changes/archive")).unwrap();
    fs::rename(
        d.join("changes/20260101-b"),
        d.join("changes/archive/20260101-b"),
    )
    .unwrap();
    let report = clm_check::status(d).unwrap();
    let groups: Vec<&str> = report.changes.iter().map(|(s, _)| s.as_str()).collect();
    assert_eq!(groups, ["implementing", "applied", "approved", "proposed"]);
    assert_eq!(report.changes[0].1[0][0], "20260101-z");
    assert_eq!(report.changes[1].1[0][0], "20260101-a");
}

#[test]
fn scenario_nothing_in_flight() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let report = clm_check::status(dir.path()).unwrap();
    assert!(report.changes.is_empty());
    let text = report.to_string();
    assert!(text.ends_with("changes\n  none\n"), "{text}");
    assert!(
        text.starts_with("components\n  a  active  0/2\n  clm  active  0/0\n"),
        "{text}"
    );
}

#[test]
fn rendered_groups_are_indented() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    change(d, "20260101-c", "implementing", &["x", "y"]);
    cite_x(d);
    let text = clm_check::status(d).unwrap().to_string();
    assert!(
        text.ends_with("changes\n  implementing\n    20260101-c  1/2 requirements\n"),
        "{text}"
    );
}

#[test]
fn status_from_files_matches_checkout() {
    // req:check.coverage/status-report@55590ca5
    let dir = repo();
    let d = dir.path();
    cite_x(d);
    let g = graph(d);
    let markers = clm_check::markers_in(
        Path::new("t.rs"),
        &fs::read_to_string(d.join("t.rs")).unwrap(),
    );
    assert_eq!(
        clm_check::status_report(&g, &markers),
        clm_check::status(d).unwrap()
    );
}
