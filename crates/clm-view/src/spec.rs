//! The `spec` view: the architecture, every node nested under its parent.

use std::collections::{BTreeMap, BTreeSet};

use clm_check::{Marker, State, pending, report};
use clm_model::{Graph, Layer};
use serde::Serialize;

use crate::FlightView;
use crate::chart::{self, Cell, Line, Style, plain, span, word};
use crate::flight::{live_progress, status_name};

/// The queue state of each node that isn't converged, by node ID, as the `flight` model
/// gives it: what the `spec` chart and SVG show beside the model.
pub(crate) type States = BTreeMap<String, String>;

/// The queue states of the nodes `flight` lists; a node it doesn't list is converged.
pub fn queue_states(flight: &FlightView) -> States {
    flight
        .nodes
        .iter()
        .map(|n| (n.node.clone(), n.state.clone()))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecView {
    /// Always `spec`.
    pub view: &'static str,
    /// Every node once, parents before their children, siblings by ID.
    pub nodes: Vec<SpecNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SpecNode {
    pub node: String,
    /// `None` for the root.
    pub parent: Option<String>,
    pub title: String,
    /// `active`, `specd` or `deprecated`.
    pub status: String,
    pub owners: Vec<String>,
    pub depends_on: Vec<String>,
    /// The requirements it holds by coverage state (`check/pending-report`): cited at their
    /// current version, only at others, or not at all.
    pub current: usize,
    pub behind: usize,
    /// Requirements nothing cites.
    pub unbuilt: usize,
    /// Requirements a live change removed from it that a marker still cites, and those
    /// removed with their node that a marker in a file it owns still cites, or, for a
    /// marker in a file no node owns, the root (`node-removal`).
    pub obsolete: usize,
    /// The requirements it holds, with the cited capabilities on the node they are pending on.
    pub total: usize,
    /// Over its size budget, as the size report given to [`spec_view`] says
    /// (`oversized-is-a-state`); absent when no report was given.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oversized: Option<bool>,
    /// The text of its Contract section, trailing whitespace trimmed (`node-detail`).
    pub contract: String,
    /// The live changes with a delta for it, by ID.
    pub changes: Vec<NodeChange>,
    /// The backlog items whose `nodes` name it, by kind then slug.
    pub backlog: Vec<NodeItem>,
}

/// A live change with a delta for a node, its progress as `clm_check::change_progress`
/// gives it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeChange {
    pub change: String,
    /// Named as the `flight` chart names it.
    pub status: String,
    pub current: usize,
    pub total: usize,
    pub complete: bool,
}

/// A backlog item naming a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NodeItem {
    pub slug: String,
    pub kind: String,
    pub title: String,
}

/// Each node's live changes, by node ID, sorted by change ID: those outside the archive
/// with status neither `done` nor `abandoned` that have a delta for it, once per change.
fn node_changes(graph: &Graph, markers: &[Marker]) -> BTreeMap<String, Vec<NodeChange>> {
    let mut by_node: BTreeMap<String, Vec<NodeChange>> = BTreeMap::new();
    for (c, p) in live_progress(graph, markers) {
        let nodes: BTreeSet<&str> = c.deltas.iter().map(|d| d.node.as_str()).collect();
        for node in nodes {
            by_node
                .entry(node.to_string())
                .or_default()
                .push(NodeChange {
                    change: p.change.clone(),
                    status: status_name(c.frontmatter.status),
                    current: p.current,
                    total: p.total,
                    complete: p.complete,
                });
        }
    }
    by_node
}

/// Each node's backlog items, by node ID, in kind order then by slug.
fn node_backlog(graph: &Graph) -> BTreeMap<&str, Vec<NodeItem>> {
    let mut items: Vec<_> = graph.backlog.values().collect();
    items.sort_by(|a, b| (a.frontmatter.kind, &a.slug).cmp(&(b.frontmatter.kind, &b.slug)));
    let mut by_node: BTreeMap<&str, Vec<NodeItem>> = BTreeMap::new();
    for i in items {
        let nodes: BTreeSet<&str> = i.frontmatter.nodes.iter().map(String::as_str).collect();
        for node in nodes {
            by_node.entry(node).or_default().push(NodeItem {
                slug: i.slug.clone(),
                kind: i.frontmatter.kind.to_string(),
                title: i.frontmatter.title.clone(),
            });
        }
    }
    by_node
}

