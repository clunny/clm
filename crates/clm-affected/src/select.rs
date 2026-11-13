//! `select`: what a set of changed paths can break, from the spec graph alone.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use clm_model::Graph;

/// A changed path, relative to the repository root, and whether it still exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changed {
    pub path: String,
    pub exists: bool,
}

/// What `clm affected` prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Every test must run: the first changed path, in path order, that the `[affected] all`
    /// globs match or no node's `paths` hold.
    All(String),
    /// The affected nodes and the selected files, each sorted.
    Nodes {
        nodes: Vec<String>,
        files: Vec<String>,
    },
}

/// The selection for `changed`, given `files`, each scanned file with the requirement IDs its
/// `req:` markers cite (none for a file without one), and the `[affected] all` globs.
/// Doc-only paths (`ledger`'s `doc_only`) are left out of `changed` first.
pub fn select(
    graph: &Graph,
    changed: &[Changed],
    files: &BTreeMap<String, Vec<String>>,
    all: &[String],
) -> Selection {
    let mut changed: Vec<&Changed> = changed
        .iter()
        .filter(|c| !clm_ledger::doc_only(&c.path))
        .collect();
    changed.sort_by(|a, b| a.path.cmp(&b.path));

    let mut nodes = BTreeSet::new();
    for c in &changed {
        let owner = clm_check::owner(graph, Path::new(&c.path));
        match owner {
            Some(owner) if !all.iter().any(|g| clm_check::path_matches(g, &c.path)) => {
                nodes.insert(owner.to_string());
            }
            _ => return Selection::All(c.path.clone()),
        }
    }
    let nodes = with_dependants(graph, nodes);

    let mut selected: BTreeSet<String> = changed
        .iter()
        .filter(|c| c.exists)
        .map(|c| c.path.clone())
        .collect();
    for (path, cited) in files {
        let hit = if cited.is_empty() {
            clm_check::owner(graph, Path::new(path)).is_some_and(|o| nodes.contains(o))
        } else {
            cited
                .iter()
                .any(|id| nodes.contains(id.split('/').next().unwrap_or(id)))
        };
        if hit {
            selected.insert(path.clone());
        }
    }
    Selection::Nodes {
        nodes: nodes.into_iter().collect(),
        files: selected.into_iter().collect(),
    }
}

/// `nodes` and every node that depends on one of them, directly or indirectly.
fn with_dependants(graph: &Graph, mut nodes: BTreeSet<String>) -> BTreeSet<String> {
    loop {
        let more: Vec<String> = graph
            .nodes
            .values()
            .filter(|n| !nodes.contains(&n.id))
            .filter(|n| n.frontmatter.depends_on.iter().any(|d| nodes.contains(d)))
            .map(|n| n.id.clone())
            .collect();
        if more.is_empty() {
            break;
        }
        nodes.extend(more);
    }
    nodes
}

/// What both selections can break: when both are `All`, the first; when one is, the other;
/// otherwise the components and files both hold, each sorted.
pub fn overlap(a: Selection, b: Selection) -> Selection {
    match (a, b) {
        (first @ Selection::All(_), Selection::All(_)) => first,
        (Selection::All(_), other) | (other, Selection::All(_)) => other,
        (
            Selection::Nodes { nodes, files },
            Selection::Nodes {
                nodes: more_nodes,
                files: more_files,
            },
        ) => {
            let both = |x: Vec<String>, y: &[String]| -> Vec<String> {
                x.into_iter().filter(|v| y.contains(v)).collect()
            };
            let files = both(files, &more_files);
            // Components alone, with no file in both, are nothing to run.
            let nodes = if files.is_empty() {
                Vec::new()
            } else {
                both(nodes, &more_nodes)
            };
            Selection::Nodes { nodes, files }
        }
    }
}
