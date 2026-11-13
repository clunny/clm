//! A component's `test_depends_on` contracts, in its bundle.

use std::path::PathBuf;

use clm_model::{Files, load_files};

fn node(title: &str, extra: &str, contract: &str, design: &str) -> Vec<u8> {
    format!(
        "---\ntitle: {title}\nstatus: active\nowners: []\n{extra}---\n\n\
         ## Contract\n{contract}\n\n## Design\n{design}\n"
    )
    .into_bytes()
}

#[test]
fn test_dependency_contract_only() {
    // req:context/bundle-contents@ec3b6384
    let mut files = Files::new();
    files.insert(
        PathBuf::from("specs/SPEC.md"),
        node("Root", "", "ROOT-C", "ROOT-D"),
    );
    files.insert(
        PathBuf::from("specs/app/SPEC.md"),
        node("App", "", "APP-CONTRACT", "APP-DESIGN"),
    );
    files.insert(
        PathBuf::from("specs/feat/SPEC.md"),
        node("Feat", "test_depends_on: [app]\n", "FEAT-C", "FEAT-D"),
    );
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let out = clm_context::bundle_from(&graph, None, "feat").unwrap();
    assert!(out.contains("APP-CONTRACT"), "{out}");
    assert!(!out.contains("APP-DESIGN"), "{out}");
}

#[test]
fn bundle_nodes_include_test_dependency() {
    // req:context/bundle-contents@ec3b6384
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), node("Root", "", "R", "R"));
    files.insert(
        PathBuf::from("specs/app/SPEC.md"),
        node("App", "", "A", "A"),
    );
    files.insert(
        PathBuf::from("specs/feat/SPEC.md"),
        node("Feat", "test_depends_on: [app]\n", "F", "F"),
    );
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let nodes = clm_context::bundle_nodes(&graph, "feat");
    assert_eq!(nodes.last().map(String::as_str), Some("app"), "{nodes:?}");
}

#[test]
fn test_dependency_listed_once_and_spine_skipped() {
    // req:context/bundle-contents@ec3b6384
    let mut files = Files::new();
    files.insert(
        PathBuf::from("specs/SPEC.md"),
        node("Root", "", "ROOT-C", "ROOT-D"),
    );
    files.insert(
        PathBuf::from("specs/app/SPEC.md"),
        node("App", "", "APP-CONTRACT", "APP-DESIGN"),
    );
    files.insert(
        PathBuf::from("specs/feat/SPEC.md"),
        node(
            "Feat",
            "depends_on: [app]\ntest_depends_on: [app, clm]\n",
            "FEAT-C",
            "FEAT-D",
        ),
    );
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    let out = clm_context::bundle_from(&graph, None, "feat").unwrap();
    assert_eq!(out.matches("APP-CONTRACT").count(), 1, "{out}");
    assert_eq!(out.matches("ROOT-C").count(), 1, "{out}");
}
