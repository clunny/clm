//! `removed-requirement-names`: no node's spec names a requirement a live change removed,
//! or one a pending change would remove (see specs/check/SPEC.md).

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use clm_model::{Change, Graph, Violation};

use crate::spans::code_spans;

/// `removed-requirement-names` violations for each node's `SPEC.md` below its frontmatter.
/// `removed` is `clm_coverage::removed(graph)`.
pub(crate) fn violations(graph: &Graph, removed: &BTreeMap<String, Vec<String>>) -> Vec<Violation> {
    let names = removed
        .iter()
        .map(|(id, changes)| (id.as_str(), changes[0].as_str()))
        .collect();
    scan(graph, &names, "removes")
}

/// The violations in `after`, the graph pending `change` would leave (`clm_apply::resulting`),
/// for the IDs its deltas list under `REMOVED Requirements` that `graph` holds and `after`
/// doesn't (`removed-names-before-apply`).
pub(crate) fn would_remove(graph: &Graph, after: &Graph, change: &Change) -> Vec<Violation> {
    let ids: BTreeSet<String> = change
        .deltas
        .iter()
        .flat_map(|d| d.removed.iter().map(|r| format!("{}/{}", d.node, r.slug)))
        .filter(|id| graph.requirement(id).is_some() && after.requirement(id).is_none())
        .collect();
    let names = ids
        .iter()
        .map(|id| (id.as_str(), change.id.as_str()))
        .collect();
    scan(after, &names, "would remove")
}

/// Each inline code span of a node's `SPEC.md` in `graph`, below its frontmatter, that names
/// one of `names` (a removed ID, to the change that removes it): the whole ID in any node, or
/// its bare slug, unless shared, in the ID's own node.
fn scan(graph: &Graph, names: &BTreeMap<&str, &str>, verb: &str) -> Vec<Violation> {
    if names.is_empty() {
        return Vec::new();
    }
    // The bare slugs each node may not name: its own removed IDs, less the shared slugs.
    let shared = shared_slugs(graph);
    let mut bare: BTreeMap<&str, BTreeMap<&str, &str>> = BTreeMap::new();
    for &id in names.keys() {
        if let Some((node, slug)) = id.split_once('/')
            && !shared.contains(slug)
            && !graph.decisions.contains_key(slug)
        {
            bare.entry(node).or_default().insert(slug, id);
        }
    }
    let mut out = Vec::new();
    for node in graph.nodes.values() {
        let text = &node.source;
        let body_start =
            clm_model::split_frontmatter(text).map_or(0, |(_, body)| text.len() - body.len());
        let own = bare.get(node.id.as_str());
        for (offset, span) in code_spans(&text[body_start..]) {
            let Some(id) = own
                .and_then(|own| own.get(span).copied())
                .or_else(|| names.get_key_value(span).map(|(&id, _)| id))
            else {
                continue;
            };
            let at = body_start + offset;
            let line = text[..at].matches('\n').count() + 1;
            out.push(Violation {
                path: PathBuf::from(format!("{}:{line}", node.path.display())),
                rule: "check/removed-requirement-names",
                message: format!("names `{id}`, which {} {verb}", names[id]),
            });
        }
    }
    out
}

/// Slugs a requirement of any node, or a human-layer entry, also names; with the decisions'
/// names (`docs/decisions/<slug>.md`), these are the shared slugs.
fn shared_slugs(graph: &Graph) -> BTreeSet<&str> {
    graph
        .nodes
        .values()
        .flat_map(|n| &n.requirements)
        .chain(graph.layers.values().flatten())
        .filter_map(|r| r.id.split_once('/').map(|(_, slug)| slug))
        .collect()
}
