//! `undeclared-imports`: imports between nodes that `depends_on` declares (see
//! specs/check/SPEC.md). The edges come from `init`'s `import_edges`, which the `clm check`
//! command passes in, since tree-sitter doesn't build for wasm.

use std::path::PathBuf;

use clm_model::{Graph, Node, Violation};

/// One undeclared import: importing file, imported file, and the IDs of nodes `a` and `b`.
type Pair = (String, String, String, String);

/// Each pair among `edges` that `graph` leaves undeclared: from a file of component `a` to a file of
/// another node `b` that `a`'s `depends_on` doesn't list. An edge with an end in no node's
/// `paths` is skipped; a file in several nodes' `paths` is judged for each.
fn pairs(graph: &Graph, edges: &[(String, String)]) -> Vec<Pair> {
    let nodes_of = |file: &str| -> Vec<&Node> {
        graph
            .nodes
            .values()
            .filter(|n| n.frontmatter.paths.iter().any(|g| crate::matches(g, file)))
            .collect()
    };
    let mut out = Vec::new();
    for (from, to) in edges {
        let targets = nodes_of(to);
        for a in nodes_of(from) {
            for b in &targets {
                if b.id != a.id && !a.frontmatter.depends_on.contains(&b.id) {
                    out.push((from.clone(), to.clone(), a.id.clone(), b.id.clone()));
                }
            }
        }
    }
    out
}

/// The violations of `edges` against `graph`, then, with the `checkout` feature, those the
/// graph each `proposed` or `approved` change would leave adds, change by change
/// (`undeclared-imports-before-apply`). A change whose deltas don't apply is skipped.
pub fn undeclared_imports(graph: &Graph, edges: &[(String, String)]) -> Vec<Violation> {
    let current = pairs(graph, edges);
    #[cfg_attr(not(feature = "checkout"), allow(unused_mut))]
    let mut out: Vec<Violation> = current
        .iter()
        .map(|(from, to, a, b)| {
            violation(
                from,
                format!("imports {to} of component `{b}`, and `{a}` doesn't depend on `{b}`"),
            )
        })
        .collect();
    #[cfg(feature = "checkout")]
    for change in graph.changes.values() {
        if !change.frontmatter.status.is_pending() {
            continue;
        }
        let Some(after) = clm_apply::resulting(graph, change) else {
            continue;
        };
        for pair in pairs(&after, edges) {
            if !current.contains(&pair) {
                let (from, to, a, b) = pair;
                out.push(violation(
                    &from,
                    format!(
                        "imports {to} of component `{b}`, and `{a}` wouldn't depend on `{b}` after {}",
                        change.id
                    ),
                ));
            }
        }
    }
    out
}

fn violation(from: &str, message: String) -> Violation {
    Violation {
        path: PathBuf::from(from),
        rule: "check/undeclared-imports",
        message,
    }
}
