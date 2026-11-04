//! `clm pending`: the requirements whose code hasn't caught up with their spec
//! (`pending-report`, `versioned-markers`, `requirement-blocks`, `requirement-needs`).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use std::path::Path;

use clm_model::{Graph, Layer, Priority, ROOT_ID};

use crate::coverage::{Coverage, Marker, State, report};
use crate::glob;
use crate::status::{follow, moves};

/// One pending requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pending {
    /// `<node-id>/<slug>`, or a human-layer entry's `<layer>/<slug>`.
    pub id: String,
    pub node: String,
    /// Its current version; `None` when it's obsolete.
    pub version: Option<String>,
    /// `Behind`, `Unbuilt` or `Obsolete`.
    pub state: State,
    /// Behind only: the distinct versions its markers cite, sorted, `none` for a bare marker.
    pub cited: Vec<String>,
    /// The markers citing it.
    pub markers: Vec<Marker>,
    /// The highest priority among the changes outside the archive whose deltas add, modify
    /// or remove it; `P5` when none does.
    pub priority: Priority,
    /// Slugs of the backlog items whose `blocks` name it, sorted.
    pub blocked_by: Vec<String>,
    /// Every other requirement it waits on, as `<node>/<slug>`, sorted: the pending ones its
    /// `Needs` name (`requirement-needs`); for an obsolete requirement, the pending
    /// requirements a move took it to (`moved-markers-wait`); for a capability, the `unbuilt`
    /// rules that serve it (`serving-rules-order-capabilities`).
    pub waits_on: Vec<String>,
}

impl fmt::Display for Pending {
    /// `<id> <version> unbuilt`, `<id> <version> behind <cited>[,<cited>...]` or
    /// `<id> obsolete <file>:<line>[ ...]`, then ` blocked <blocker>[,...]` when blocked:
    /// the `backlog/<slug>` items first, then the pending needs.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.state {
            State::Obsolete => {
                write!(f, "{} obsolete", self.id)?;
                for m in &self.markers {
                    write!(f, " {m}")?;
                }
            }
            State::Behind => write!(
                f,
                "{} {} behind {}",
                self.id,
                self.version.as_deref().unwrap_or_default(),
                self.cited.join(",")
            )?,
            _ => write!(
                f,
                "{} {} {}",
                self.id,
                self.version.as_deref().unwrap_or_default(),
                self.state
            )?,
        }
        let blockers: Vec<String> = self
            .blocked_by
            .iter()
            .map(|s| format!("backlog/{s}"))
            .chain(self.waits_on.iter().cloned())
            .collect();
        if !blockers.is_empty() {
            write!(f, " blocked {}", blockers.join(","))?;
        }
        Ok(())
    }
}

