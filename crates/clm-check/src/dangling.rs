//! `dangling-markers`: every `req:` marker names a requirement a test can prove (see
//! specs/check/SPEC.md).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clm_model::{Graph, Layer, Violation};

use crate::Marker;

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
