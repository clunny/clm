//! Requirement text in a delta sits under a `### Requirement:` heading.

use std::fs;
use std::path::Path;

use clm_model::{Violation, load};
use tempfile::TempDir;

const RULE: &str = "model/requirement-text-under-a-heading";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Loads a repo with a root node and one change whose only delta is `delta`.
fn load_delta(delta: &str) -> (clm_model::Graph, Vec<Violation>) {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n",
    );
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(dir.path(), "changes/c/deltas/clm.md", delta);
    load(dir.path())
}

fn only(violations: &[Violation]) -> &Violation {
    assert_eq!(violations.len(), 1, "{violations:?}");
    let v = &violations[0];
    assert_eq!(v.rule, RULE);
    assert_eq!(v.path, Path::new("changes/c/deltas/clm.md"));
    v
}

#[test]
fn modified_text_with_no_heading() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta = "---\ncomponent: clm\n---\n\n## MODIFIED Requirements\n\nThe model SHALL x.\n\n#### Scenario: s\n- y\n";
    let (graph, violations) = load_delta(delta);
    let v = only(&violations);
    assert!(v.message.contains("MODIFIED Requirements"), "{v:?}");
    assert!(v.message.contains("line 7"), "{v:?}");
    assert!(graph.changes["c"].deltas.is_empty());
}

#[test]
fn added_text_before_the_first_heading() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta = "---\ncomponent: clm\n---\n## ADDED Requirements\nstray\n### Requirement: x\nt\n#### Scenario: s\n- y\n";
    let (graph, violations) = load_delta(delta);
    let v = only(&violations);
    assert!(v.message.contains("ADDED Requirements"), "{v:?}");
    assert!(v.message.contains("line 5"), "{v:?}");
    assert!(graph.changes["c"].deltas.is_empty());
}

#[test]
fn every_non_blank_line_counts() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    for (before, line) in [
        ("<!-- note -->\n", 5),
        ("```\n### Requirement: hidden\n```\n", 5),
    ] {
        let delta = format!(
            "---\ncomponent: clm\n---\n## ADDED Requirements\n{before}### Requirement: x\nt\n#### Scenario: s\n- y\n"
        );
        let (_, violations) = load_delta(&delta);
        let v = only(&violations);
        assert!(v.message.contains(&format!("line {line}")), "{v:?}");
    }
    // A fenced heading isn't one: text with no real heading.
    let delta =
        "---\ncomponent: clm\n---\n## MODIFIED Requirements\n\n```\n### Requirement: x\n```\n";
    let v = only(&load_delta(delta).1).message.clone();
    assert!(v.contains("line 6"), "{v}");
}

#[test]
fn one_violation_per_section() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta =
        "---\ncomponent: clm\n---\n## ADDED Requirements\na\nb\n\n## MODIFIED Requirements\nc\n";
    let (_, violations) = load_delta(delta);
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(violations[0].message.contains("ADDED Requirements"));
    assert!(violations[0].message.contains("line 5"));
    assert!(violations[1].message.contains("MODIFIED Requirements"));
    assert!(violations[1].message.contains("line 9"));
}

#[test]
fn line_numbers_with_crlf() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta = "---\r\ncomponent: clm\r\n---\r\n## ADDED Requirements\r\n\r\nstray\r\n";
    let v = only(&load_delta(delta).1).message.clone();
    assert!(v.contains("line 6"), "{v}");
}

#[test]
fn removal_rationale() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta = "---\ncomponent: clm\n---\n## REMOVED Requirements\nMoved to `check`.\n\n### Requirement: x <!-- base: 0123456789abcdef -->\n";
    let (graph, violations) = load_delta(delta);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["c"].deltas[0].removed[0].slug, "x");
}

#[test]
fn blank_lines() {
    // req:model/requirement-text-under-a-heading@e3d63eda
    let delta = "---\ncomponent: clm\n---\n## MODIFIED Requirements\n\n  \n### Requirement: x <!-- base: 0123456789abcdef -->\nt\n#### Scenario: s\n- y\n";
    let (graph, violations) = load_delta(delta);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["c"].deltas[0].modified[0].slug, "x");
}
