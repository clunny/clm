//! A change's recorded moves (`move-records`).

use std::fs;
use std::path::Path;

use clm_model::{Move, load, parse_change_frontmatter};
use tempfile::TempDir;

/// The delta for `a` that removes `old` and `gone` and adds `new`.
const DELTA_A: &str = "---\ncomponent: a\n---\n## ADDED Requirements\n### Requirement: new\nThe a SHALL x.\n#### Scenario: s\n- y\n\n## REMOVED Requirements\n### Requirement: old <!-- base: 0123456789abcdef -->\n### Requirement: gone <!-- base: 0123456789abcdef -->\n";

/// The delta for `b` that modifies `kept`.
const DELTA_B: &str = "---\ncomponent: b\n---\n## MODIFIED Requirements\n### Requirement: kept <!-- base: 0123456789abcdef -->\nThe b SHALL x.\n#### Scenario: s\n- y\n";

fn write(dir: &TempDir, rel: &str, text: &str) {
    let path = dir.path().join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A repo with change `x` at `changes/x` (or `at`) whose `change.md` sets `moves`, and the
/// deltas given.
fn repo(at: &str, moves: &str, deltas: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().unwrap();
    write(
        &dir,
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: active\nowners: [\"@a\"]\n---\n\n## Contract\nc\n",
    );
    write(
        &dir,
        &format!("{at}/change.md"),
        &format!("---\nstatus: proposed\n{moves}---\n"),
    );
    for (name, text) in deltas {
        write(&dir, &format!("{at}/deltas/{name}"), text);
    }
    dir
}

fn mv(requirement: &str, to: &str) -> Move {
    Move {
        requirement: requirement.into(),
        to: to.into(),
    }
}

#[test]
fn a_merge_recorded() {
    // req:model/move-records@acfffee2
    let dir = repo(
        "changes/x",
        "moves:\n  - requirement: a/old\n    to: a/new\n  - requirement: a/gone\n    to: b/kept\n",
        &[("a.md", DELTA_A), ("b.md", DELTA_B)],
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.changes["x"].moves,
        [mv("a/old", "a/new"), mv("a/gone", "b/kept")]
    );
}

#[test]
fn a_change_with_no_moves_holds_none() {
    // req:model/move-records@acfffee2
    let dir = repo("changes/x", "", &[("a.md", DELTA_A)]);
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.changes["x"].moves.is_empty());
}

#[test]
fn a_move_the_deltas_dont_make() {
    // req:model/move-records@acfffee2
    let dir = repo(
        "changes/x",
        "moves:\n  - requirement: a/kept\n    to: a/new\n  - requirement: a/old\n    to: a/new\n  - requirement: a/gone\n    to: a/other\n  - requirement: a/old\n    to: a/new\n  - requirement: a/gone\n    to: new\n    why: merged\n",
        &[("a.md", DELTA_A)],
    );
    let (graph, violations) = load(dir.path());
    assert!(!graph.changes.contains_key("x"));
    let mut entries = Vec::new();
    for v in &violations {
        assert_eq!(v.rule, "model/move-records");
        assert_eq!(v.path, Path::new("changes/x/change.md"));
        let rest = v.message.strip_prefix("entry ").expect(&v.message);
        entries.push(rest.split(':').next().unwrap().to_string());
    }
    assert_eq!(entries, ["1", "3", "4", "5"]);
}

#[test]
fn each_malformed_entry_is_reported() {
    // req:model/move-records@acfffee2
    let dir = repo(
        "changes/x",
        "moves:\n  - requirement: a/old\n  - to: a/new\n  - requirement: A/old\n    to: a/new\n  - requirement: a/old\n    to: a/New\n  - a/old\n  - requirement: [a/old]\n    to: a/new\n  - requirement: a/old\n    to: b/new\n",
        &[("a.md", DELTA_A)],
    );
    let (graph, violations) = load(dir.path());
    assert!(!graph.changes.contains_key("x"));
    let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();
    let entries: Vec<&str> = messages
        .iter()
        .map(|m| m.strip_prefix("entry ").unwrap().split(':').next().unwrap())
        .collect();
    assert_eq!(entries, ["1", "2", "3", "4", "5", "6", "7"], "{messages:?}");
}

#[test]
fn moves_that_arent_a_list() {
    // req:model/move-records@acfffee2
    let dir = repo("changes/x", "moves: sprocket\n", &[]);
    write(
        &dir,
        "changes/archive/y/change.md",
        "---\nstatus: done\nmoves: sprocket\n---\n",
    );
    let (graph, violations) = load(dir.path());
    assert!(!graph.changes.contains_key("x"));
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/move-records");
    assert_eq!(violations[0].path, Path::new("changes/x/change.md"));
    assert!(graph.changes["y"].moves.is_empty());
}

#[test]
fn an_archived_change_loads_its_moves_unchecked() {
    // req:model/move-records@acfffee2
    let dir = repo(
        "changes/archive/y",
        "moves:\n  - requirement: a/old\n    to: a/elsewhere\n  - requirement: a/old\n    to: a/again\n  - sprocket\n  - requirement: a/x\n    to: a/y\n    why: merged\n",
        &[],
    );
    let (graph, violations) = load(dir.path());
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(
        graph.changes["y"].moves,
        [mv("a/old", "a/elsewhere"), mv("a/old", "a/again")]
    );
}

#[test]
fn moves_alone() {
    // req:model/parse-changes@374972bb
    let fm = parse_change_frontmatter("---\nstatus: proposed\nmoves: sprocket\n---\n")
        .expect("moves alone parses");
    assert!(fm.status.is_pending());
    assert_eq!(fm.status, clm_model::ChangeStatus::Proposed);
}
