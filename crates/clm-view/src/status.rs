//! `clm status`: a summary, then the `components`, `changes` and `claims` sections laid
//! out with aligned columns (`status-report`).

use std::collections::{BTreeMap, BTreeSet};

use clm_check::{Marker, Row, State, report, status_report};
use clm_model::{BacklogKind, ChangeStatus, Graph, Layer, Status};
use clm_orchestrator::{ClaimRef, DEFAULT_EXPIRY_HOURS, claim_state, claims_section};

use crate::Checkout;
use crate::chart;
use crate::flight::{by_state, flight_view, status_name, status_rank};

/// The whole of `clm status` for `graph` and the markers of its code, with the `impl/*`
/// node claims and the `propose/*` backlog claims as of the last fetch. The summary's
/// pending and complete counts come from the `flight` view (`views-rendered-by-clm`).
/// `oversized` is the size report, as [`crate::spec_view`] takes it: each node it names gets
/// the cell `oversized`, and the `components` line their count (`oversized-is-a-state`).
/// `checkout`, the commit the command read, is
/// the summary's first line when there is one (`views-name-their-commit`).
pub fn status_text(
    graph: &Graph,
    markers: &[Marker],
    refs: &[ClaimRef],
    backlog_refs: &[ClaimRef],
    now: u64,
    oversized: Option<&BTreeSet<String>>,
    checkout: Option<&Checkout>,
) -> String {
    let sections = status_report(graph, markers);
    let flight = flight_view(graph, markers, refs, now, DEFAULT_EXPIRY_HOURS);
    let claims = claims_section(refs, backlog_refs, now);
    let mut text = String::from("CLM status\n");
    if let Some(c) = checkout {
        text.push_str("  ");
        text.push_str(&chart::to_text(&[c.line()], 0));
    }

    let active = count(graph.nodes.values(), |n| {
        n.frontmatter.status == Status::Active
    });
    let unbuilt = count(graph.nodes.values(), |n| {
        n.frontmatter.status == Status::Specd
    });
    let coverage = report(graph, markers);
    // An entry standing by a proof isn't a requirement to make current; its layer's own
    // counts follow (`human-layer-proof`).
    let cited: Vec<_> = coverage
        .iter()
        .filter(|c| c.proof.is_none() && c.state != State::Obsolete)
        .collect();
    let current = count(cited.iter(), |c| c.state == State::Current);
    let mut line = format!(
        "  components {}: {active} active, {unbuilt} unbuilt; {current}/{} requirements current",
        graph.nodes.len(),
        cited.len()
    );
    let architecture = graph
        .layers
        .get(&Layer::Architecture)
        .map(Vec::as_slice)
        .unwrap_or_default();
    if !architecture.is_empty() {
        let enforcing: Vec<_> = architecture
            .iter()
            .filter(|e| !e.enforced_by.is_empty())
            .collect();
        // An entry's line is `current` exactly when every rule it names is.
        let enforced = count(enforcing.iter(), |e| {
            coverage
                .iter()
                .any(|c| c.id == e.id && c.state == State::Current)
        });
        line.push_str(&format!(
            ", architecture {enforced}/{} enforced",
            enforcing.len()
        ));
    }
    let untested = count(graph.layers.values().flatten(), |e| e.untested);
    if untested > 0 {
        line.push_str(&format!(", {untested} untested"));
    }
    let is_oversized = |id: &str| oversized.is_some_and(|o| o.contains(id));
    let big = count(graph.nodes.keys(), |id| is_oversized(id));
    if big > 0 {
        line.push_str(&format!(", {big} oversized"));
    }
    text.push_str(&line);
    text.push('\n');

    let live: Vec<_> = graph.changes.values().filter(|c| !c.archived).collect();
    let mut by_status: Vec<(ChangeStatus, usize)> = Vec::new();
    for c in &live {
        match by_status
            .iter_mut()
            .find(|(s, _)| *s == c.frontmatter.status)
        {
            Some((_, n)) => *n += 1,
            None => by_status.push((c.frontmatter.status, 1)),
        }
    }
    by_status.sort_by_key(|(s, _)| status_rank(*s));
    let mut parts: Vec<String> = by_status
        .iter()
        .map(|(s, n)| format!("{n} {}", status_name(*s)))
        .collect();
    let complete = flight
        .changes
        .iter()
        .flat_map(|g| &g.changes)
        .filter(|c| c.complete)
        .count();
    if complete > 0 {
        parts.push(format!("{complete} complete"));
    }
    text.push_str(&format!("  changes {} live{}\n", live.len(), tail(&parts)));

    let states = by_state(&flight.totals.by_state);
    text.push_str(&format!(
        "  pending {} components{}\n",
        flight.totals.pending_nodes,
        tail(&if states.is_empty() {
            Vec::new()
        } else {
            vec![states]
        })
    ));

    let mut you: Vec<String> = graph
        .backlog
        .values()
        .filter(|b| {
            matches!(
                b.frontmatter.kind,
                BacklogKind::Question | BacklogKind::Human
            )
        })
        .map(|b| format!("backlog/{}", b.slug))
        .collect();
    you.extend(
        live.iter()
            .filter(|c| c.frontmatter.status.is_pending())
            .map(|c| format!("changes/{}", c.id)),
    );
    you.sort();
    text.push_str(&format!("  waiting on you {}{}\n", you.len(), tail(&you)));

    let mut kinds: BTreeMap<BacklogKind, usize> = BTreeMap::new();
    for b in graph.backlog.values() {
        *kinds.entry(b.frontmatter.kind).or_default() += 1;
    }
    let kinds: Vec<String> = kinds.iter().map(|(k, n)| format!("{n} {k}")).collect();
    text.push_str(&format!(
        "  backlog {}{}\n",
        graph.backlog.len(),
        tail(&kinds)
    ));

    let claim_count = |state: &str| {
        count(refs.iter().chain(backlog_refs), |r| {
            claim_state(r, now, DEFAULT_EXPIRY_HOURS) == state
        })
    };
    text.push_str(&format!("  claims {} live", claim_count("live")));
    for state in ["lapsed", "merged"] {
        let n = claim_count(state);
        if n > 0 {
            text.push_str(&format!(", {n} {state}"));
        }
    }
    text.push_str("\n\n");

    text.push_str("components\n");
    text.push_str(&section(
        sections
            .nodes
            .iter()
            .map(|r| {
                let mut r = r.clone();
                if is_oversized(&r[0]) {
                    r.push("oversized".into());
                }
                ("  ", r)
            })
            .collect(),
    ));
    text.push_str("\nchanges\n");
    if sections.changes.is_empty() {
        text.push_str("  none\n");
    }
    let mut rows = Vec::new();
    for (status, group) in &sections.changes {
        rows.push(("  ", vec![status.clone()]));
        rows.extend(group.iter().map(|r| ("    ", r.clone())));
    }
    text.push_str(&section(rows));
    text.push_str("\nclaims\n");
    text.push_str(&section(claims.iter().map(|r| ("  ", r.clone())).collect()));
    text.push('\n');
    text
}

