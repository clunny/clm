//! `clm status`'s text, as `status_text` returns it for a fixture repository's graph and
//! markers and the claims it is given.

mod common;

use std::fs;
use std::path::Path;

use clm_view::ClaimRef;
use common::{claim, status_of};
use tempfile::TempDir;

const REQ: &str = concat!("req", ":");

/// What `clm status` prints for the fixture at `root`, with no claims.
fn status(root: &Path) -> String {
    status_of(root, &[], &[])
}

/// The summary: the lines up to the first blank one.
fn summary(text: &str) -> String {
    text.split("\n\n").next().unwrap().to_string()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

fn node(status: &str, reqs: &[&str]) -> String {
    let mut s = format!("---\ntitle: A\nstatus: {status}\nowners: []\n---\n\n## Requirements\n");
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\n{}\n\n#### Scenario: s\n- s\n",
            r.to_uppercase()
        ));
    }
    s
}

/// Node `a` with `a/x` (cited at its current version) and `a/y` (uncited).
fn base() -> TempDir {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "specs/SPEC.md", ROOT);
    write(dir.path(), "specs/a/SPEC.md", &node("active", &["x", "y"]));
    cite(dir.path(), "t", &["a/x"]);
    dir
}

/// `tests/<file>.rs` citing `ids` at their current versions.
fn cite(root: &Path, file: &str, ids: &[&str]) {
    let (graph, _) = clm_model::load(root);
    let mut text: String = ids
        .iter()
        .map(|id| {
            format!(
                "// {REQ}{id}@{}\n",
                graph.requirement(id).unwrap().version()
            )
        })
        .collect();
    text.push_str("#[test]\nfn t() {}\n");
    write(root, &format!("tests/{file}.rs"), &text);
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
            "### Requirement: {slug}\n{}\n\n#### Scenario: s\n- s\n\n",
            slug.to_uppercase()
        ));
    }
    write(root, &format!("changes/{id}/deltas/a.md"), &delta);
}

fn item(root: &Path, slug: &str, kind: &str) {
    write(
        root,
        &format!("backlog/{slug}.md"),
        &format!("---\ntitle: I\nkind: {kind}\n---\n"),
    );
}

#[test]
fn scenario_groups_in_order() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    change(d, "20260101-a", "applied", &["x"]);
    change(d, "20260101-z", "implementing", &["y"]);
    let text = status(d);
    assert!(
        text.contains(
            "changes\n  implementing\n    20260101-z  0/1 requirements\n  applied\n    20260101-a  1/1 requirements  complete\n"
        ),
        "{text}"
    );
    assert!(
        text.contains("changes 2 live: 1 implementing, 1 applied, 1 complete\n"),
        "{text}"
    );
}

#[test]
fn scenario_summary() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    item(d, "i1", "idea");
    item(d, "i2", "idea");
    let text = status(d);
    assert!(text.starts_with("CLM status\n"), "{text}");
    let summary = summary(&text);
    for line in [
        "  components 2: 2 active, 0 unbuilt; 1/2 requirements current",
        "  changes 1 live: 1 applied",
        "  pending 1 components: 1 ready",
        "  waiting on you 0",
        "  backlog 2: 2 idea",
        "  claims 0 live",
    ] {
        assert!(
            summary.contains(&format!("\n{line}\n")) || summary.ends_with(line),
            "{line:?} missing from {text}"
        );
    }
    // The sections follow, each after a blank line.
    assert!(
        text.ends_with("\n\ncomponents\n  a    active  1/2\n  clm  active  0/0\n\nchanges\n  applied\n    20260101-c  1/2 requirements\n\nclaims\n  none\n\n"),
        "{text}"
    );
}

#[test]
fn summary_counts_unbuilt_nodes() {
    // req:view/status-report@1507b00c
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    let text = status(d);
    assert!(
        text.contains("  components 2: 1 active, 1 unbuilt; 0/1 requirements current"),
        "{text}"
    );
}

