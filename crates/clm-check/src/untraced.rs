//! `check/untraced`: once either human layer holds an entry, every rule serves one
//! (`traced-rules`). Entries themselves are never reported.

use clm_model::{Graph, Violation};

/// One violation per rule whose effective serves is empty; none while the layers are empty.
pub(crate) fn violations(graph: &Graph) -> Vec<Violation> {
    if graph.layers.values().all(Vec::is_empty) {
        return Vec::new();
    }
    let mut out = Vec::new();
    for node in graph.nodes.values() {
        for r in &node.requirements {
            if graph.effective_serves(&r.id).is_empty() {
                out.push(Violation {
                    path: node.path.clone(),
                    rule: "check/untraced",
                    message: format!(
                        "{} serves nothing; give component {} a serves list or the rule a #### Serves block",
                        r.id, node.id
                    ),
                });
            }
        }
    }
    out
}
