//! Fixtures shared by the view tests: in-memory graphs and markers, no checkout.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use clm_coverage::Marker;
use clm_model::{Files, Graph};
use clm_orchestrator::{ClaimRef, DEFAULT_EXPIRY_HOURS};
use clm_view::{Format, Name, View, blockers_view, flight_view, queue_states, render, spec_view};

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

/// The graph and the markers of the fixture repository at `root`, which must be valid, as
/// `clm status` and `clm view` read a working tree.
pub fn read(root: &Path) -> (Graph, Vec<Marker>) {
    let (graph, violations) = clm_model::load(root);
    assert!(violations.is_empty(), "{violations:?}");
    (graph, clm_coverage::scan(root).req)
}

/// What `clm status` prints for the fixture at `root` with the component claims `refs` and
/// the backlog claims `backlog` at [`NOW`], naming no commit and given no size report.
pub fn status_of(root: &Path, refs: &[ClaimRef], backlog: &[ClaimRef]) -> String {
    let (graph, markers) = read(root);
    clm_view::status_text(&graph, &markers, refs, backlog, NOW, None, None)
}

/// What `clm view <name>` prints for the fixture at `root` with the claims `refs` at
/// [`NOW`], naming no commit and given no size report.
pub fn view_of(root: &Path, name: Name, format: Format, width: usize, refs: &[ClaimRef]) -> String {
    let (graph, markers) = read(root);
    let flight = || flight_view(&graph, &markers, refs, NOW, DEFAULT_EXPIRY_HOURS);
    let view = match name {
        Name::Spec => View::Spec(spec_view(&graph, &markers, None), queue_states(&flight())),
        Name::Flight => View::Flight(flight()),
        Name::Blockers => View::Blockers(blockers_view(
            &graph,
            &markers,
            refs,
            NOW,
            DEFAULT_EXPIRY_HOURS,
        )),
    };
    render(&view, format, width, None)
}
