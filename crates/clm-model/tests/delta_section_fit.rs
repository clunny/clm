//! A delta holds only the sections its kind of delta can apply.

use std::fs;
use std::path::Path;

use clm_model::{Graph, Violation, load};
use tempfile::TempDir;

const RULE: &str = "model/delta-section-fit";
const PATH: &str = "changes/c/deltas/d.md";

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Loads a repo with root node `clm`, node `a`, and one change whose only delta is `delta`.
fn load_delta(delta: &str) -> (Graph, Vec<Violation>) {
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n",
    );
    write(
        dir.path(),
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n",
    );
    write(
        dir.path(),
        "changes/c/change.md",
        "---\nstatus: proposed\n---\n",
    );
    write(dir.path(), PATH, delta);
    load(dir.path())
}

fn fits(violations: &[Violation]) -> Vec<&Violation> {
    let fit: Vec<_> = violations.iter().filter(|v| v.rule == RULE).collect();
    for v in &fit {
        assert_eq!(v.path, Path::new(PATH));
    }
    fit
}

fn not_loaded(graph: &Graph) {
    assert!(graph.changes["c"].deltas.is_empty());
}

const CREATE_FM: &str = "## Frontmatter\n```yaml\ntitle: B\nstatus: specd\nowners: []\n```\n";

#[test]
fn stray_contract_under_modified_contract() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\ncomponent: a\n---\n\n## MODIFIED Contract\n\n## Contract\nThe contract.\n";
    let (graph, violations) = load_delta(delta);
    assert_eq!(violations.len(), 2, "{violations:?}");
    let fit = fits(&violations);
    assert_eq!(fit.len(), 2, "{violations:?}");
    let m = &fit[0].message;
    assert!(m.contains("`## MODIFIED Contract`"), "{m}");
    assert!(m.contains("line 5"), "{m}");
    assert!(m.contains("empty"), "{m}");
    let m = &fit[1].message;
    assert!(m.contains("`## Contract`"), "{m}");
    assert!(m.contains("line 7"), "{m}");
    assert!(m.contains("create: true"), "{m}");
    assert!(m.contains("## MODIFIED Contract"), "{m}");
    not_loaded(&graph);
}

#[test]
fn design_without_create() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\ncomponent: a\n---\n## Design\nThe design.\n";
    let (graph, violations) = load_delta(delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let m = &fits(&violations)[0].message;
    assert!(m.contains("`## Design`"), "{m}");
    assert!(m.contains("line 4"), "{m}");
    assert!(m.contains("## MODIFIED Design"), "{m}");
    not_loaded(&graph);
}

