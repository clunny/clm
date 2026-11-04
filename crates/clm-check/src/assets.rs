//! `check/asset-references`: a node's spec text names only assets it has.

use std::collections::BTreeSet;

use clm_model::{Graph, Violation};

use crate::spans::code_spans;

/// Violations for each concrete `` `assets/<path>` `` span in a node's Contract or Design
/// whose path isn't one of the node's assets. Requirements, scenarios and frontmatter are
/// never scanned.
pub(crate) fn violations(graph: &Graph) -> Vec<Violation> {
    let mut out = Vec::new();
    for node in graph.nodes.values() {
        let mentioned: BTreeSet<&str> = [&node.contract, &node.design]
            .into_iter()
            .flat_map(|text| code_spans(text))
            .map(|(_, span)| span)
            .filter(|span| is_concrete(span))
            .filter_map(|span| span.strip_prefix("assets/"))
            .collect();
        for path in mentioned {
            if !node.assets.contains_key(path) {
                out.push(Violation {
                    path: node.path.clone(),
                    rule: "check/asset-references",
                    message: format!(
                        "component `{}` mentions `assets/{path}`, which isn't in {}/assets/",
                        node.id,
                        node.path.parent().unwrap_or(&node.path).display()
                    ),
                });
            }
        }
    }
    out
}

/// A path names one file: no placeholder (`<`, `>`), no glob (`*`), not a directory (`/`).
fn is_concrete(path: &str) -> bool {
    !path.contains(['<', '>', '*']) && !path.ends_with('/')
}
