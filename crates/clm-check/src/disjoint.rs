//! `paths-disjoint`: a file belongs to one node (see specs/check/SPEC.md).

use std::path::PathBuf;

use clm_model::{Graph, Violation};

/// Each of the `tracked` files that the `paths` of two or more nodes match, once. Outside a
/// git work tree there are no tracked files, so none.
pub fn violations(tracked: &[PathBuf], graph: &Graph) -> Vec<Violation> {
    tracked
        .iter()
        .filter_map(|file| {
            let name = file.to_string_lossy();
            // `graph.nodes` is sorted by ID.
            let owners: Vec<String> = graph
                .nodes
                .values()
                .filter(|n| n.frontmatter.paths.iter().any(|g| crate::matches(g, &name)))
                .map(|n| format!("`{}`", n.id))
                .collect();
            (owners.len() > 1).then(|| Violation {
                path: file.clone(),
                rule: "check/paths-disjoint",
                message: format!("owned by {}", owners.join(", ")),
            })
        })
        .collect()
}
