//! The `blockers` view: what holds work up, every blocked node under the root
//! blocker at the end of its chain of waits.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use clm_check::{Marker, Pending, pending};
use clm_model::{BacklogKind, Graph};
use clm_orchestrator::{ClaimRef, State, queue_from_refs};
use serde::Serialize;

use crate::chart::{self, Cell, Line, Style, plain, span, word};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BlockersView {
    /// Always `blockers`.
    pub view: &'static str,
    /// Roots that wait on the human first, then the rest, each by weight (highest first)
    /// then name.
    pub roots: Vec<Root>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Root {
    /// A node ID or `backlog/<slug>`.
    pub root: String,
    /// `node` or `backlog`.
    pub kind: String,
    /// A node's queue state (`ready` or `claimed`), or an item's kind.
    pub state: String,
    /// A `question` or `human-task` item.
    pub waits_on_human: bool,
    /// Distinct nodes it holds up, directly or through other nodes.
    pub weight: usize,
    /// The nodes it holds up, parents before children.
    pub holds: Vec<Held>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Held {
    pub node: String,
    /// The nodes between the root and this one, from the root's side.
    pub via: Vec<String>,
}

/// What a node waits on.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Target {
    Node(String),
    Item(String),
}

/// Where the release of pending requirement `id`, which a node needs, will come from
/// (`requirement-needs`): its node when it is unblocked, else each of its blockers in turn,
/// its items and what the requirements it needs reach. `seen` holds the requirements already
/// walked, so shared needs are walked once and a graph with a cycle of needs (which the model
/// reports as a violation) still terminates.
fn need_targets<'a>(
    id: &'a str,
    needed: &'a BTreeMap<String, Pending>,
    seen: &mut BTreeSet<&'a str>,
    out: &mut BTreeSet<Target>,
) {
    if !seen.insert(id) {
        return;
    }
    let Some(p) = needed.get(id) else { return };
    if p.blocked_by.is_empty() && p.waits_on.is_empty() {
        out.insert(Target::Node(p.node.clone()));
        return;
    }
    out.extend(
        p.blocked_by
            .iter()
            .map(|slug| Target::Item(format!("backlog/{slug}"))),
    );
    for need in &p.waits_on {
        need_targets(need, needed, seen, out);
    }
}

