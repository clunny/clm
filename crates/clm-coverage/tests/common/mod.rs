#![allow(dead_code)] // each test file uses part of this

use std::fs;
use std::path::{Path, PathBuf};

use clm_coverage::Marker;
use clm_model::Graph;

pub const ROOT: &str = "---\ntitle: R\nstatus: active\nowners: []\n---\n";

pub fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// A node's spec with `paths` and one requirement per slug.
pub fn spec(paths: &str, slugs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\n---\n\n## Requirements\n"
    );
    for slug in slugs {
        s.push_str(&format!(
            "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A change `id` with `status` whose delta for node `a` adds `added` and removes `removed`.
pub fn change(root: &Path, id: &str, status: &str, added: &[&str], removed: &[&str]) {
    write(
        root,
        &format!("changes/{id}/change.md"),
        &format!("---\nstatus: {status}\n---\n"),
    );
    let mut delta = String::from("---\ncomponent: a\n---\n");
    if !added.is_empty() {
        delta.push_str("## ADDED Requirements\n");
        for slug in added {
            delta.push_str(&format!(
                "\n### Requirement: {slug}\n{slug}.\n\n#### Scenario: s\n- ok\n"
            ));
        }
    }
    if !removed.is_empty() {
        delta.push_str("\n## REMOVED Requirements\n");
        for slug in removed {
            delta.push_str(&format!(
                "\n### Requirement: {slug} <!-- base: 0123456789abcdef -->\n"
            ));
        }
    }
    write(root, &format!("changes/{id}/deltas/a.md"), &delta);
}

pub fn graph(root: &Path) -> Graph {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// A marker at `path:line` citing `id` at `version`.
pub fn marker(id: &str, version: Option<&str>, path: &str, line: usize) -> Marker {
    Marker {
        id: id.into(),
        version: version.map(str::to_string),
        path: PathBuf::from(path),
        line,
    }
}

/// A marker at `path:line` citing `id` at its current version in `graph`.
pub fn current(graph: &Graph, id: &str, path: &str, line: usize) -> Marker {
    let version = graph.requirement(id).unwrap().version().to_string();
    marker(id, Some(&version), path, line)
}