#[test]
fn scenario_aligned_columns() {
    // req:view/status-report@1507b00c
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    let ten: Vec<String> = (0..10).map(|i| format!("r{i}")).collect();
    let ten: Vec<&str> = ten.iter().map(String::as_str).collect();
    write(d, "specs/a/SPEC.md", &node("active", &ten));
    write(d, "specs/check/SPEC.md", &node("specd", &["x", "y"]));
    let text = status(d);
    assert!(
        text.contains(
            "\ncomponents\n  a      active  0/10\n  check  specd   0/2\n  clm    active  0/0\n\n"
        ),
        "{text}"
    );
}

#[test]
fn scenario_oversized_node() {
    // req:view/status-report@1507b00c
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    for id in ["a", "b", "c"] {
        write(d, &format!("specs/{id}/SPEC.md"), &node("active", &["x"]));
    }
    let (graph, markers) = common::read(d);
    let size: std::collections::BTreeSet<String> = ["a".to_string()].into();
    let text = clm_view::status_text(&graph, &markers, &[], &[], common::NOW, Some(&size), None);
    let summary = summary(&text);
    let line = summary
        .lines()
        .find(|l| l.starts_with("  components "))
        .unwrap();
    assert!(line.ends_with(", 1 oversized"), "{text}");
    // Whether `id`'s row in the `components` section holds the cell `oversized`.
    let marked = |id: &str| {
        let prefix = format!("  {id} ");
        text.lines()
            .skip_while(|l| *l != "components")
            .find(|l| l.starts_with(&prefix))
            .unwrap()
            .split("  ")
            .any(|c| c.trim() == "oversized")
    };
    assert!(marked("a"), "{text}");
    assert!(!marked("b") && !marked("c"), "{text}");
}

#[test]
fn scenario_changes_grouped() {
    // req:view/status-report@1507b00c
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("active", &["x", "y", "z"]));
    change(d, "20260101-c", "implementing", &["x", "y", "z"]);
    cite(d, "t", &["a/x"]);
    let text = status(d);
    assert!(
        text.contains("\nchanges\n  implementing\n    20260101-c  1/3 requirements\n\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  changes 1 live: 1 implementing\n"),
        "{text}"
    );
}

#[test]
fn scenario_complete_change() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    change(d, "20260101-c", "applied", &["x", "y"]);
    cite(d, "u", &["a/y"]);
    let text = status(d);
    assert!(
        text.contains("\n  applied\n    20260101-c  2/2 requirements  complete\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  changes 1 live: 1 applied, 1 complete\n"),
        "{text}"
    );
}

#[test]
fn scenario_waiting_on_you() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    item(d, "q", "question");
    item(d, "h", "human-task");
    item(d, "i", "idea");
    let text = status(d);
    assert!(
        text.contains("\n  waiting on you 2: backlog/h, backlog/q\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  backlog 3: 1 idea, 1 question, 1 human-task\n"),
        "{text}"
    );
}

#[test]
fn a_change_awaiting_approval_waits_on_you_and_a_node_writing_a_protected_path_is_ready() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    write(
        d,
        "specs/i/SPEC.md",
        &(node("specd", &[]).replace("owners: []", "owners: []\npaths: [\".claude/**\"]")
            + "\n### Requirement: w\nWrites `.claude/settings.json`.\n\n#### Scenario: s\n- s\n"),
    );
    change(d, "20260101-p", "proposed", &["z"]);
    change(d, "20260101-q", "approved", &["w"]);
    let text = status(d);
    assert!(
        text.contains("\n  waiting on you 2: changes/20260101-p, changes/20260101-q\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  pending 2 components: 2 ready\n"),
        "{text}"
    );
}

#[test]
fn scenario_claims_counted() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/d/SPEC.md", &node("specd", &["x"]));
    let text = status_of(d, &[claim("b", 1), claim("d", 5)], &[]);
    assert!(text.contains("\n  claims 1 live, 1 lapsed\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b  live\n  d  lapsed\n\n"),
        "{text}"
    );
    // The lapsed claim leaves `d` ready; the live one makes `b` claimed.
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 2 ready\n"),
        "{text}"
    );
}

