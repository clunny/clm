//! `clm status`'s `components` and `changes` sections (`status-report`). `view` lays them out.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

use clm_model::{Change, ChangeStatus, Graph, Layer};

use crate::coverage::{Marker, State, cited_entry, report};
use crate::pending::entries_node;

/// A row's cells.
pub type Row = Vec<String>;

/// The two sections `check.coverage` contributes to `clm status`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StatusReport {
    /// One row per component, sorted by ID: ID, status, `<current>/<total>`.
    pub nodes: Vec<Row>,
    /// Groups of changes outside the archive by status, in the order `implementing`,
    /// `applied`, `approved`, `proposed`, then any other sorted; each a row per change
    /// sorted by ID: ID, `<current>/<total> requirements`, and `complete` when it is: an
    /// `applied` or `implementing` change (`archive-on-sync`) whose added and modified
    /// requirements, each followed through the moves while the spec doesn't hold it, are all
    /// current, and that nothing cites a requirement it removed that the spec no longer
    /// holds.
    pub changes: Vec<(String, Vec<Row>)>,
}

impl fmt::Display for StatusReport {
    /// Each section's name, then its rows indented two spaces, or four under a group's
    /// name, itself indented two; cells separated by two spaces; `none` for no rows.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "components")?;
        if self.nodes.is_empty() {
            writeln!(f, "  none")?;
        }
        for row in &self.nodes {
            writeln!(f, "  {}", row.join("  "))?;
        }
        writeln!(f, "changes")?;
        if self.changes.is_empty() {
            writeln!(f, "  none")?;
        }
        for (status, rows) in &self.changes {
            writeln!(f, "  {status}")?;
            for row in rows {
                writeln!(f, "    {}", row.join("  "))?;
            }
        }
        Ok(())
    }
}

fn status_name(status: ChangeStatus) -> String {
    format!("{status:?}").to_lowercase()
}

/// The order groups come in; the rest follow, sorted.
const GROUP_ORDER: [ChangeStatus; 4] = [
    ChangeStatus::Implementing,
    ChangeStatus::Applied,
    ChangeStatus::Approved,
    ChangeStatus::Proposed,
];

/// One live change's progress (`change-progress`): the counts and the `complete` mark
/// `clm status` prints for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeProgress {
    pub change: String,
    /// Requirements its deltas add or modify, among those counted, that are current.
    pub current: usize,
    /// The requirements its deltas add or modify that are counted: for an `applied` or
    /// `implementing` change, those the spec still holds, each one it doesn't replaced by
    /// those moves took it to; for any other, all of them.
    pub total: usize,
    /// The markers (`<file>:<line>`, in the order of `markers`) citing a requirement its
    /// deltas remove that the spec no longer holds, or a capability it lists under
    /// `removes_capabilities` that `CAPABILITIES.md` no longer holds.
    pub obsolete: Vec<String>,
    /// `archive-on-sync`: `applied` or `implementing`, everything counted current, and
    /// nothing obsolete.
    pub complete: bool,
}

/// The progress of each change outside the archive, in ID order, whatever its status,
/// computed from one coverage report.
///
/// An `applied` or `implementing` change counts a requirement the spec no longer holds as
/// each one the moves the changes record took it to (`progress-follows-moves`), and leaves
/// it out when no move takes it to one the spec holds: a later change removed it. One that
/// hasn't applied counts all of them, and is never complete: its modified requirements are
/// still the old text. An applied one is complete when everything it counts is current and
/// no marker cites a requirement it removed that the spec no longer holds. One a later change added back is a new
/// requirement, so its markers, at any version, don't count.
pub fn change_progress(graph: &Graph, markers: &[Marker]) -> Vec<ChangeProgress> {
    progress(graph, markers, &states(graph, markers))
        .map(|(_, p)| p)
        .collect()
}

/// `change_progress` from a coverage state map already computed, each with its change.
fn progress<'g>(
    graph: &'g Graph,
    markers: &[Marker],
    states: &BTreeMap<String, State>,
) -> impl Iterator<Item = (&'g Change, ChangeProgress)> {
    let state = |id: &str| states.get(id).copied();
    let moves = moves(graph);
    graph
        .changes
        .values()
        .filter(|c| !c.archived)
        .map(move |c| {
            let applied = matches!(
                c.frontmatter.status,
                ChangeStatus::Applied | ChangeStatus::Implementing
            );
            let mut counted: Vec<String> = c
                .deltas
                .iter()
                .flat_map(|d| {
                    d.added
                        .iter()
                        .chain(&d.modified)
                        .map(move |r| format!("{}/{}", d.node, r.slug))
                })
                .flat_map(|id| {
                    if applied {
                        follow(graph, &moves, id)
                    } else {
                        vec![id]
                    }
                })
                .collect();
            counted.sort();
            counted.dedup();
            let current = counted
                .iter()
                .filter(|id| state(id) == Some(State::Current))
                .count();
            let removed: BTreeSet<String> = c
                .deltas
                .iter()
                .flat_map(|d| {
                    d.removed
                        .iter()
                        .map(move |r| format!("{}/{}", d.node, r.slug))
                })
                .chain(
                    c.frontmatter
                        .removes_capabilities
                        .iter()
                        .map(|slug| format!("capability/{slug}")),
                )
                .filter(|id| state(id) == Some(State::Obsolete))
                .collect();
            let obsolete: Vec<String> = if removed.is_empty() {
                Vec::new()
            } else {
                markers
                    .iter()
                    .filter(|m| removed.contains(&m.id))
                    .map(ToString::to_string)
                    .collect()
            };
            let progress = ChangeProgress {
                change: c.id.clone(),
                current,
                total: counted.len(),
                complete: applied && current == counted.len() && obsolete.is_empty(),
                obsolete,
            };
            (c, progress)
        })
}

