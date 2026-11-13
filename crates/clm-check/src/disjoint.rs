//! `paths-disjoint`: a file belongs to one component (see specs/check/SPEC.md).

use std::path::PathBuf;

use clm_model::{Graph, Violation};

/// Each of the `tracked` files that two or more components own (`ownership::owners`), once.
/// Outside a git work tree there are no tracked files, so none.
pub fn violations(tracked: &[PathBuf], graph: &Graph) -> Vec<Violation> {
    tracked
        .iter()
        .filter_map(|file| {
            // `owners` come sorted by ID.
            let owners = crate::ownership::owners(graph, &file.to_string_lossy());
            (owners.len() > 1).then(|| Violation {
                path: file.clone(),
                rule: "check/paths-disjoint",
                message: format!(
                    "owned by {}",
                    owners
                        .iter()
                        .map(|n| format!("`{}`", n.id))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            })
        })
        .collect()
}
