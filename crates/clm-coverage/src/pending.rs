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
    /// requirements a move took it to (`moved-markers-wait`), but not one on this line's own
    /// node, and its own ID on its own node's line while a line on another node is left; for
    /// a capability, the rules that serve it and hold it back
    /// (`serving-rules-order-capabilities`).
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
    for c in graph.changes.values().filter(|c| !c.archived) {
        for slug in &c.frontmatter.removes_capabilities {
            let p = priorities
                .entry(format!("capability/{slug}"))
                .or_insert(Priority::P5);
            *p = (*p).min(c.frontmatter.priority);
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
    // What a requirement waits on besides its serving rules: its pending `Needs`, and, when it
    // is obsolete, the pending requirements a move took it to (`moved-markers-wait`).
    let base_waits = |id: &str, state: State| {
        let mut needs: BTreeSet<String> = graph
            .requirement(id)
            .into_iter()
            .flat_map(|r| &r.needs)
            .filter(|n| pending_ids.contains(*n))
            .cloned()
            .collect();
        if state == State::Obsolete {
            needs.extend(
                follow(graph, &moves, id.to_string())
                    .into_iter()
                    .filter(|n| pending_ids.contains(n)),
            );
        }
        needs
    };
    // The rules that hold back each pending capability, by one pass over the requirements
    // (`serving-rules-order-capabilities`): the `unbuilt` ones, and the `behind` ones that an
    // item or a wait blocks. A rule's own blockers never include a capability's wait, so they
    // are known first.
    let mut serving: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for c in &coverage {
        let holds_back = match c.state {
            State::Unbuilt => true,
            State::Behind => {
                blocks.contains_key(c.id.as_str()) || !base_waits(&c.id, c.state).is_empty()
            }
            _ => false,
        };
        if !holds_back {
            continue;
        }
        for cap in graph.effective_serves(&c.id) {
            if pending_ids.contains(cap) {
                serving.entry(cap).or_default().push(c.id.clone());
            }
        }
    }
    // What a line pending on `node` waits on: the above, less a move's target on `node`
    // itself, whose one build makes it current; for an `unbuilt` or `behind` capability the
    // rules that hold it back; and the requirement's own ID when the line stands for its
    // lines on other components (`pending-report`).
    let waits_on = |id: &str, state: State, node: &str, waits_self: bool| {
        let mut needs = base_waits(id, state);
        if state == State::Obsolete {
            needs.retain(|n| n.split_once('/').map(|(n, _)| n) != Some(node));
        }
        if let Some(rules) = serving.get(id).filter(|_| state != State::Obsolete) {
            needs.extend(rules.iter().cloned());
        }
        if waits_self {
            needs.insert(id.to_string());
        }
        needs.into_iter().collect::<Vec<_>>()
    };

    let mut out: Vec<Pending> = coverage
        .into_iter()
        .flat_map(|c| by_owner(graph, c))
        .map(
            |Line {
                 node,
                 c,
                 waits_self,
             }| {
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
                    priority: priorities.get(&c.id).copied().unwrap_or_default(),
                    blocked_by,
                    waits_on: waits_on(&c.id, c.state, &node, waits_self),
                    node,
                    cited,
                    id: c.id,
                    version: c.version,
                    state: c.state,
                    markers: c.markers,
                }
            },
        )
        .collect();
    out.sort_by(|a, b| (&a.node, &a.id).cmp(&(&b.node, &b.id)));
    out
}

/// One line of a pending requirement: the node it is pending on, the coverage it lists, and
/// whether it waits on the requirement's own ID, standing for its lines on other nodes.
struct Line {
    node: String,
    c: Coverage,
    waits_self: bool,
}

/// The lines each requirement is pending on: on `entries_node` for a capability that still
/// exists (no other entry is ever pending); on the node its ID names; but for an obsolete
/// requirement, one line per node its markers' files take it to, each holding only that
/// node's markers. Whose node no longer exists (`node-removal`), each file's owner takes it,
/// or the root. Whose node the graph holds (a removed capability's `capability` included,
/// `removed-capabilities`), that node takes every file while the root sets no `wiring`;
/// once it does, each file's owner takes it, but a file a `wiring` glob matches or no node
/// owns is the requirement's own node's, and the own node's line waits on the ID while
/// another node's line is left (`pending-report`).
fn by_owner(graph: &Graph, c: Coverage) -> Vec<Line> {
    let whole = |node: &str, c| {
        vec![Line {
            node: node.to_string(),
            c,
            waits_self: false,
        }]
    };
    if Layer::of(&c.id).is_some() && c.state != State::Obsolete {
        return whole(entries_node(graph), c);
    }
    let node = c.id.split_once('/').map_or("", |(n, _)| n).to_string();
    if c.state != State::Obsolete {
        return whole(&node, c);
    }
    let mut lines = if graph.nodes.contains_key(node.as_str()) {
        let Some(wiring) = graph.wiring() else {
            return whole(&node, c);
        };
        split(c, |path| {
            let wired = wiring.iter().any(|w| glob::matches(w, path));
            let owned = owner(graph, Path::new(path)).filter(|_| !wired);
            owned.unwrap_or(&node).to_string()
        })
    } else {
        split(c, |path| {
            owner(graph, Path::new(path)).unwrap_or(ROOT_ID).to_string()
        })
    };
    if lines.len() > 1
        && let Some(own) = lines.iter_mut().find(|l| l.node == node)
    {
        own.waits_self = true;
    }
    lines
}

/// `c`'s markers grouped by the node `place` gives each one's file, one line per node, sorted
/// by node.
fn split(c: Coverage, place: impl Fn(&str) -> String) -> Vec<Line> {
    let mut placed: BTreeMap<String, Vec<Marker>> = BTreeMap::new();
    for m in c.markers {
        placed
            .entry(place(&m.path.to_string_lossy()))
            .or_default()
            .push(m);
    }
    placed
        .into_iter()
        .map(|(node, markers)| Line {
            node,
            c: Coverage {
                id: c.id.clone(),
                state: c.state,
                version: c.version.clone(),
                markers,
                proof: None,
            },
            waits_self: false,
        })
        .collect()
}

/// The node the capabilities are pending on, which builds their tests: the model's
/// `Graph::capability_tests_node` (`pending-report`, `model/capability-tests-node`).
pub(crate) fn entries_node(graph: &Graph) -> &str {
    graph.capability_tests_node()
}

/// The node whose `tests` match the repository-relative `path` (the first by ID), else the
/// node whose `paths` hold it (the deepest, then the first by ID); `None` when no node's
/// `tests` or `paths` do (`owner`).
pub fn owner<'g>(graph: &'g Graph, path: &Path) -> Option<&'g str> {
    let path = &*path.to_string_lossy();
    let holding = |globs: fn(&clm_model::Node) -> &[String]| {
        graph
            .nodes
            .values()
            .filter(move |n| globs(n).iter().any(|p| glob::matches(p, path)))
            .map(|n| n.id.as_str())
    };
    let depth = |id: &str| {
        if id == ROOT_ID {
            0
        } else {
            id.split('.').count()
        }
    };
    holding(|n| &n.frontmatter.tests).min().or_else(|| {
        holding(|n| &n.frontmatter.paths).min_by_key(|id| (std::cmp::Reverse(depth(id)), *id))
    })
}
