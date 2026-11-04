//! Fixtures shared by the view tests: in-memory graphs and markers, no checkout.
#![allow(dead_code)]

use std::path::PathBuf;

use clm_check::Marker;
use clm_model::{Files, Graph};
use clm_orchestrator::ClaimRef;

/// A node with `status`, depending on `deps` (comma-separated), owned by `owners` (a YAML
/// list body), with requirements `reqs`, each `It SHALL.`.
pub fn node(status: &str, deps: &str, owners: &str, reqs: &[&str]) -> String {
    let mut s = format!(
        "---\ntitle: T\nstatus: {status}\nowners: [{owners}]\ndepends_on: [{deps}]\n---\n\n\
         ## Contract\nc\n\n## Design\nd\n\n## Requirements\n"
    );
    for r in reqs {
        s.push_str(&format!(
            "\n### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n"
        ));
    }
    s
}

/// A `specd` node with no owners.
pub fn specd(deps: &str, reqs: &[&str]) -> String {
    node("specd", deps, "", reqs)
}

pub fn delta(node: &str, reqs: &[&str]) -> String {
    let mut s = format!("---\ncomponent: {node}\n---\n## ADDED Requirements\n");
    for r in reqs {
        s.push_str(&format!(
            "### Requirement: {r}\nIt SHALL.\n\n#### Scenario: s\n- ok\n\n"
        ));
    }
    s
}

/// `change.md` with `status` and `extra` frontmatter lines.
pub fn change(status: &str, extra: &str) -> String {
    format!("---\nstatus: {status}\n{extra}---\n")
}

/// A backlog item of `kind` blocking `blocks` (comma-separated requirement IDs).
pub fn item(kind: &str, blocks: &str) -> String {
    format!("---\ntitle: I\nkind: {kind}\nblocks: [{blocks}]\n---\n")
}

/// `s` with its ANSI escapes removed.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A graph loaded from `(path, text)` files, which must be valid.
pub fn graph(files: &[(&str, &str)]) -> Graph {
    let files: Files = files
        .iter()
        .map(|(p, t)| (PathBuf::from(p), t.as_bytes().to_vec()))
        .collect();
    let (graph, violations) = clm_model::load_files(&files);
    assert!(violations.is_empty(), "{violations:?}");
    graph
}

/// A marker citing `id` at `version` (`None` for a bare one) at `t.rs:<line>`.
pub fn marker(id: &str, version: Option<&str>, line: usize) -> Marker {
    Marker {
        id: id.into(),
        version: version.map(String::from),
        path: PathBuf::from("t.rs"),
        line,
    }
}

/// A marker citing `id` at its current version in `graph`.
pub fn current(graph: &Graph, id: &str) -> Marker {
    marker(id, Some(graph.requirement(id).unwrap().version()), 1)
}

pub const NOW: u64 = 1_800_000_000;

/// A live claim on `node` pushed `hours` hours before [`NOW`].
pub fn claim(node: &str, hours: u64) -> ClaimRef {
    ClaimRef {
        node: node.into(),
        tip_time: NOW - hours * 3600,
        merged: false,
    }
}

/// The text of `json` at `pointer`, panicking with the whole document when it's missing.
pub fn at<'a>(json: &'a serde_json::Value, pointer: &str) -> &'a serde_json::Value {
    json.pointer(pointer)
        .unwrap_or_else(|| panic!("no {pointer} in {json:#}"))
}
