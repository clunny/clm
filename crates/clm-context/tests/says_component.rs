//! The bundle's ancestors section (`bundle-contents`).

use std::path::PathBuf;

use clm_context::bundle_from;
use clm_model::{Files, Graph, load_files};

/// Components `a` and `a.b` under root `clm`.
fn graph() -> Graph {
    let spec = |title: &str| {
        format!("---\ntitle: {title}\nstatus: active\nowners: []\n---\n\n## Contract\nc\n")
    };
    let mut files = Files::new();
    files.insert(PathBuf::from("specs/SPEC.md"), spec("CLM").into());
    files.insert(PathBuf::from("specs/a/SPEC.md"), spec("A").into());
    files.insert(PathBuf::from("specs/a/b/SPEC.md"), spec("B").into());
    let (graph, violations) = load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

#[test]
fn nested_component_has_ancestors_section() {
    // req:context/bundle-contents@ffb0697c
    let out = bundle_from(&graph(), None, "a.b").unwrap();
    let at = |s: &str| {
        out.find(s)
            .unwrap_or_else(|| panic!("{s} missing in {out}"))
    };
    assert!(out.contains("\n# Ancestors\n\n## clm: CLM\n"), "{out}");
    assert!(
        at("## clm: CLM") < at("## a: A") && at("## a: A") < at("## a.b: B"),
        "{out}"
    );
}
