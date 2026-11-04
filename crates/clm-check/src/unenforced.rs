//! `check/unenforced`: every architecture entry names the rules that enforce it, or says
//! why it is untested (`human-layer-proof`).

use std::path::Path;

use clm_model::{Graph, Layer, Violation};

/// One violation per architecture entry with neither block, in file order.
pub(crate) fn violations(graph: &Graph) -> Vec<Violation> {
    let entries = graph.layers.get(&Layer::Architecture).into_iter().flatten();
    entries
        .filter(|e| e.enforced_by.is_empty() && !e.untested)
        .map(|e| Violation {
            path: Path::new(Layer::Architecture.file()).to_path_buf(),
            rule: "check/unenforced",
            message: format!(
                "{} names no enforcing rule and no reason it is untested",
                e.id
            ),
        })
        .collect()
}
