//! `ARCHITECTURE.md`: a `## Constraints` section of `### Constraint:` entries
//! (`human-layers`).

use std::path::{Path, PathBuf};

use clm_model::{Files, Graph, Layer, Violation, load_files};

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

fn body(heading: &str, slug: &str) -> String {
    format!(
        "{heading} {slug}\nGit SHALL be the source of truth.\n\n#### Scenario: no server state\n- GIVEN a repo\n- THEN git holds it\n\n"
    )
}

fn load(architecture: Option<&str>) -> (Graph, Vec<Violation>) {
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), ROOT.into());
    if let Some(text) = architecture {
        files.insert(PathBuf::from("ARCHITECTURE.md"), text.as_bytes().to_vec());
    }
    load_files(&files)
}

fn ids(graph: &Graph) -> Vec<&str> {
    graph.layers[&Layer::Architecture]
        .iter()
        .map(|r| r.id.as_str())
        .collect()
}

/// The one `model/human-layers` violation on `ARCHITECTURE.md`, naming each of `names`.
fn reported(violations: &[Violation], names: &[&str]) {
    assert_eq!(violations.len(), 1, "{violations:?}");
    assert_eq!(violations[0].rule, "model/human-layers");
    assert_eq!(violations[0].path, Path::new("ARCHITECTURE.md"));
    for name in names {
        assert!(
            violations[0].message.contains(name),
            "{name}: {violations:?}"
        );
    }
}

#[test]
fn constraints_load() {
    // req:model/human-layers@c4f90bbc
    let text = format!(
        "# Architecture\n\n## Constraints\n\n{}",
        body("### Constraint:", "git-is-truth")
    );
    let (graph, violations) = load(Some(&text));
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(ids(&graph), ["architecture/git-is-truth"]);
    let entry = graph.requirement("architecture/git-is-truth").unwrap();
    let block = body("### Constraint:", "git-is-truth");
    assert_eq!(entry.hash, clm_model::content_hash(&block));
    assert_eq!(entry.version(), &clm_model::content_hash(&block)[..8]);
    assert_eq!(graph.heading(Layer::Architecture), "### Constraint:");
    assert_eq!(graph.heading(Layer::Capability), "### Capability:");
}

#[test]
fn an_absent_file_writes_constraints() {
    // req:model/human-layers@c4f90bbc
    let (graph, violations) = load(None);
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(graph.heading(Layer::Architecture), "### Constraint:");
    assert_eq!(
        Graph::default().heading(Layer::Architecture),
        "### Constraint:"
    );
}

#[test]
fn the_old_requirement_heading_is_prose() {
    // req:model/human-layers@c4f90bbc
    let in_constraints = format!(
        "## Constraints\n\n{}{}",
        body("### Constraint:", "a"),
        body("### Requirement:", "b")
    );
    let (graph, violations) = load(Some(&in_constraints));
    assert!(violations.is_empty(), "{violations:?}");
    assert_eq!(ids(&graph), ["architecture/a"]);

    let no_section = format!("## Requirements\n\n{}", body("### Requirement:", "c"));
    let (graph, violations) = load(Some(&no_section));
    assert!(violations.is_empty(), "{violations:?}");
    assert!(ids(&graph).is_empty());
    assert_eq!(graph.heading(Layer::Architecture), "### Constraint:");
}

#[test]
fn the_other_form_in_a_fence_is_text() {
    // req:model/human-layers@c4f90bbc
    let text = format!(
        "## Constraints\n\n{}```markdown\n### Requirement: shown\n```\n",
        body("### Constraint:", "a")
    );
    let (_, violations) = load(Some(&text));
    assert!(violations.is_empty(), "{violations:?}");
}

#[test]
fn a_constraint_outside_its_section_is_reported() {
    // req:model/human-layers@c4f90bbc
    let text = format!(
        "## Constraints\n\n{}## Elsewhere\n\n{}",
        body("### Constraint:", "a"),
        body("### Constraint:", "lost")
    );
    let (graph, violations) = load(Some(&text));
    reported(&violations, &["`lost`", "Constraints"]);
    assert_eq!(ids(&graph), ["architecture/a"]);
}

#[test]
fn free_form_architecture_has_no_entries() {
    // req:model/human-layers@c4f90bbc
    let text = "# Architecture\n\n## Layer 1 invariants\n\n- **`git-is-truth`**: git holds it.\n";
    let (graph, violations) = load(Some(text));
    assert!(violations.is_empty(), "{violations:?}");
    assert!(ids(&graph).is_empty());
}