/// The moves the graph's changes record (`progress-follows-moves`), from each moved ID to
/// the IDs it moved to: a change whose status is `applied`, `implementing` or `done`, in the
/// archive or not, moves `<a>/<slug>` to `<b>/<slug>` when one of its deltas removes the
/// first and another, for a node `<b>` other than `<a>`, adds the second, as a rename does.
pub(crate) fn moves(graph: &Graph) -> BTreeMap<String, BTreeSet<String>> {
    let mut moves: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let moving = graph.changes.values().filter(|c| {
        matches!(
            c.frontmatter.status,
            ChangeStatus::Applied | ChangeStatus::Implementing | ChangeStatus::Done
        )
    });
    for c in moving {
        for from in &c.deltas {
            for r in &from.removed {
                let to = c
                    .deltas
                    .iter()
                    .filter(|d| d.node != from.node && d.added.iter().any(|a| a.slug == r.slug));
                for d in to {
                    moves
                        .entry(format!("{}/{}", from.node, r.slug))
                        .or_default()
                        .insert(format!("{}/{}", d.node, r.slug));
                }
            }
        }
    }
    moves
}

/// The IDs the spec holds that `id` counts as: itself when the spec holds it, else those its
/// moves reach, followed breadth first through the IDs the spec doesn't hold, each once.
/// None when no move takes it to one the spec holds, since a later change removed it.
pub(crate) fn follow(
    graph: &Graph,
    moves: &BTreeMap<String, BTreeSet<String>>,
    id: String,
) -> Vec<String> {
    let holds = |id: &str| graph.requirement(id).is_some();
    if holds(&id) {
        return vec![id];
    }
    let mut held = Vec::new();
    let mut seen = BTreeSet::from([id.clone()]);
    let mut queue = VecDeque::from([id]);
    while let Some(id) = queue.pop_front() {
        if holds(&id) {
            held.push(id);
            continue;
        }
        for next in moves.get(&id).into_iter().flatten() {
            if seen.insert(next.clone()) {
                queue.push_back(next.clone());
            }
        }
    }
    held
}

/// Each requirement's coverage state by ID: every requirement the spec holds, then each
/// one a live change removed that a marker still cites, as `obsolete`.
fn states(graph: &Graph, markers: &[Marker]) -> BTreeMap<String, State> {
    report(graph, markers)
        .into_iter()
        .map(|c| (c.id, c.state))
        .collect()
}

/// The report for a loaded graph and the markers of every file.
pub fn status_report(graph: &Graph, markers: &[Marker]) -> StatusReport {
    let states = states(graph, markers);
    let current = |id: &str| states.get(id) == Some(&State::Current);
    // The capabilities that tests cite count on the row of the node they are pending on;
    // the architecture entries and untested capabilities on none (`human-layer-proof`).
    let entries_node = entries_node(graph);
    let capabilities: Vec<_> = graph
        .layers
        .get(&Layer::Capability)
        .into_iter()
        .flatten()
        .filter(|e| cited_entry(e))
        .collect();
    let nodes = graph
        .nodes
        .values()
        .map(|node| {
            let entries = capabilities
                .iter()
                .copied()
                .filter(|_| node.id == entries_node);
            let all: Vec<_> = node.requirements.iter().chain(entries).collect();
            let total = all.len();
            let done = all.iter().filter(|r| current(&r.id)).count();
            vec![
                node.id.clone(),
                format!("{:?}", node.frontmatter.status).to_lowercase(),
                format!("{done}/{total}"),
            ]
        })
        .collect();

    let mut groups: Vec<(ChangeStatus, Vec<Row>)> = Vec::new();
    for (c, progress) in progress(graph, markers, &states) {
        let status = c.frontmatter.status;
        let mut row = vec![
            progress.change,
            format!("{}/{} requirements", progress.current, progress.total),
        ];
        if progress.complete {
            row.push("complete".into());
        }
        match groups.iter_mut().find(|(s, _)| *s == status) {
            Some((_, rows)) => rows.push(row),
            None => groups.push((status, vec![row])),
        }
    }
    let rank = |s: ChangeStatus| {
        GROUP_ORDER.iter().position(|g| *g == s).map_or_else(
            || (GROUP_ORDER.len(), status_name(s)),
            |i| (i, String::new()),
        )
    };
    groups.sort_by_key(|(s, _)| rank(*s));
    StatusReport {
        nodes,
        changes: groups
            .into_iter()
            .map(|(s, rows)| (status_name(s), rows))
            .collect(),
    }
}

/// The IDs, sorted, of the complete changes outside the archive, as `change_progress` marks
/// them (`archive-on-sync`). The graph and the markers come from the same tree.
pub fn complete_changes(graph: &Graph, markers: &[Marker]) -> Vec<String> {
    change_progress(graph, markers)
        .into_iter()
        .filter(|p| p.complete)
        .map(|p| p.change)
        .collect()
}