/// Every pending requirement of every node, sorted by node ID then requirement ID.
pub fn pending(graph: &Graph, markers: &[Marker]) -> Vec<Pending> {
    let mut priorities: BTreeMap<String, Priority> = BTreeMap::new();
    for c in graph.changes.values().filter(|c| !c.archived) {
        for d in &c.deltas {
            for r in d.added.iter().chain(&d.modified).chain(&d.removed) {
                let p = priorities
                    .entry(format!("{}/{}", d.node, r.slug))
                    .or_insert(Priority::P5);
                *p = (*p).min(c.frontmatter.priority);
            }
        }
    }
    let mut blocks: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for item in graph.backlog.values() {
        for id in &item.frontmatter.blocks {
            blocks.entry(id).or_default().push(item.slug.clone());
        }
    }

    let coverage: Vec<_> = report(graph, markers)
        .into_iter()
        .filter(Coverage::is_pending)
        .collect();
    let pending_ids: BTreeSet<String> = coverage.iter().map(|c| c.id.clone()).collect();
    let moves = moves(graph);
    // The `unbuilt` rules that serve each pending capability, by one pass
    // (`serving-rules-order-capabilities`).
    let mut serving: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for c in coverage.iter().filter(|c| c.state == State::Unbuilt) {
        for cap in graph.effective_serves(&c.id) {
            if pending_ids.contains(cap) {
                serving.entry(cap).or_default().push(c.id.clone());
            }
        }
    }
    // What a requirement waits on: its pending `Needs`; for an `unbuilt` or `behind`
    // capability, the `unbuilt` rules that serve it; and, when it is obsolete, the pending
    // requirements a move took it to (`moved-markers-wait`).
    let waits_on = |id: &str, state: State| {
        let mut needs: BTreeSet<String> = graph
            .requirement(id)
            .into_iter()
            .flat_map(|r| &r.needs)
            .filter(|n| pending_ids.contains(*n))
            .cloned()
            .collect();
        if let Some(rules) = serving.get(id).filter(|_| state != State::Obsolete) {
            needs.extend(rules.iter().cloned());
        }
        if state == State::Obsolete {
            needs.extend(
                follow(graph, &moves, id.to_string())
                    .into_iter()
                    .filter(|n| pending_ids.contains(n)),
            );
        }
        needs.into_iter().collect()
    };

    let mut out: Vec<Pending> = coverage
        .into_iter()
        .flat_map(|c| by_owner(graph, c))
        .map(|(node, c)| {
            let mut cited = Vec::new();
            if c.state == State::Behind {
                cited.extend(
                    c.listed()
                        .map(|m| m.version.clone().unwrap_or_else(|| "none".into())),
                );
                cited.sort();
                cited.dedup();
            }
            let mut blocked_by = blocks.get(c.id.as_str()).cloned().unwrap_or_default();
            blocked_by.sort();
            Pending {
                node,
                cited,
                priority: priorities.get(&c.id).copied().unwrap_or_default(),
                blocked_by,
                waits_on: waits_on(&c.id, c.state),
                id: c.id,
                version: c.version,
                state: c.state,
                markers: c.markers,
            }
        })
        .collect();
    out.sort_by(|a, b| (&a.node, &a.id).cmp(&(&b.node, &b.id)));
    out
}

/// The node each requirement's line is pending on: `entries_node` for a capability that
/// still exists (no other entry is ever pending); the node its ID names; or, for an obsolete requirement whose node
/// no longer exists (`node-removal`), the node owning each marker's file, one entry per
/// owner holding only that owner's markers.
fn by_owner(graph: &Graph, c: Coverage) -> Vec<(String, Coverage)> {
    if Layer::of(&c.id).is_some() && c.state != State::Obsolete {
        return vec![(entries_node(graph).to_string(), c)];
    }
    let node = c.id.split_once('/').map_or("", |(n, _)| n);
    if c.state != State::Obsolete || graph.nodes.contains_key(node) {
        return vec![(node.to_string(), c)];
    }
    let mut owned: BTreeMap<&str, Vec<Marker>> = BTreeMap::new();
    for m in c.markers {
        owned
            .entry(owner(graph, &m.path).unwrap_or(ROOT_ID))
            .or_default()
            .push(m);
    }
    owned
        .into_iter()
        .map(|(node, markers)| {
            let part = Coverage {
                id: c.id.clone(),
                state: c.state,
                version: c.version.clone(),
                markers,
                proof: None,
            };
            (node.to_string(), part)
        })
        .collect()
}

/// The node the capabilities are pending on, which builds their tests: the model's
/// `Graph::capability_tests_node` (`pending-report`, `model/capability-tests-node`).
pub(crate) fn entries_node(graph: &Graph) -> &str {
    graph.capability_tests_node()
}

/// The node whose `paths` hold the repository-relative `path`: the deepest such node, then
/// the first by ID; `None` when no node's `paths` do (`owner`).
pub fn owner<'g>(graph: &'g Graph, path: &Path) -> Option<&'g str> {
    let path = path.to_string_lossy();
    let depth = |id: &str| {
        if id == ROOT_ID {
            0
        } else {
            id.split('.').count()
        }
    };
    graph
        .nodes
        .values()
        .filter(|n| n.frontmatter.paths.iter().any(|p| glob::matches(p, &path)))
        .map(|n| n.id.as_str())
        .min_by_key(|id| (std::cmp::Reverse(depth(id)), *id))
}
