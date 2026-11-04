//! A live delta holds no text before its first section, and no `### Requirement:` heading in a
//! section that isn't a requirements section.

use std::fs;
use std::path::Path;

use clm_model::{Violation, load};
use tempfile::TempDir;

const RULE: &str = "model/misplaced-delta-text";
const DELTA: &str = "changes/c/deltas/clm.md";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A repo with a root node `clm` and one change, `c` (under `changes/archive/` with
/// `archived`), whose only delta is `delta`.
fn load_delta_in(delta: &str, archived: bool) -> (clm_model::Graph, Vec<Violation>) {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (change, status) = match archived {
        true => ("changes/archive/c", "done"),
        false => ("changes/c", "proposed"),
    };
    write(
        dir.path(),
        &format!("{change}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    write(dir.path(), &format!("{change}/deltas/clm.md"), delta);
    load(dir.path())
}

fn load_delta(delta: &str) -> (clm_model::Graph, Vec<Violation>) {
    load_delta_in(delta, false)
}

/// This rule's violations, each checked to name the delta's file.
fn misplaced(violations: &[Violation]) -> Vec<&Violation> {
    let found: Vec<_> = violations.iter().filter(|v| v.rule == RULE).collect();
    for v in &found {
        assert_eq!(v.path, Path::new(DELTA), "{v:?}");
    }
    found
}

const ADDS_X: &str =
    "## ADDED Requirements\n### Requirement: x\nThe model SHALL x.\n\n#### Scenario: s\n- y\n";

#[test]
fn requirement_under_a_design_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = "---\ncomponent: clm\n---\n## MODIFIED Design\nA paragraph.\n\n### Requirement: x\nThe model SHALL x.\n\n#### Scenario: s\n- y\n";
    let (graph, violations) = load_delta(delta);
    let found = misplaced(&violations);
    assert_eq!(found.len(), 1, "{violations:?}");
    assert!(found[0].message.contains("MODIFIED Design"), "{found:?}");
    assert!(found[0].message.contains("line 7"), "{found:?}");
    assert!(graph.changes["c"].deltas.is_empty());
}

#[test]
fn requirement_under_a_contract_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = "---\ncomponent: a\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: A\nstatus: specd\nowners: []\n```\n\n## Contract\n### Requirement: x\n\n## Design\nd\n### Requirement: y\n";
    let (graph, violations) = load_delta(delta);
    let found = misplaced(&violations);
    assert!(graph.changes["c"].deltas.is_empty());
    assert_eq!(found.len(), 2, "{violations:?}");
    assert!(found[0].message.contains("`## Contract`"), "{found:?}");
    assert!(found[0].message.contains("line 13"), "{found:?}");
    assert!(found[1].message.contains("`## Design`"), "{found:?}");
    assert!(found[1].message.contains("line 17"), "{found:?}");
}

#[test]
fn two_headings_in_one_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = "---\ncomponent: clm\n---\n## MODIFIED Design\n### Requirement: x\nt\n### Requirement: y\nu\n";
    let found_in = load_delta(delta).1;
    let found = misplaced(&found_in);
    assert_eq!(found.len(), 1, "{found_in:?}");
    assert!(found[0].message.contains("MODIFIED Design"), "{found:?}");
    assert!(found[0].message.contains("line 5"), "{found:?}");
}

#[test]
fn text_before_the_first_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = format!("---\ncomponent: clm\n---\n\nA rationale paragraph.\n\n{ADDS_X}");
    let (graph, violations) = load_delta(&delta);
    let found = misplaced(&violations);
    assert_eq!(found.len(), 1, "{violations:?}");
    assert!(found[0].message.contains("line 5"), "{found:?}");
    assert!(graph.changes["c"].deltas.is_empty());
}

#[test]
fn every_non_blank_line_before_the_first_section_counts() {
    // req:model/misplaced-delta-text@4bb4e87b
    for before in ["<!-- note -->\n", "```\n## Not a section\n```\n"] {
        let delta = format!("---\ncomponent: clm\n---\n{before}{ADDS_X}");
        let violations = load_delta(&delta).1;
        let found = misplaced(&violations);
        assert_eq!(found.len(), 1, "{before:?}: {violations:?}");
        assert!(found[0].message.contains("line 4"), "{found:?}");
    }
    // No section at all.
    let violations = load_delta("---\ncomponent: clm\n---\nJust text.\n").1;
    let found = misplaced(&violations);
    assert_eq!(found.len(), 1, "{violations:?}");
    assert!(found[0].message.contains("line 4"), "{found:?}");
}

#[test]
fn heading_in_a_code_fence() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta =
        "---\ncomponent: clm\n---\n## MODIFIED Design\nd\n```markdown\n### Requirement: x\n```\n";
    let (graph, violations) = load_delta(delta);
    assert!(violations.is_empty(), "{violations:?}");
    let design = graph.changes["c"].deltas[0]
        .modified_design
        .as_deref()
        .unwrap();
    assert!(design.contains("```markdown"), "{design}");
}

#[test]
fn blank_lines_before_the_first_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = format!("---\ncomponent: clm\n---\n\n\n{ADDS_X}");
    let (graph, violations) = load_delta(&delta);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["c"].deltas[0].added[0].slug, "x");
}

#[test]
fn reported_beside_the_other_rules() {
    // req:model/misplaced-delta-text@4bb4e87b
    // A section another rule rejects is still judged by this one.
    let delta = "---\ncomponent: clm\nremove: true\n---\n## MODIFIED Design\n### Requirement: x\n";
    let violations = load_delta(delta).1;
    assert_eq!(misplaced(&violations).len(), 1, "{violations:?}");
    assert!(
        violations.iter().any(|v| v.rule == "model/parse-changes"),
        "{violations:?}"
    );
}

#[test]
fn heading_in_frontmatter_rationale_and_an_unknown_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta = "---\ncomponent: clm\n---\n## MODIFIED Frontmatter\nWhy.\n### Requirement: x\n```yaml\ntitle: T\n```\n\n## Odd\n### Requirement: y\n";
    let violations = load_delta(delta).1;
    let found = misplaced(&violations);
    assert_eq!(found.len(), 2, "{violations:?}");
    assert!(
        found[0].message.contains("MODIFIED Frontmatter"),
        "{found:?}"
    );
    assert!(found[1].message.contains("`## Odd`"), "{found:?}");
    // The unknown section is `parse-changes`' too.
    assert!(
        violations.iter().any(|v| v.rule == "model/parse-changes"),
        "{violations:?}"
    );
}

#[test]
fn archived_delta_with_text_before_its_first_section() {
    // req:model/misplaced-delta-text@4bb4e87b
    let delta =
        "---\ncomponent: clm\n---\nA paragraph.\n\n## MODIFIED Design\nd\n### Requirement: x\n";
    let (graph, violations) = load_delta_in(delta, true);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(graph.changes["c"].archived);
    assert_eq!(graph.changes["c"].deltas.len(), 1);
}