#[test]
fn frontmatter_without_create() {
    // req:model/delta-section-fit@c678a1c3
    let delta = format!("---\ncomponent: a\n---\n{CREATE_FM}");
    let (graph, violations) = load_delta(&delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let m = &fits(&violations)[0].message;
    assert!(m.contains("`## Frontmatter`"), "{m}");
    assert!(m.contains("line 4"), "{m}");
    assert!(m.contains("## MODIFIED Frontmatter"), "{m}");
    not_loaded(&graph);
}

#[test]
fn edit_section_in_a_create_delta() {
    // req:model/delta-section-fit@c678a1c3
    let delta = format!(
        "---\ncomponent: b\ncreate: true\n---\n{CREATE_FM}\n## Contract\nc\n\n## MODIFIED Requirements\n### Requirement: x <!-- base: 0123456789abcdef -->\nt\n#### Scenario: s\n- y\n"
    );
    let (graph, violations) = load_delta(&delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let m = &fits(&violations)[0].message;
    assert!(m.contains("`## MODIFIED Requirements`"), "{m}");
    assert!(m.contains("line 15"), "{m}");
    assert!(m.contains("writes its component whole"), "{m}");
    not_loaded(&graph);
}

#[test]
fn every_edit_section_in_a_create_delta() {
    // req:model/delta-section-fit@c678a1c3
    for section in [
        "MODIFIED Frontmatter",
        "MODIFIED Contract",
        "MODIFIED Design",
        "MODIFIED Requirements",
        "REMOVED Requirements",
        "REMOVED Assets",
    ] {
        // Empty too, but reported once, as edit-only.
        let delta = format!("---\ncomponent: b\ncreate: true\n---\n{CREATE_FM}## {section}\n");
        let (graph, violations) = load_delta(&delta);
        assert_eq!(violations.len(), 1, "{section}: {violations:?}");
        let m = &fits(&violations)[0].message;
        assert!(m.contains(&format!("`## {section}`")), "{m}");
        assert!(m.contains("writes its component whole"), "{m}");
        not_loaded(&graph);
    }
}

#[test]
fn misfit_removed_assets_judged_no_further() {
    // req:model/delta-section-fit@c678a1c3
    let delta = format!(
        "---\ncomponent: b\ncreate: true\n---\n{CREATE_FM}## REMOVED Assets\n- not an entry\n"
    );
    let (graph, violations) = load_delta(&delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, RULE);
    not_loaded(&graph);
}

#[test]
fn stray_frontmatter_without_a_fence() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\ncomponent: a\n---\n## Frontmatter\ntitle: B\n";
    let (graph, violations) = load_delta(delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let m = &fits(&violations)[0].message;
    assert!(m.contains("`## Frontmatter`"), "{m}");
    not_loaded(&graph);
}

#[test]
fn edit_section_with_stray_text_in_a_create_delta() {
    // req:model/delta-section-fit@c678a1c3
    let delta = format!(
        "---\ncomponent: b\ncreate: true\n---\n{CREATE_FM}## MODIFIED Requirements\nstray\n### Requirement: x <!-- base: 0123456789abcdef -->\nt\n#### Scenario: s\n- y\n"
    );
    let (graph, violations) = load_delta(&delta);
    assert_eq!(violations.len(), 2, "{violations:?}");
    let fit = fits(&violations);
    assert_eq!(fit.len(), 1, "{violations:?}");
    assert!(fit[0].message.contains("`## MODIFIED Requirements`"));
    let under = violations
        .iter()
        .find(|v| v.rule == "model/requirement-text-under-a-heading")
        .expect("requirement-text-under-a-heading");
    assert!(under.message.contains("MODIFIED Requirements"), "{under:?}");
    not_loaded(&graph);
}

#[test]
fn blank_modified_design() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\ncomponent: a\n---\n## MODIFIED Design\n\n  \n\t\n## ADDED Requirements\n### Requirement: x\nt\n#### Scenario: s\n- y\n";
    let (graph, violations) = load_delta(delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    let m = &fits(&violations)[0].message;
    assert!(m.contains("`## MODIFIED Design`"), "{m}");
    assert!(m.contains("line 4"), "{m}");
    assert!(m.contains("empty"), "{m}");
    not_loaded(&graph);
}

#[test]
fn comment_or_fence_is_not_empty() {
    // req:model/delta-section-fit@c678a1c3
    for body in ["<!-- todo -->\n", "```\n```\n"] {
        let delta = format!("---\ncomponent: a\n---\n## MODIFIED Contract\n{body}");
        let (graph, violations) = load_delta(&delta);
        assert!(violations.is_empty(), "{violations:?}");
        assert!(graph.changes["c"].deltas[0].modified_contract.is_some());
    }
}

#[test]
fn line_numbers_with_crlf() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\r\ncomponent: a\r\n---\r\n\r\n## Design\r\nd\r\n";
    let (_, violations) = load_delta(delta);
    let m = &fits(&violations)[0].message;
    assert!(m.contains("line 5"), "{m}");
}

#[test]
fn well_formed_deltas() {
    // req:model/delta-section-fit@c678a1c3
    let (graph, violations) =
        load_delta("---\ncomponent: a\n---\n## MODIFIED Contract\nThe new contract.\n");
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["c"].deltas.len(), 1);
    let delta = format!(
        "---\ncomponent: b\ncreate: true\n---\n{CREATE_FM}## Contract\nc\n## Design\nd\n## ADDED Requirements\n### Requirement: x\nt\n#### Scenario: s\n- y\n"
    );
    let (graph, violations) = load_delta(&delta);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.changes["c"].deltas.len(), 1);
}

#[test]
fn removal_delta_judged_once() {
    // req:model/delta-section-fit@c678a1c3
    let delta = "---\ncomponent: a\nremove: true\n---\n## Contract\nc\n";
    let (graph, violations) = load_delta(delta);
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/parse-changes");
    assert!(violations[0].message.contains("## Contract"));
    not_loaded(&graph);
}

#[test]
fn archived_deltas_too() {
    // req:model/delta-section-fit@c678a1c3
    let dir = TempDir::new().unwrap();
    write(
        dir.path(),
        "specs/SPEC.md",
        "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n",
    );
    write(
        dir.path(),
        "changes/archive/c/change.md",
        "---\nstatus: done\n---\n",
    );
    write(
        dir.path(),
        "changes/archive/c/deltas/d.md",
        "---\ncomponent: clm\n---\n## Design\nd\n",
    );
    let (_, violations) = load(dir.path());
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, RULE);
}
