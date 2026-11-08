//! `why_chain`: the chain by which a change selects a path (`why-chain`).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

use clm_model::Graph;

use crate::combined;
use crate::select::{Changed, Selection};

/// The lines saying why the change selects `path`: its first reason that holds, then the
/// shortest `depends_on` path from the component that brought it in to a component owning a
/// changed file. `cited` and `reads` are what the scan found, as for `combined`.
pub fn why_chain(
    graph: &Graph,
    differing: &[Changed],
    cited: &BTreeMap<String, Vec<String>>,
    reads: &BTreeMap<String, Vec<String>>,
    all: &[String],
    path: &str,
) -> Vec<String> {
    let (nodes, files) = match combined(graph, differing, cited, reads, all) {
        Selection::All(file) => return vec![format!("all: {file}")],
        Selection::Nodes { nodes, files } => (nodes, files),
    };
    if !files.iter().any(|f| f == path) {
        return vec![format!("{path} not selected")];
    }

    // The changed files the selection reasons from, in path order, with their owners.
    let changed: Vec<(&str, &str)> = differing
        .iter()
        .filter(|c| !clm_ledger::doc_only(&c.path))
        .filter_map(|c| {
            Some((
                c.path.as_str(),
                clm_check::owner(graph, Path::new(&c.path))?,
            ))
        })
        .collect();
    if differing
        .iter()
        .any(|c| c.path == path && !clm_ledger::doc_only(&c.path))
    {
        return vec![format!("{path} (changed)")];
    }
    if let Some(read) = reads.get(path).and_then(|globs| {
        differing
            .iter()
            .find(|c| globs.iter().any(|g| clm_check::path_matches(g, &c.path)))
    }) {
        return vec![format!("{path} reads {}", read.path)];
    }

    let affected: BTreeSet<&str> = nodes.iter().map(String::as_str).collect();
    let cites = cited
        .get(path)
        .into_iter()
        .flatten()
        .map(|id| id.split('/').next().unwrap_or(id))
        .filter(|c| affected.contains(c))
        .min();
    let (mut lines, start) = match cites {
        Some(component) => (vec![format!("{path} cites {component}")], component),
        None => {
            let owner = clm_check::owner(graph, Path::new(path)).expect("a selected file is owned");
            (vec![format!("{path} owned by {owner}")], owner)
        }
    };

    let hops = route(graph, start, &changed);
    for pair in hops.windows(2) {
        lines.push(format!("{} depends on {}", pair[0], pair[1]));
    }
    let last = hops.last().expect("a route holds its start");
    let (owned, _) = changed
        .iter()
        .find(|(_, owner)| owner == last)
        .expect("a route ends at an owner of a changed file");
    lines.push(format!("{last} owns {owned} (changed)"));
    lines
}

/// The components from `start` to the nearest owner of a changed file along `depends_on`,
/// among equally short routes the first in ID order.
fn route<'a>(graph: &'a Graph, start: &'a str, changed: &[(&str, &str)]) -> Vec<&'a str> {
    let owns = |id: &str| changed.iter().any(|(_, owner)| *owner == id);
    let mut parent: BTreeMap<&str, &str> = BTreeMap::new();
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(id) = queue.pop_front() {
        if owns(id) {
            let mut hops = vec![id];
            while let Some(&p) = parent.get(hops.last().expect("hops is not empty")) {
                hops.push(p);
            }
            hops.reverse();
            return hops;
        }
        let mut deps: Vec<&str> = graph.nodes[id]
            .frontmatter
            .depends_on
            .iter()
            .map(String::as_str)
            .collect();
        deps.sort();
        for dep in deps {
            if seen.insert(dep) {
                parent.insert(dep, id);
                queue.push_back(dep);
            }
        }
    }
    unreachable!("an affected component reaches an owner of a changed file")
}