/// The `blockers` view; its arguments are [`crate::flight_view`]'s.
pub fn blockers_view(
    graph: &Graph,
    markers: &[Marker],
    refs: &[ClaimRef],
    now: u64,
    expiry_hours: u64,
) -> BlockersView {
    let queue = queue_from_refs(graph, markers, refs, now, expiry_hours);
    let needed: BTreeMap<String, Pending> = pending(graph, markers)
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();
    let states: BTreeMap<&str, &State> = queue
        .nodes
        .iter()
        .map(|n| (n.node.as_str(), &n.state))
        .collect();
    let waits: BTreeMap<&str, Vec<Target>> = queue
        .nodes
        .iter()
        .filter_map(|n| match &n.state {
            State::Blocked { by } => {
                let mut targets = BTreeSet::new();
                let mut seen = BTreeSet::new();
                for b in by {
                    match (&b.item, &b.requirement) {
                        (Some(item), _) => {
                            targets.insert(Target::Item(item.clone()));
                        }
                        (None, Some(id)) => need_targets(id, &needed, &mut seen, &mut targets),
                        (None, None) => {}
                    }
                }
                Some((n.node.as_str(), targets.into_iter().collect()))
            }
            _ => None,
        })
        .collect();

    // Root (as named) -> held node -> path from the root's side.
    let mut roots: BTreeMap<String, BTreeMap<String, Vec<String>>> = BTreeMap::new();
    let mut root_kind: BTreeMap<String, (String, String, bool)> = BTreeMap::new();
    for &start in waits.keys() {
        // Breadth-first along the waits, so the first path to a root is the shortest.
        let mut queue_: VecDeque<(Target, Vec<String>)> = VecDeque::new();
        let mut seen: BTreeSet<Target> = BTreeSet::new();
        queue_.push_back((Target::Node(start.to_string()), Vec::new()));
        while let Some((here, path)) = queue_.pop_front() {
            if !seen.insert(here.clone()) {
                continue;
            }
            match &here {
                Target::Node(id) if id != start && waits.contains_key(id.as_str()) => {
                    let mut via = path.clone();
                    via.push(id.clone());
                    for next in &waits[id.as_str()] {
                        queue_.push_back((next.clone(), via.clone()));
                    }
                }
                Target::Node(id) if id == start => {
                    for next in &waits[start] {
                        queue_.push_back((next.clone(), Vec::new()));
                    }
                }
                Target::Node(id) => {
                    let state = match states.get(id.as_str()) {
                        Some(State::Ready { .. }) => "ready",
                        Some(State::Claimed { .. }) => "claimed",
                        Some(State::Converged) => "converged",
                        Some(State::Blocked { .. }) => "blocked",
                        None => "unknown",
                    };
                    root_kind
                        .entry(id.clone())
                        .or_insert(("node".into(), state.into(), false));
                    let via: Vec<String> = path.iter().rev().cloned().collect();
                    roots
                        .entry(id.clone())
                        .or_default()
                        .entry(start.to_string())
                        .or_insert(via);
                }
                Target::Item(item) => {
                    let kind = item
                        .strip_prefix("backlog/")
                        .and_then(|slug| graph.backlog.get(slug))
                        .map(|b| b.frontmatter.kind);
                    let human = matches!(kind, Some(BacklogKind::Question | BacklogKind::Human));
                    root_kind.entry(item.clone()).or_insert((
                        "backlog".into(),
                        kind.map_or_else(|| "missing".to_string(), |k| k.to_string()),
                        human,
                    ));
                    let via: Vec<String> = path.iter().rev().cloned().collect();
                    roots
                        .entry(item.clone())
                        .or_default()
                        .entry(start.to_string())
                        .or_insert(via);
                }
            }
        }
    }

    let mut out: Vec<Root> = roots
        .into_iter()
        .map(|(root, held)| {
            let (kind, state, waits_on_human) = root_kind[&root].clone();
            let mut holds: Vec<Held> = held
                .into_iter()
                .map(|(node, via)| Held { node, via })
                .collect();
            holds.sort_by(|a, b| {
                let pa: Vec<&str> = a
                    .via
                    .iter()
                    .map(String::as_str)
                    .chain([a.node.as_str()])
                    .collect();
                let pb: Vec<&str> = b
                    .via
                    .iter()
                    .map(String::as_str)
                    .chain([b.node.as_str()])
                    .collect();
                pa.cmp(&pb)
            });
            Root {
                weight: holds.len(),
                root,
                kind,
                state,
                waits_on_human,
                holds,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        (!a.waits_on_human, std::cmp::Reverse(a.weight), &a.root).cmp(&(
            !b.waits_on_human,
            std::cmp::Reverse(b.weight),
            &b.root,
        ))
    });
    BlockersView {
        view: "blockers",
        roots: out,
    }
}

/// The chart: each root with its state, whether it waits on you and its weight, then the
/// tree of what it holds up; or `nothing is blocked`.
pub(crate) fn chart(view: &BlockersView) -> Vec<Line> {
    let mut lines = vec![chart::heading("blockers")];
    if view.roots.is_empty() {
        lines.push(vec![span("nothing is blocked", Style::Green)]);
        return lines;
    }
    let rows: Vec<Vec<Cell>> = view
        .roots
        .iter()
        .map(|r| {
            vec![
                vec![span(&r.root, Style::Indigo)],
                vec![word(&r.state)],
                if r.waits_on_human {
                    vec![span("waits on you", Style::Amber)]
                } else {
                    Vec::new()
                },
                vec![plain(format!("holds up {}", r.weight))],
            ]
        })
        .collect();
    for (root, row) in view.roots.iter().zip(chart::align(rows)) {
        lines.push(row);
        lines.extend(tree(root));
    }
    lines
}

/// The held nodes as a tree, by their `via` paths.
fn tree(root: &Root) -> Vec<Line> {
    let paths: Vec<Vec<&str>> = root
        .holds
        .iter()
        .map(|h| {
            h.via
                .iter()
                .map(String::as_str)
                .chain([h.node.as_str()])
                .collect()
        })
        .collect();
    // Whether the node at `path` is the last of its siblings: no path of the same length
    // and parent sorts after it.
    let last = |path: &[&str]| {
        let d = path.len();
        !paths
            .iter()
            .any(|p| p.len() == d && p[..d - 1] == path[..d - 1] && p[..] > path[..])
    };
    root.holds
        .iter()
        .zip(&paths)
        .map(|(h, path)| {
            let ancestors: Vec<bool> = (1..path.len()).map(|d| last(&path[..d])).collect();
            vec![
                plain(chart::tree_prefix(&ancestors, last(path))),
                span(&h.node, Style::Rose),
            ]
        })
        .collect()
}
