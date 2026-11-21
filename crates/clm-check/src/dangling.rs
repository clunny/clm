//! `dangling-markers`: every `req:` marker names a requirement a test can prove (see
//! specs/check/SPEC.md).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clm_model::{Graph, Layer, Violation};

use clm_coverage::Marker;
#[cfg(feature = "checkout")]
use clm_coverage::Scan;

/// Every `req:` marker of a scan, those outside a test included (`Scan::outside`), each once
/// by its file, line and ID: a marker that proves nothing still can't name what doesn't exist.
#[cfg(feature = "checkout")]
pub(crate) fn every_marker(scan: &Scan) -> Vec<Marker> {
    let mut seen = std::collections::BTreeSet::new();
    scan.req
        .iter()
        .chain(&scan.outside)
        .filter(|m| seen.insert((m.path.clone(), m.line, m.id.clone())))
        .cloned()
        .collect()
}

/// `dangling-markers` violations. `removed` is `clm_coverage::removed(graph)`.
pub fn violations(
    graph: &Graph,
    markers: &[Marker],
    removed: &BTreeMap<String, Vec<String>>,
) -> Vec<Violation> {
    markers
        .iter()
        // A marker of a requirement a live change removed marks obsolete code, not a
        // dangling marker: the node's next build removes it (`removed-requirements-build-out`).
        .filter(|m| !removed.contains_key(&m.id))
        .filter_map(|m| {
            // A test never proves an architecture entry or an untested entry
            // (`human-layer-proof`).
            let message = match graph.requirement(&m.id) {
                None if m.id.starts_with("capability/") => format!(
                    "marker cites `{}`, which is not a requirement; a change that removes the capability lists it under removes_capabilities",
                    m.id
                ),
                None => format!("marker cites `{}`, which is not a requirement", m.id),
                Some(_) if Layer::of(&m.id) == Some(Layer::Architecture) => {
                    "architecture entries are proven by the rules their #### Enforced by names, not by tests".to_string()
                }
                Some(r) if r.untested => format!("{} is untested", m.id),
                Some(_) => return None,
            };
            Some(Violation {
                path: PathBuf::from(m.to_string()),
                rule: "check/dangling-markers",
                message,
            })
        })
        .collect()
}

#[cfg(all(test, feature = "checkout"))]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn marker(id: &str, line: usize) -> Marker {
        Marker {
            id: id.to_string(),
            version: None,
            path: PathBuf::from("src/lib.rs"),
            line,
        }
    }

    #[test]
    fn a_marker_only_outside_a_test_is_judged_and_one_in_both_lists_once() {
        // req:check/dangling-markers@80e30156
        let scan = Scan {
            req: vec![marker("a/both", 2)],
            outside: vec![marker("a/both", 2), marker("a/outside", 5)],
            remove_by: Vec::new(),
        };
        let (graph, _) = clm_model::load(std::path::Path::new("/nonexistent-clm-root"));
        let found = violations(&graph, &every_marker(&scan), &BTreeMap::new());
        let lines: Vec<String> = found.iter().map(|v| v.path.display().to_string()).collect();
        assert_eq!(lines, ["src/lib.rs:2", "src/lib.rs:5"]);
    }
}
