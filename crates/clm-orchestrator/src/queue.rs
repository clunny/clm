//! The node queue: every node of the graph with its state, its pending requirements and
//! its effective priority, and the complete changes (`node-reconciliation`).
//!
//! [`queue_from_refs`] is the queue without git or a checkout, for `clm next` and the `web`
//! Worker alike. For each node, as the Design says:
//! - its pending requirements are those `check` reports as not current, obsolete ones
//!   included (`check/pending-report`);
//! - a requirement is blocked while a backlog item names it in `blocks`
//!   (`requirement-blocks`), or while a requirement its `Needs` name is pending
//!   (`requirement-needs`);
//! - it is `converged` with nothing pending; `blocked` when everything pending is blocked;
//!   otherwise `ready`, whatever its `depends_on` (`needs-alone-order`); a live claim makes it
//!   `claimed` instead, so the `claimed` nodes are exactly the live claims;
//! - its effective priority is the highest among its unblocked pending requirements and the
//!   lines waiting on its lines (a `Needs`, a move, an obsolete requirement's own ID, a serving
//!   rule), each wait resolved to its lines by `check.coverage`'s `waits`, through nodes of any
//!   state (`priority-orders-nodes`).

use std::collections::{BTreeMap, BTreeSet};

use clm_coverage::{Marker, Pending, complete_changes, pending, waits};
use clm_model::{Graph, Priority};
use serde::Serialize;

use crate::{DEFAULT_EXPIRY_HOURS, lapsed};

/// A claim branch `impl/<node>` on `origin`: its tip's committer time, in Unix seconds, and
/// whether that tip is already in `origin/main` (`merged-claims-released`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimRef {
    /// The branch name without `impl/`.
    pub node: String,
    pub tip_time: u64,
    pub merged: bool,
}

/// A claim's state as `clm status` names it: `merged`, else `lapsed` when its tip is older
/// than `expiry_hours` at `now`, else `live`.
pub fn claim_state(r: &ClaimRef, now: u64, expiry_hours: u64) -> &'static str {
    if r.merged {
        "merged"
    } else if lapsed(r.tip_time, now, expiry_hours) {
        "lapsed"
    } else {
        "live"
    }
}

/// The `claims` section of `clm status` (`status-claims`): one row per node claim, `<node>`
/// and its state by the default expiry, then one per backlog claim, `backlog/<slug>` and its
/// state; `[["none"]]` when there are none.
pub fn claims_section(nodes: &[ClaimRef], backlog: &[ClaimRef], now: u64) -> Vec<Vec<String>> {
    let state = |r: &ClaimRef| claim_state(r, now, DEFAULT_EXPIRY_HOURS).to_string();
    let mut rows: Vec<Vec<String>> = nodes
        .iter()
        .map(|r| vec![r.node.clone(), state(r)])
        .collect();
    rows.extend(
        backlog
            .iter()
            .map(|r| vec![format!("backlog/{}", r.node), state(r)]),
    );
    if rows.is_empty() {
        vec![vec!["none".into()]]
    } else {
        rows
    }
}

/// A pending requirement as the queue reports it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Requirement {
    /// `<node-id>/<slug>`.
    pub id: String,
    /// Its current version; `None` when it's obsolete.
    pub version: Option<String>,
    /// `unbuilt`, `behind` or `obsolete`.
    pub state: String,
    /// Behind only: the distinct versions its markers cite, `none` for a bare marker.
    pub cited: Vec<String>,
    /// The markers citing it, as `<file>:<line>`.
    pub markers: Vec<String>,
    pub priority: Priority,
    /// Slugs of the backlog items whose `blocks` name it.
    pub blocked_by: Vec<String>,
    /// What else it waits on, as `<node>/<slug>`: the pending requirements its `Needs` name, for
    /// an obsolete one the pending ones a move took it to and, on its own component's line, its
    /// own ID while it has lines on other components, and for a capability the rules serving
    /// it that hold it back (`check.coverage/pending-report`). `waits` resolves each to its
    /// lines (`check.coverage/wait-lines`).
    pub waits_on: Vec<String>,
}

