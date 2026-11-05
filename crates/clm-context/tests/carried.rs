// clm:reads ARCHITECTURE.md
//! CLM's carried constraints in a bundle.

use std::path::PathBuf;

use clm_model::{Files, Graph, load_files};

const SOURCE: &str = "## Constraints\n\n### Constraint: a\nIt SHALL a.\n\n#### Scenario: s\n- ok\n\n#### Carries\n- method\n";
const PROJECT: &str = "## Constraints\n\n### Constraint: x\nIt SHALL x.\n\n#### Replaces\n- expand-contract\n\n#### Scenario: s\n- ok\n";

fn graph(architecture: &str) -> Graph {
    let root = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";
    let a = "---\ntitle: A\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), root.into());
    files.insert(PathBuf::from("ARCHITECTURE.md"), architecture.into());
    files.insert(PathBuf::from("specs/a/SPEC.md"), a.into());
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

fn bundle(architecture: &str) -> String {
    clm_context::bundle_from(&graph(architecture), Some(architecture), "a").unwrap()
}

#[test]
fn a_project_gets_the_carried_section() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let out = bundle(PROJECT);
    let at = |needle: &str| {
        out.find(needle)
            .unwrap_or_else(|| panic!("no {needle:?} in {out}"))
    };
    assert!(at("### Constraint: x") < at("# CLM's constraints"));
    assert!(at("# CLM's constraints") < at("# Ancestors"));
    assert!(at("### Constraint: proven-by-tests (method)") > at("# CLM's constraints"));
    assert!(out.contains("### Constraint: yagni (default)"));
    assert!(!out.contains("### Constraint: expand-contract"));
    assert!(out.contains("clm guide constraints"));
    assert!(out.contains("kept in CLM's repository"));
    let method = at("### Constraint: proven-by-tests (method)");
    assert!(!out[method..method + out[method..].find("\n### ").unwrap()].contains("Guarantee:"));
}

#[test]
fn a_replaceable_entry_prints_its_guarantee_and_statement() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let out = bundle("## Constraints\n");
    let entry = clm_model::carried()
        .into_iter()
        .find(|c| c.slug == "expand-contract")
        .unwrap();
    assert!(out.contains("### Constraint: expand-contract (replaceable)"));
    assert!(out.contains(&entry.statement));
    assert!(out.contains(&format!("Guarantee: {}", entry.guarantee.unwrap())));
}

#[test]
fn the_source_gets_no_section() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let out = bundle(SOURCE);
    assert!(!out.contains("# CLM's constraints"));
}

#[test]
fn a_project_without_the_file_gets_the_section_first() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let out = clm_context::bundle_from(&graph("## Constraints\n"), None, "a").unwrap();
    assert!(out.starts_with("# CLM's constraints"), "{out}");
}

#[test]
fn bundle_from_matches_the_checkout() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("ARCHITECTURE.md"), PROJECT).unwrap();
    let g = graph(PROJECT);
    let on_disk = clm_context::bundle(&g, dir.path(), "a").unwrap();
    assert_eq!(on_disk, bundle(PROJECT));
}

#[test]
fn section_precedes_served_capabilities_in_carried_order() {
    // req:context/bundle-carries-constraints@ff31b1a3
    let root = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n";
    let a = "---\ntitle: A\nstatus: active\nowners: []\nserves: [capability/c]\n---\n\n## Contract\nc\n\n## Requirements\n\n### Requirement: p\nIt SHALL p.\n\n#### Scenario: s\n- ok\n";
    let caps =
        "## Capabilities\n\n### Capability: c\nC-TEXT\n\n#### Why\nw\n\n#### Scenario: s\n- ok\n";
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), root.into());
    files.insert(PathBuf::from("ARCHITECTURE.md"), PROJECT.into());
    files.insert(PathBuf::from("CAPABILITIES.md"), caps.into());
    files.insert(PathBuf::from("specs/a/SPEC.md"), a.into());
    let (g, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let out = clm_context::bundle_from(&g, Some(PROJECT), "a").unwrap();
    let section = out.find("# CLM's constraints").unwrap();
    assert!(section < out.find("# Capabilities served").unwrap());
    assert!(out.contains("from the `clm` that runs"));
    let places: Vec<usize> = clm_model::carried()
        .into_iter()
        .filter(|c| c.slug != "expand-contract")
        .map(|c| out.find(&format!("### Constraint: {} (", c.slug)).unwrap())
        .collect();
    assert!(places.windows(2).all(|w| w[0] < w[1]));
}
