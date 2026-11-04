//! The `flight` view: the work in flight, nodes with pending work and live changes.

use std::collections::BTreeMap;

use clm_check::{ChangeProgress, Marker, change_progress, pending};
use clm_model::{Change, ChangeStatus, Graph, Priority};
use clm_orchestrator::{Block, ClaimRef, Requirement, State, queue_from_refs};
use serde::Serialize;

use crate::chart::{self, Cell, Line, Style, heading, plain, span, word};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightView {
    /// Always `flight`.
    pub view: &'static str,
    pub totals: Totals,
    /// Every node that isn't converged, by effective priority then ID.
    pub nodes: Vec<FlightNode>,
    /// The live changes grouped by status: `implementing`, `applied`, `approved`,
    /// `proposed`, then any other; only groups with changes.
    pub changes: Vec<FlightGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Totals {
    /// Nodes that aren't converged.
    pub pending_nodes: usize,
    /// Those nodes by state.
    pub by_state: BTreeMap<String, usize>,
    pub live_changes: usize,
    /// Pending requirements over every node: unbuilt, behind or obsolete.
    pub requirements_pending: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightNode {
    pub node: String,
    /// Its effective priority (`priority-orders-nodes`).
    pub priority: Priority,
    /// `ready`, `claimed` or `blocked` (`needs-alone-order`).
    pub state: String,
    /// Its pending requirements, most urgent first.
    pub pending: Vec<Requirement>,
    /// Claimed only.
    pub claim: Option<Claim>,
    /// Blocked only.
    pub blocked_by: Vec<Block>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Claim {
    /// Unix seconds.
    pub lapses_at: u64,
    /// Hours since the claim's last push, to the nearest hour.
    pub pushed_hours_ago: u64,
    /// Hours until the claim lapses, to the nearest hour.
    pub lapses_in_hours: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightGroup {
    pub status: String,
    /// By priority, highest first, then ID.
    pub changes: Vec<FlightChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FlightChange {
    pub change: String,
    pub priority: Priority,
    /// Requirements its deltas add or modify that are current.
    pub current: usize,
    pub total: usize,
    /// Markers (`<file>:<line>`) still citing a requirement it removed.
    pub obsolete: Vec<String>,
    /// Complete (`archive-on-sync`): ready to archive.
    pub complete: bool,
}

/// The order change groups come in; any other status follows, sorted by name.
const GROUP_ORDER: [ChangeStatus; 4] = [
    ChangeStatus::Implementing,
    ChangeStatus::Applied,
    ChangeStatus::Approved,
    ChangeStatus::Proposed,
];

pub(crate) fn status_name(status: ChangeStatus) -> String {
    format!("{status:?}").to_lowercase()
}

/// A change status's place in [`GROUP_ORDER`], the rest after them by name.
pub(crate) fn status_rank(s: ChangeStatus) -> (usize, String) {
    GROUP_ORDER.iter().position(|g| *g == s).map_or_else(
        || (GROUP_ORDER.len(), status_name(s)),
        |i| (i, String::new()),
    )
}

/// The live changes, those `change_progress` gives (outside the archive, in ID order) whose
/// status is neither `done` nor `abandoned`, each with its progress: what the `flight` view
/// lists and the `spec` model's node detail names.
pub(crate) fn live_progress<'g>(
    graph: &'g Graph,
    markers: &[Marker],
) -> impl Iterator<Item = (&'g Change, ChangeProgress)> {
    change_progress(graph, markers)
        .into_iter()
        .filter_map(|p| graph.changes.get(&p.change).map(|c| (c, p)))
        .filter(|(c, _)| {
            !matches!(
                c.frontmatter.status,
                ChangeStatus::Done | ChangeStatus::Abandoned
            )
        })
}

fn hours(secs: u64) -> u64 {
    (secs + 1800) / 3600
}

/// The `flight` view: `refs` are the `impl/*` branches on `origin`, a live one being a
/// claim while its tip is within `expiry_hours` of `now` and isn't merged.
pub fn flight_view(
    graph: &Graph,
    markers: &[Marker],
    refs: &[ClaimRef],
    now: u64,
    expiry_hours: u64,
) -> FlightView {
    let queue = queue_from_refs(graph, markers, refs, now, expiry_hours);
    let mut by_node: BTreeMap<String, Vec<Requirement>> = BTreeMap::new();
    for p in pending(graph, markers) {
        by_node
            .entry(p.node.clone())
            .or_default()
            .push(Requirement::from(&p));
    }
    for list in by_node.values_mut() {
        list.sort_by(|a, b| (a.priority, &a.id).cmp(&(b.priority, &b.id)));
    }
    let requirements_pending = by_node.values().map(Vec::len).sum();

    let mut nodes: Vec<FlightNode> = queue
        .nodes
        .iter()
        .filter(|n| n.state != State::Converged)
        .map(|n| {
            let mut out = FlightNode {
                node: n.node.clone(),
                priority: n.priority,
                state: String::new(),
                pending: by_node.get(&n.node).cloned().unwrap_or_default(),
                claim: None,
                blocked_by: Vec::new(),
            };
            out.state = match &n.state {
                State::Converged => "converged",
                State::Ready { .. } => "ready",
                State::Claimed { lapses_at } => {
                    let tip = refs
                        .iter()
                        .filter(|r| r.node == n.node && !r.merged)
                        .map(|r| r.tip_time)
                        .max()
                        .unwrap_or(*lapses_at);
                    out.claim = Some(Claim {
                        lapses_at: *lapses_at,
                        pushed_hours_ago: hours(now.saturating_sub(tip)),
                        lapses_in_hours: hours(lapses_at.saturating_sub(now)),
                    });
                    "claimed"
                }
                State::Blocked { by } => {
                    out.blocked_by = by.clone();
                    "blocked"
                }
            }
            .to_string();
            out
        })
        .collect();
    nodes.sort_by(|a, b| (a.priority, &a.node).cmp(&(b.priority, &b.node)));

    let mut by_state: BTreeMap<String, usize> = BTreeMap::new();
    for n in &nodes {
        *by_state.entry(n.state.clone()).or_default() += 1;
    }

    // Each change exactly as `check.coverage/change-progress` gives it, so this view and
    // `clm status` never disagree about a change.
    let mut groups: Vec<(ChangeStatus, Vec<FlightChange>)> = Vec::new();
    for (c, p) in live_progress(graph, markers) {
        let status = c.frontmatter.status;
        let change = FlightChange {
            change: p.change,
            priority: c.frontmatter.priority,
            current: p.current,
            total: p.total,
            obsolete: p.obsolete,
            complete: p.complete,
        };
        match groups.iter_mut().find(|(s, _)| *s == status) {
            Some((_, list)) => list.push(change),
            None => groups.push((status, vec![change])),
        }
    }
    groups.sort_by_key(|(s, _)| status_rank(*s));
    let live_changes = groups.iter().map(|(_, l)| l.len()).sum();
    let changes = groups
        .into_iter()
        .map(|(s, mut list)| {
            list.sort_by(|a, b| (a.priority, &a.change).cmp(&(b.priority, &b.change)));
            FlightGroup {
                status: status_name(s),
                changes: list,
            }
        })
        .collect();

    FlightView {
        view: "flight",
        totals: Totals {
            pending_nodes: nodes.len(),
            by_state,
            live_changes,
            requirements_pending,
        },
        nodes,
        changes,
    }
}

const STATE_ORDER: [&str; 3] = ["claimed", "ready", "blocked"];

/// `<count> <state>` for each state in [`STATE_ORDER`] that has nodes, joined by `, `.
pub(crate) fn by_state(counts: &BTreeMap<String, usize>) -> String {
    STATE_ORDER
        .iter()
        .filter_map(|s| counts.get(*s).map(|n| format!("{n} {s}")))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The chart: a totals line, the pending nodes, then the live changes by group. The node
/// lines' shape is pinned (`flight-node-lines`): under `components`, exactly one line per pending
/// component starting `  <id> ` (or `  none`), then `changes`; the `clm-panel` plugin finds a
/// node's line by that prefix.
pub(crate) fn chart(view: &FlightView) -> Vec<Line> {
    let t = &view.totals;
    let mut summary = format!("{} components pending", t.pending_nodes);
    if t.pending_nodes > 0 {
        summary.push_str(&format!(" ({})", by_state(&t.by_state)));
    }
    summary.push_str(&format!(
        "  ·  {} changes live  ·  {} requirements to do",
        t.live_changes, t.requirements_pending
    ));
    let mut lines = vec![vec![
        span("flight", Style::Indigo),
        plain("  "),
        plain(summary),
    ]];
    lines.push(heading("components"));
    if view.nodes.is_empty() {
        lines.push(vec![plain("  none")]);
    }
    let rows: Vec<Vec<Cell>> = view.nodes.iter().map(node_row).collect();
    lines.extend(chart::align(rows));
    lines.push(heading("changes"));
    if view.changes.is_empty() {
        lines.push(vec![plain("  none")]);
    }
    for group in &view.changes {
        lines.push(vec![plain("  "), span(&group.status, Style::Indigo)]);
        let rows: Vec<Vec<Cell>> = group.changes.iter().map(change_row).collect();
        lines.extend(chart::align(rows));
    }
    lines
}

fn node_row(n: &FlightNode) -> Vec<Cell> {
    let mut bar = chart::pending_bar(n.pending.iter().map(|p| p.state.as_str()), 10);
    bar.push(plain(format!(" {} pending", n.pending.len())));
    let detail = match n.state.as_str() {
        "claimed" => n
            .claim
            .as_ref()
            .map(|c| {
                format!(
                    "pushed {}h ago, lapses in {}h",
                    c.pushed_hours_ago, c.lapses_in_hours
                )
            })
            .unwrap_or_default(),
        "blocked" => format!(
            "by {}",
            n.blocked_by
                .iter()
                .map(|b| b.blocker())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        _ => String::new(),
    };
    vec![
        vec![plain("  "), plain(&n.node)],
        vec![chart::priority(n.priority)],
        vec![word(&n.state)],
        bar,
        vec![plain(detail)],
    ]
}

fn change_row(c: &FlightChange) -> Vec<Cell> {
    let mut bar = chart::ratio_bar(c.current, c.total, 10);
    bar.push(plain(format!(" {}/{}", c.current, c.total)));
    let mut note = Vec::new();
    if c.complete {
        note.push(word("complete"));
    } else if !c.obsolete.is_empty() {
        note.push(span(format!("{} obsolete", c.obsolete.len()), Style::Rose));
    }
    vec![
        vec![plain("    "), plain(&c.change)],
        vec![chart::priority(c.priority)],
        bar,
        note,
    ]
}