impl From<&Pending> for Requirement {
    fn from(p: &Pending) -> Self {
        Requirement {
            id: p.id.clone(),
            version: p.version.clone(),
            state: p.state.to_string(),
            cited: p.cited.clone(),
            markers: p.markers.iter().map(ToString::to_string).collect(),
            priority: p.priority,
            blocked_by: p.blocked_by.clone(),
            waits_on: p.waits_on.clone(),
        }
    }
}

/// Whether a backlog item or a wait blocks the pending line `p`.
fn is_blocked(p: &Pending) -> bool {
    !p.blocked_by.is_empty() || !p.waits_on.is_empty()
}

/// What blocks requirements of a node: a backlog item or a pending requirement; exactly one
/// of `item` and `requirement` is set, and `components` with `requirement` alone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    /// `backlog/<slug>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// `<node>/<slug>`, a pending requirement the blocked ones need.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
    /// For a requirement blocker, the components of the lines it stands for, sorted;
    /// `None` for an item blocker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<String>>,
    pub requirements: Vec<String>,
}

impl Block {
    /// The blocker's name: `backlog/<slug>` or `<node>/<slug>`.
    pub fn blocker(&self) -> &str {
        self.item
            .as_deref()
            .or(self.requirement.as_deref())
            .unwrap_or_default()
    }
}

/// A node's state, serialised with a `state` tag so new states pass through as they are.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum State {
    Converged,
    /// Its pending requirements, most urgent first, the blocked ones included and marked.
    Ready {
        pending: Vec<Requirement>,
    },
    /// Claimed until `lapses_at`, in Unix seconds.
    Claimed {
        lapses_at: u64,
    },
    /// Every pending requirement is blocked, by these items and needed requirements.
    Blocked {
        by: Vec<Block>,
    },
}

/// One node of the queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Node {
    pub node: String,
    /// Its effective priority (`priority-orders-nodes`).
    pub priority: Priority,
    #[serde(flatten)]
    pub state: State,
}

/// The queue: every node of the graph, sorted by ID, and the complete changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    pub nodes: Vec<Node>,
    /// IDs of the complete changes outside the archive, sorted (`archive-on-sync`).
    pub complete: Vec<String>,
}

impl Queue {
    /// What `clm next` prints: the `ready` nodes, in [`Queue::in_order`]'s order.
    pub fn next(&self) -> Vec<String> {
        self.in_order()
            .filter(|n| matches!(n.state, State::Ready { .. }))
            .map(|n| n.node.clone())
            .collect()
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.node == id)
    }

    /// The nodes by effective priority, then each with no pending requirement that waits
    /// before each that has one, then by ID: at equal priority the places go first to work
    /// that waits for nothing (`priority-orders-nodes`, `needs-alone-order`).
    fn in_order(&self) -> impl Iterator<Item = &Node> {
        let mut nodes: Vec<&Node> = self.nodes.iter().collect();
        nodes.sort_by_key(|n| (n.priority, has_wait(n), &n.node));
        nodes.into_iter()
    }
}

/// Whether one of `node`'s pending requirements waits: has an entry in its `waits_on`.
fn has_wait(node: &Node) -> bool {
    match &node.state {
        State::Ready { pending } => pending.iter().any(|p| !p.waits_on.is_empty()),
        _ => false,
    }
}

