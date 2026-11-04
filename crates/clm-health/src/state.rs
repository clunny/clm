//! The signals of state other commands each show a part of: `check`, `pending`,
//! `oversized`, `lapsed-claim`, `stalled-change` and `unanswered` (`state-signals`).

use std::collections::BTreeMap;
use std::path::Path;

use clm_check::{Marker, Size, State};
use clm_model::{BacklogKind, ChangeStatus, Graph};

use crate::{Failure, Section, finding};

const SECS_PER_HOUR: u64 = 3600;
const SECS_PER_DAY: u64 = 86_400;

/// Every violation `clm_check::run` finds; no detectors run.
pub(crate) fn check(root: &Path) -> Section {
    let lines = clm_check::run(root)
        .iter()
        .map(|v| {
            finding(
                v.path.display(),
                "check",
                format!("{}: {}", v.rule, v.message),
            )
        })
        .collect();
    Section::of("check", "violations", lines)
}

/// Each node with pending requirements, with how many are in each pending state.
pub(crate) fn pending(graph: &Graph, markers: &[Marker]) -> Section {
    let mut counts: BTreeMap<&str, [usize; 3]> = BTreeMap::new();
    let list = clm_check::pending(graph, markers);
    for p in &list {
        let slot = match p.state {
            State::Unbuilt => 0,
            State::Behind => 1,
            State::Obsolete => 2,
            State::Current => continue,
        };
        counts.entry(p.node.as_str()).or_default()[slot] += 1;
    }
    let lines = counts
        .into_iter()
        .map(|(node, [s, b, o])| {
            // A removed node's obsolete markers have no `SPEC.md` of their own.
            let path = graph.nodes.get(node).map_or_else(
                || format!("specs/{}/SPEC.md", node.replace('.', "/")),
                |n| n.path.display().to_string(),
            );
            finding(
                path,
                "pending",
                format!(
                    "{} pending ({s} unbuilt, {b} behind, {o} obsolete)",
                    s + b + o
                ),
            )
        })
        .collect();
    Section::of("pending", "components", lines)
}

/// Each node over a size budget, with both counts (`oversized-is-a-state`).
pub(crate) fn oversized(graph: &Graph, sizes: &[Size]) -> Section {
    let lines = sizes
        .iter()
        .filter(|s| s.oversized())
        .map(|s| {
            finding(
                graph.nodes[&s.node].path.display(),
                "oversized",
                format!(
                    "spec {}/{}, code {}/{}",
                    s.spec.tokens, s.spec.budget, s.code.tokens, s.code.budget
                ),
            )
        })
        .collect();
    Section::of("oversized", "components", lines)
}

/// Each node and backlog claim on `origin`, as last fetched, whose tip is older than
/// `expiry` hours at `now` and not merged.
pub(crate) fn lapsed_claims(root: &Path, expiry: u64, now: u64) -> Result<Section, Failure> {
    const SIGNAL: &str = "lapsed-claim";
    let main = format!("refs/remotes/{}/main", clm_git::REMOTE);
    if clm_git::git(root, &["for-each-ref", "--format=%(refname)", &main])?.is_empty() {
        return Ok(Section::skipped(SIGNAL, "no origin".into()));
    }
    let impls = clm_orchestrator::claim_refs(root);
    let proposes = clm_orchestrator::propose_refs(root);
    let claims = impls
        .iter()
        .map(|r| ("impl", r))
        .chain(proposes.iter().map(|r| ("propose", r)));
    let lines = claims
        .filter(|(_, r)| clm_orchestrator::claim_state(r, now, expiry) == "lapsed")
        .map(|(prefix, r)| {
            let hours = now.saturating_sub(r.tip_time) / SECS_PER_HOUR;
            finding(
                format!("{prefix}/{}", r.node),
                SIGNAL,
                format!("last pushed {hours}h ago"),
            )
        })
        .collect();
    Ok(Section::of(SIGNAL, "claims", lines))
}

/// Each `applied` or `implementing` change outside the archive with requirements, none
/// of them current.
pub(crate) fn stalled_changes(graph: &Graph, markers: &[Marker]) -> Section {
    let lines = clm_check::change_progress(graph, markers)
        .into_iter()
        .filter(|p| p.current == 0 && p.total > 0)
        .filter_map(|p| {
            let change = &graph.changes[&p.change];
            let live = matches!(
                change.frontmatter.status,
                ChangeStatus::Applied | ChangeStatus::Implementing
            );
            (live && !change.archived).then(|| {
                finding(
                    change.path.display(),
                    "stalled-change",
                    format!("0/{} requirements current", p.total),
                )
            })
        })
        .collect();
    Section::of("stalled-change", "changes", lines)
}

/// Each `question` or `human-task` item whose file was added to git more than `stale_days`
/// days before `now`; an untracked item is skipped.
pub(crate) fn unanswered(
    root: &Path,
    graph: &Graph,
    stale_days: u64,
    now: u64,
) -> Result<Section, Failure> {
    let mut lines = Vec::new();
    // Without a commit no item was added to git, and `git log` fails.
    let items = if crate::has_history(root)? {
        graph.backlog.values()
    } else {
        Default::default()
    };
    for item in items {
        let kind = item.frontmatter.kind;
        if !matches!(kind, BacklogKind::Question | BacklogKind::Human) {
            continue;
        }
        let path = item.path.to_string_lossy();
        let log = clm_git::git(
            root,
            &["log", "--diff-filter=A", "--format=%ct", "--", &path],
        )?;
        let Some(added) = log.lines().filter_map(|l| l.parse::<u64>().ok()).min() else {
            continue;
        };
        let days = now.saturating_sub(added) / SECS_PER_DAY;
        if now.saturating_sub(added) > stale_days * SECS_PER_DAY {
            lines.push(finding(
                &path,
                "unanswered",
                format!("{kind} open {days} days"),
            ));
        }
    }
    Ok(Section::of("unanswered", "items", lines))
}