/// The `spec` view of `graph` with the markers of its code. `oversized` is the size
/// report: the IDs of the nodes `clm_check::size` says are over their budgets, or `None`
/// when sizes weren't checked, which leaves every node's `oversized` unset.
pub fn spec_view(
    graph: &Graph,
    markers: &[Marker],
    oversized: Option<&BTreeSet<String>>,
) -> SpecView {
    let mut counts: BTreeMap<String, [usize; 4]> = BTreeMap::new();
    // A cited capability counts on the node the capabilities are pending on, as `pending`
    // puts it there; an entry standing by a proof counts on no node (`human-layer-proof`).
    let entries_node = graph.capability_tests_node();
    for c in report(graph, markers) {
        if c.proof.is_some() {
            continue;
        }
        let i = match c.state {
            State::Current => 0,
            State::Behind => 1,
            State::Unbuilt => 2,
            // Counted from `pending` below, on the node each line is pending on.
            State::Obsolete => continue,
        };
        let node = match c.id.split_once('/') {
            _ if Layer::of(&c.id).is_some() => entries_node,
            Some((n, _)) => n,
            None => "",
        };
        counts.entry(node.to_string()).or_default()[i] += 1;
    }
    // One per pending line: a removed node's requirement counts on each node owning a
    // file that cites it, or the root (`node-removal`), as `clm pending` counts it.
    for p in pending(graph, markers) {
        if p.state == State::Obsolete {
            counts.entry(p.node).or_default()[3] += 1;
        }
    }
    let mut changes = node_changes(graph, markers);
    let mut backlog = node_backlog(graph);
    let mut children: BTreeMap<Option<&str>, Vec<&str>> = BTreeMap::new();
    for n in graph.nodes.values() {
        children
            .entry(n.parent.as_deref())
            .or_default()
            .push(n.id.as_str());
    }
    let mut nodes = Vec::new();
    let mut stack: Vec<&str> = children
        .get(&None)
        .map(|roots| roots.iter().rev().copied().collect())
        .unwrap_or_default();
    while let Some(id) = stack.pop() {
        let n = &graph.nodes[id];
        let [current, behind, unbuilt, obsolete] = counts.get(id).copied().unwrap_or_default();
        nodes.push(SpecNode {
            node: n.id.clone(),
            parent: n.parent.clone(),
            title: n.frontmatter.title.clone(),
            status: format!("{:?}", n.frontmatter.status).to_lowercase(),
            owners: n.frontmatter.owners.clone(),
            depends_on: n.frontmatter.depends_on.clone(),
            current,
            behind,
            unbuilt,
            obsolete,
            total: current + behind + unbuilt,
            oversized: oversized.map(|o| o.contains(id)),
            contract: n.contract.trim_end().to_string(),
            changes: changes.remove(id).unwrap_or_default(),
            backlog: backlog.remove(id).unwrap_or_default(),
        });
        if let Some(kids) = children.get(&Some(id)) {
            stack.extend(kids.iter().rev());
        }
    }
    SpecView {
        view: "spec",
        nodes,
    }
}

/// The children of each node in the view's order, keyed by parent.
pub(crate) fn children(view: &SpecView) -> BTreeMap<Option<&str>, Vec<&SpecNode>> {
    let mut map: BTreeMap<Option<&str>, Vec<&SpecNode>> = BTreeMap::new();
    for n in &view.nodes {
        map.entry(n.parent.as_deref()).or_default().push(n);
    }
    map
}

/// The chart: an indented tree, one line per node with its ID, status, its queue state in
/// `states` when it isn't converged, `⚠ oversized` when it is, a bar by coverage state with `current/total` and
/// `✕<obsolete>`, owners, and `→` with its dependencies.
pub(crate) fn chart(view: &SpecView, states: &States) -> Vec<Line> {
    let kids = children(view);
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut ancestors: Vec<bool> = Vec::new();
    // Depth-first with explicit frames: (node, is last sibling, depth).
    let mut stack: Vec<(&SpecNode, bool, usize)> = Vec::new();
    if let Some(roots) = kids.get(&None) {
        for (i, r) in roots.iter().enumerate().rev() {
            stack.push((r, i + 1 == roots.len(), 0));
        }
    }
    while let Some((n, last, depth)) = stack.pop() {
        ancestors.truncate(depth.saturating_sub(1));
        let prefix = if depth == 0 {
            String::new()
        } else {
            chart::tree_prefix(&ancestors, last)
        };
        rows.push(row(n, states.get(&n.node), &prefix));
        if let Some(c) = kids.get(&Some(n.node.as_str())) {
            if depth > 0 {
                ancestors.push(last);
            }
            for (i, k) in c.iter().enumerate().rev() {
                stack.push((k, i + 1 == c.len(), depth + 1));
            }
        }
    }
    chart::align(rows)
}

/// The node's ten-cell bar by coverage state.
pub(crate) fn bar(n: &SpecNode) -> Cell {
    chart::state_bar(n.current, n.behind, n.unbuilt, 10)
}

/// What follows the bar: `current/total` and, when markers cite removed requirements,
/// `✕<obsolete>`.
pub(crate) fn count(n: &SpecNode) -> Cell {
    let mut count = vec![plain(format!(" {}/{}", n.current, n.total))];
    if n.obsolete > 0 {
        count.push(plain(" "));
        count.push(span(format!("✕{}", n.obsolete), Style::Rose));
    }
    count
}

fn row(n: &SpecNode, state: Option<&String>, prefix: &str) -> Vec<Cell> {
    let deps = if n.depends_on.is_empty() {
        Vec::new()
    } else {
        vec![span(format!("→ {}", n.depends_on.join(", ")), Style::Plain)]
    };
    vec![
        vec![plain(prefix), plain(&n.node)],
        vec![word(&n.status)],
        state.map(|s| vec![word(s)]).unwrap_or_default(),
        if n.oversized == Some(true) {
            vec![span("⚠ oversized", Style::Amber)]
        } else {
            Vec::new()
        },
        [bar(n), count(n)].concat(),
        vec![plain(n.owners.join(" "))],
        deps,
    ]
}
