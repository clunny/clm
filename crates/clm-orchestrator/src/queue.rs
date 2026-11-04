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
//!   requirements waiting on its pending ones (a `Needs`, a move, a serving rule), through nodes of any state
//!   (`priority-orders-nodes`).

use std::collections::{BTreeMap, BTreeSet};

use clm_check::{Marker, Pending, complete_changes, pending};
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
    /// an obsolete one the pending ones a move took it to, and for a capability the `unbuilt`
    /// rules serving it (`check.coverage/pending-report`).
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

impl Requirement {
    /// Whether a backlog item or a pending requirement it needs blocks it.
    fn is_blocked(&self) -> bool {
        !self.blocked_by.is_empty() || !self.waits_on.is_empty()
    }
}

/// What blocks requirements of a node: a backlog item or a pending requirement of another
/// node; exactly one of `item` and `requirement` is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Block {
    /// `backlog/<slug>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<String>,
    /// `<node>/<slug>`, a pending requirement the blocked ones need.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<String>,
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
    /// What `clm next` prints: the `ready` nodes, by effective priority then ID.
    pub fn next(&self) -> Vec<String> {
        self.in_order()
            .filter(|n| matches!(n.state, State::Ready { .. }))
            .map(|n| n.node.clone())
            .collect()
    }

    pub fn node(&self, id: &str) -> Option<&Node> {
        self.nodes.iter().find(|n| n.node == id)
    }

    fn in_order(&self) -> impl Iterator<Item = &Node> {
        let mut nodes: Vec<&Node> = self.nodes.iter().collect();
        nodes.sort_by(|a, b| (a.priority, &a.node).cmp(&(b.priority, &b.node)));
        nodes.into_iter()
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

    let mut by_node: BTreeMap<&str, Vec<Requirement>> = graph
        .nodes
        .keys()
        .map(|id| (id.as_str(), Vec::new()))
        .collect();
    for p in pending(graph, markers) {
        if let Some(list) = by_node.get_mut(p.node.as_str()) {
            list.push(Requirement::from(&p));
        }
    }
    for list in by_node.values_mut() {
        list.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
    }
    let priorities = effective_priorities(&by_node);

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
            } else if pending.iter().all(Requirement::is_blocked) {
                State::Blocked {
                    by: blocks(pending),
                }
            } else {
                State::Ready {
                    pending: pending.clone(),
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

/// What blocks `pending`, each with the requirements it blocks: the items sorted by slug,
/// then the needed requirements sorted by ID.
fn blocks(pending: &[Requirement]) -> Vec<Block> {
    let mut by_item: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut by_need: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for p in pending {
        for slug in &p.blocked_by {
            by_item.entry(slug).or_default().push(p.id.clone());
        }
        for need in &p.waits_on {
            by_need.entry(need).or_default().push(p.id.clone());
        }
    }
    let items = by_item.into_iter().map(|(slug, requirements)| Block {
        item: Some(format!("backlog/{slug}")),
        requirement: None,
        requirements,
    });
    let needs = by_need.into_iter().map(|(need, requirements)| Block {
        item: None,
        requirement: Some(need.to_string()),
        requirements,
    });
    items.chain(needs).collect()
}

/// Each node's effective priority (`priority-orders-nodes`): its own, the highest among its
/// unblocked pending requirements, raised by every requirement that waits on one of its pending
/// ones (a `Needs`, a move, or for a capability a serving rule). A requirement blocked by a wait seeds its priority on the node of each pending
/// requirement it waits on, whatever that node's state, and one waited on that is itself
/// blocked by a wait passes it on. `depends_on` carries none.
fn effective_priorities(by_node: &BTreeMap<&str, Vec<Requirement>>) -> BTreeMap<String, Priority> {
    let requirements: BTreeMap<&str, &Requirement> = by_node
        .values()
        .flatten()
        .map(|p| (p.id.as_str(), p))
        .collect();
    let mut effective: BTreeMap<String, Priority> = by_node
        .iter()
        .filter_map(|(id, list)| {
            list.iter()
                .filter(|p| !p.is_blocked())
                .map(|p| p.priority)
                .min()
                .map(|p| (id.to_string(), p))
        })
        .collect();
    for p in by_node.values().flatten() {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<&str> = p.waits_on.iter().map(String::as_str).collect();
        while let Some(need) = stack.pop() {
            if !seen.insert(need) {
                continue;
            }
            let Some(node) = need.split_once('/').map(|(n, _)| n) else {
                continue;
            };
            if !by_node.contains_key(node) {
                continue;
            }
            let e = effective.entry(node.to_string()).or_insert(p.priority);
            *e = (*e).min(p.priority);
            if let Some(needed) = requirements.get(need) {
                stack.extend(needed.waits_on.iter().map(String::as_str));
            }
        }
    }
    effective
}