#[test]
fn backlog_claims_are_counted_with_node_claims() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    let text = status_of(d, &[claim("b", 1)], &[claim("q", 1), claim("r", 5)]);
    assert!(text.contains("\n  claims 2 live, 1 lapsed\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b          live\n  backlog/q  live\n  backlog/r  lapsed\n\n"),
        "{text}"
    );
}

#[test]
fn merged_claims_are_counted_and_released() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/e/SPEC.md", &node("specd", &["x"]));
    // `origin/main` holds `impl/e`'s tip, as after its land.
    let merged = ClaimRef {
        merged: true,
        ..claim("e", 1)
    };
    let text = status_of(d, &[claim("b", 1), merged], &[]);
    assert!(text.contains("\n  claims 1 live, 1 merged\n"), "{text}");
    assert!(
        text.ends_with("\nclaims\n  b  live\n  e  merged\n\n"),
        "{text}"
    );
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 2 ready\n"),
        "{text}"
    );
}

#[test]
fn scenario_pending_states() {
    // req:view/status-report@1507b00c
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(d, "specs/x/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [x/x]\n---\n",
    );
    let text = status_of(d, &[claim("b", 1)], &[]);
    assert!(
        text.contains("\n  pending 3 components: 1 claimed, 1 ready, 1 blocked\n"),
        "{text}"
    );
    assert!(!summary(&text).contains("waiting\n"), "{text}");
    assert!(!summary(&text).contains("waiting,"), "{text}");
}

#[test]
fn summary_orders_statuses_states_and_kinds() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    // Nodes in every pending state: `a` and `i` ready (`i` writes a protected path), `b`
    // claimed, `w` ready though it depends on `a` (`needs-alone-order`), `x` blocked by
    // `backlog/q`.
    write(d, "specs/b/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "specs/i/SPEC.md",
        &(node("specd", &[]).replace("owners: []", "owners: []\npaths: [\".claude/**\"]")
            + "\n### Requirement: w\nWrites `.claude/settings.json`.\n\n#### Scenario: s\n- s\n"),
    );
    write(
        d,
        "specs/w/SPEC.md",
        &node("specd", &["x"]).replace("owners: []", "owners: []\ndepends_on: [a]"),
    );
    write(d, "specs/x/SPEC.md", &node("specd", &["x"]));
    write(
        d,
        "backlog/q.md",
        "---\ntitle: Q\nkind: question\nblocks: [x/x]\n---\n",
    );
    for (slug, kind) in [
        ("h", "human-task"),
        ("i", "idea"),
        ("f", "deferred"),
        ("g", "bug"),
    ] {
        item(d, slug, kind);
    }
    for (id, status) in [
        ("20260101-p", "proposed"),
        ("20260101-a", "approved"),
        ("20260101-i", "implementing"),
        ("20260101-d", "applied"),
    ] {
        change(d, id, status, &["y"]);
    }
    let text = status_of(d, &[claim("b", 1)], &[]);
    for line in [
        "  changes 4 live: 1 implementing, 1 applied, 1 approved, 1 proposed\n",
        "  pending 5 components: 1 claimed, 3 ready, 1 blocked\n",
        "  waiting on you 4: backlog/h, backlog/q, changes/20260101-a, changes/20260101-p\n",
        "  backlog 5: 1 bug, 1 deferred, 1 idea, 1 question, 1 human-task\n",
    ] {
        assert!(text.contains(line), "{line:?} missing from {text}");
    }
}

#[test]
fn scenario_nothing_in_flight() {
    // req:view/status-report@1507b00c
    let dir = base();
    let d = dir.path();
    cite(d, "u", &["a/y"]);
    write(
        d,
        "changes/archive/20250101-old/change.md",
        "---\nstatus: done\n---\n",
    );
    let text = status(d);
    for line in [
        "  changes 0 live\n",
        "  pending 0 components\n",
        "  backlog 0\n",
        "  claims 0 live\n",
    ] {
        assert!(text.contains(line), "{line:?} missing from {text}");
    }
    assert!(
        text.ends_with("\n\nchanges\n  none\n\nclaims\n  none\n\n"),
        "{text}"
    );
}