fn count<T>(items: impl Iterator<Item = T>, f: impl Fn(&T) -> bool) -> usize {
    items.filter(|i| f(i)).count()
}

/// `: a, b` for a non-empty list, else nothing.
fn tail(parts: &[String]) -> String {
    if parts.is_empty() {
        String::new()
    } else {
        format!(": {}", parts.join(", "))
    }
}

/// Rows, each with its indent, as lines whose columns start at the same position: a cell's
/// column is its index after the indent, so group names sit in the first column.
fn section(rows: Vec<(&str, Row)>) -> String {
    let mut widths: Vec<usize> = Vec::new();
    for (indent, row) in &rows {
        for (i, cell) in row.iter().enumerate() {
            let w = if i == 0 { indent.len() } else { 0 } + cell.chars().count();
            if widths.len() <= i {
                widths.push(0);
            }
            widths[i] = widths[i].max(w);
        }
    }
    let mut out = String::new();
    for (indent, row) in &rows {
        let mut line = String::new();
        for (i, cell) in row.iter().enumerate() {
            if i == 0 {
                line.push_str(indent);
            }
            line.push_str(cell);
            if i + 1 < row.len() {
                let w = if i == 0 { indent.len() } else { 0 } + cell.chars().count();
                line.push_str(&" ".repeat(widths[i] - w + 2));
            }
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}