/// The queue for `graph` and the markers of its code. `refs` are the `impl/*` branches on
/// `origin`; one is a live claim while its tip is within `expiry_hours` of `now` and isn't
/// merged. A nested ref such as `impl/archive/<id>`, whose name contains `/`, claims no node
/// (`nested-branches-not-claims`); callers such as the Worker pass such refs unfiltered.
pub fn queue_from_refs(
    graph: &Graph,
    markers: &[Marker],
    refs: &[ClaimRef],
    now: u64,
    expiry_hours: u64,
) -> Queue {
    let live: BTreeMap<&str, u64> = refs
        .iter()
        .filter(|r| !r.node.contains('/') && !r.merged && !lapsed(r.tip_time, now, expiry_hours))
        .map(|r| {
            let lapses_at = r.tip_time.saturating_add(expiry_hours.saturating_mul(3600));
            (r.node.as_str(), lapses_at)
        })
        .collect();

    let lines = pending(graph, markers);
    let mut by_node: BTreeMap<&str, Vec<&Pending>> = graph
        .nodes
        .keys()
        .map(|id| (id.as_str(), Vec::new()))
        .collect();
    for p in &lines {
        if let Some(list) = by_node.get_mut(p.node.as_str()) {
            list.push(p);
        }
    }
    for list in by_node.values_mut() {
        list.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
    }
    let priorities = effective_priorities(&lines, &by_node);

    let nodes = graph
        .nodes
        .values()
        .map(|n| {
            let id = n.id.as_str();
            let pending = &by_node[id];
            let state = if let Some(&lapses_at) = live.get(id) {
                State::Claimed { lapses_at }
            } else if pending.is_empty() {
                State::Converged
            } else if pending.iter().all(|p| is_blocked(p)) {
                State::Blocked {
                    by: blocks(&lines, pending),
                }
            } else {
                State::Ready {
                    pending: pending.iter().map(|p| Requirement::from(*p)).collect(),
                }
            };
            Node {
                node: n.id.clone(),
                priority: priorities.get(id).copied().unwrap_or_default(),
                state,
            }
        })
        .collect();
    Queue {
        nodes,
        complete: complete_changes(graph, markers),
    }
}

/// What blocks `pending`, the lines of one node, each with the requirements it blocks: the
/// items sorted by slug, then the waited requirements sorted by ID, each with the components
/// of the lines `waits` gives for it (`check.coverage/wait-lines`), never the component its
/// ID names. `lines` is the whole list `pending` returned.
fn blocks(lines: &[Pending], pending: &[&Pending]) -> Vec<Block> {
    let mut by_item: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut by_wait: BTreeMap<&str, (BTreeSet<&str>, Vec<String>)> = BTreeMap::new();
    for p in pending {
        for slug in &p.blocked_by {
            by_item.entry(slug).or_default().push(p.id.clone());
        }
        let waited = waits(lines, p);
        for id in &p.waits_on {
            let (components, requirements) = by_wait.entry(id).or_default();
            components.extend(
                waited
                    .iter()
                    .filter(|w| w.id == *id)
                    .map(|w| w.node.as_str()),
            );
            requirements.push(p.id.clone());
        }
    }
    let items = by_item.into_iter().map(|(slug, requirements)| Block {
        item: Some(format!("backlog/{slug}")),
        requirement: None,
        components: None,
        requirements,
    });
    let waits = by_wait
        .into_iter()
        .map(|(id, (components, requirements))| Block {
            item: None,
            requirement: Some(id.to_string()),
            components: Some(components.into_iter().map(String::from).collect()),
            requirements,
        });
    items.chain(waits).collect()
}

/// Each node's effective priority (`priority-orders-nodes`): the highest among its unblocked
/// pending requirements, raised by every line that waits on one of its lines. A line blocked
/// by a wait seeds its priority on the node of each line `waits` gives for it, whatever that
/// node's state, never on the node a waited ID names, and a line waited on that waits in turn
/// passes it on, each line followed once by its node and ID together. `depends_on` carries
/// none.
fn effective_priorities(
    lines: &[Pending],
    by_node: &BTreeMap<&str, Vec<&Pending>>,
) -> BTreeMap<String, Priority> {
    let mut effective: BTreeMap<String, Priority> = by_node
        .iter()
        .filter_map(|(id, list)| {
            list.iter()
                .filter(|p| !is_blocked(p))
                .map(|p| p.priority)
                .min()
                .map(|p| (id.to_string(), p))
        })
        .collect();
    for p in by_node.values().flatten() {
        // The line itself is never seeded, so a blocked line lends nothing to its own node.
        let mut seen: BTreeSet<(&str, &str)> = BTreeSet::from([(p.node.as_str(), p.id.as_str())]);
        let mut stack = waits(lines, p);
        while let Some(w) = stack.pop() {
            if !seen.insert((w.node.as_str(), w.id.as_str())) {
                continue;
            }
            if by_node.contains_key(w.node.as_str()) {
                let e = effective.entry(w.node.clone()).or_insert(p.priority);
                *e = (*e).min(p.priority);
            }
            stack.extend(waits(lines, w));
        }
    }
    effective
}
