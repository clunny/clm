//! Deterministic checks. Each violation's rule is the ID of the requirement it enforces.
//!
//! Without the default feature `checkout`, only what works on files already in memory is
//! built (`markers_in`, `report`, `pending`, `status_report`, `format`), so the crate builds
//! for `wasm32-unknown-unknown`.
//!
//! The coverage library is `check.coverage`'s crate `clm-coverage`, re-exported here
//! unchanged so callers keep their `clm_check::` imports (`check-coverage-split`).

#[cfg(feature = "checkout")]
mod assets;
mod attributes;
#[cfg(feature = "checkout")]
mod caps;
#[cfg(feature = "checkout")]
mod dangling;
#[cfg(feature = "detectors")]
mod detectors;
#[cfg(feature = "checkout")]
mod disjoint;
#[cfg(feature = "checkout")]
mod entries;
mod imports;
#[cfg(feature = "checkout")]
mod remove_by;
#[cfg(feature = "checkout")]
mod removed_names;
mod size;
#[cfg(feature = "checkout")]
mod skipped;
#[cfg(feature = "checkout")]
mod spans;
#[cfg(feature = "checkout")]
mod unenforced;
#[cfg(feature = "checkout")]
mod unowned;
#[cfg(feature = "checkout")]
mod untraced;

#[cfg(feature = "checkout")]
use std::path::Path;

pub use attributes::linguist_excluded;
pub use clm_coverage::{
    ChangeProgress, Coverage, Marker, Pending, Row, STATE_NAMES, State, StatusReport,
    change_progress, complete_changes, markers_in, matches, matches as path_matches, owner,
    pending, removed, report, status_report,
};
#[cfg(feature = "checkout")]
pub use clm_coverage::{Scan, scan};
pub use clm_model::Violation;
#[cfg(feature = "detectors")]
pub use detectors::{Detector, detectors};
pub use imports::undeclared_imports;
#[cfg(feature = "detectors")]
pub use size::size;
pub use size::{Budgets, Count, Length, Size, TestPart, size_of};

/// Runs every rule against the repo at `root`.
#[cfg(feature = "checkout")]
pub fn run(root: &Path) -> Vec<Violation> {
    let (graph, mut violations) = clm_model::load(root);
    for change in graph.changes.values() {
        if change.frontmatter.status.is_pending() {
            violations.extend(clm_apply::validate(&graph, change));
            // Only a change that removes a requirement can leave a stale name.
            let removes = change.deltas.iter().any(|d| !d.removed.is_empty());
            if removes && let Some(after) = clm_apply::resulting(&graph, change) {
                violations.extend(removed_names::would_remove(&graph, &after, change));
            }
        }
    }
    violations.extend(archive_violations(&graph));
    violations.extend(assets::violations(&graph));
    violations.extend(caps::violations(&graph));
    violations.extend(entries::violations(&graph));
    violations.extend(untraced::violations(&graph));
    violations.extend(unenforced::violations(&graph));
    let removed = removed(&graph);
    violations.extend(removed_names::violations(&graph, &removed));
    // Listed once for both rules; outside a git work tree the listing fails.
    let tracked = clm_git::tracked_files(root);
    violations.extend(disjoint::violations(
        tracked.as_deref().unwrap_or_default(),
        &graph,
    ));
    violations.extend(unowned::violations(root, &graph, &tracked));
    let markers = scan(root);
    violations.extend(dangling::violations(&graph, &markers.req, &removed));
    violations.extend(remove_by::violations(&graph, &markers.remove_by));
    violations.extend(skipped::violations(root, &markers.req));
    violations
}

/// Finished changes must be archived, and only finished changes may be.
#[cfg(feature = "checkout")]
fn archive_violations(graph: &clm_model::Graph) -> Vec<Violation> {
    use clm_model::ChangeStatus::{Abandoned, Done};
    graph
        .changes
        .values()
        .filter_map(|c| {
            let finished = matches!(c.frontmatter.status, Done | Abandoned);
            let message = match (finished, c.archived) {
                (true, false) => format!(
                    "change `{}` is {:?}; run `clm archive {}`",
                    c.id, c.frontmatter.status, c.id
                ),
                (false, true) => format!(
                    "archived change `{}` is {:?}; only done or abandoned changes belong in the archive",
                    c.id, c.frontmatter.status
                ),
                _ => return None,
            };
            Some(Violation {
                path: c.path.clone(),
                rule: "check/archive-done",
                message,
            })
        })
        .collect()
}

/// Backlog items sorted by priority, then kind, then slug. `Err` carries spec violations.
#[cfg(feature = "checkout")]
pub fn backlog(root: &Path) -> Result<Vec<clm_model::BacklogItem>, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    let mut items: Vec<_> = graph.backlog.into_values().collect();
    items.sort_by(|a, b| {
        let key = |i: &clm_model::BacklogItem| (i.frontmatter.priority, i.frontmatter.kind);
        (key(a), &a.slug).cmp(&(key(b), &b.slug))
    });
    Ok(items)
}

/// `clm status`'s `nodes` and `changes` sections for the checkout at `root`. `Err` carries
/// spec violations.
#[cfg(feature = "checkout")]
pub fn status(root: &Path) -> Result<StatusReport, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(status_report(&graph, &scan(root).req))
}

/// The pending requirements of every node, or of `node`, for the checkout at `root`.
/// `Err` carries spec violations, or one naming an unknown `node`.
#[cfg(feature = "checkout")]
pub fn pending_in(root: &Path, node: Option<&str>) -> Result<Vec<Pending>, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    pending_for(&graph, &scan(root).req, node)
}

/// The pending requirements of every component, or of `component`, for a graph and markers
/// loaded from anywhere (`pending-narrows-on-a-graph`). A `component` the graph doesn't hold
/// is the violation `pending_in` returns for it.
pub fn pending_for(
    graph: &clm_model::Graph,
    markers: &[Marker],
    component: Option<&str>,
) -> Result<Vec<Pending>, Vec<Violation>> {
    let mut list = pending(graph, markers);
    if let Some(node) = component {
        // A removed node's obsolete markers are pending on the nodes owning their files.
        if !graph.nodes.contains_key(node) {
            return Err(vec![Violation {
                path: std::path::Path::new("specs")
                    .join(node.replace('.', "/"))
                    .join("SPEC.md"),
                rule: "check.coverage/pending-report",
                message: format!("unknown component `{node}`"),
            }]);
        }
        list.retain(|p| p.node == node);
    }
    Ok(list)
}

/// Every requirement's coverage, sorted by ID. `Err` carries spec violations that
/// make the report unreliable.
#[cfg(feature = "checkout")]
pub fn coverage(root: &Path) -> Result<Vec<Coverage>, Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    Ok(report(&graph, &scan(root).req))
}

/// `<path>: <rule>: <message>`
pub fn format(v: &Violation) -> String {
    format!("{}: {}: {}", v.path.display(), v.rule, v.message)
}
