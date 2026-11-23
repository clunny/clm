//! The `clm status` and `clm view` commands' reading of the repository, for `clm-view`'s
//! models and renderers: the working tree's graph and markers (or, with `--at`, a commit's),
//! the checkout, and the `impl/*` and `propose/*` remote-tracking refs as of the last fetch.
//! Neither command fetches.

mod at;
mod commands;

use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

pub use clm_check::Budgets;
use clm_check::Size;
use clm_coverage::Marker;
use clm_model::{Graph, Violation};
use clm_orchestrator::DEFAULT_EXPIRY_HOURS;
use clm_view::{
    Checkout, Format, Head, Name, View, blockers_view, flight_view, queue_states, render,
    spec_view, status_text,
};
pub use commands::{
    Env, NodeSize, PendingArgs, StatusArgs, ViewArgs, pending_command, status_command, view_command,
};

fn load(root: &Path) -> Result<(Graph, Vec<Marker>), Vec<Violation>> {
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        return Err(violations);
    }
    let markers = clm_coverage::scan(root).req;
    Ok((graph, markers))
}

/// The commit the working tree is on, and how many of `origin/main`'s commits it lacks, from
/// the local refs alone (`views-name-their-commit`). `None` when `HEAD` can't be read: no
/// repository, or no commit yet. `behind` is `None` without `refs/remotes/origin/main`;
/// when that ref exists but can't be compared, also `None`, with `checkout not compared:
/// <error>` on `err`, so a missing warning is never silent.
fn checkout(root: &Path, err: &mut dyn Write) -> Option<Checkout> {
    let main = format!("refs/remotes/{}/main", clm_git::REMOTE);
    let head = clm_git::git(root, &["rev-parse", "--verify", "HEAD"]).ok()?;
    // Cut to 7 rather than `--short`, which lengthens a hash to keep it unique.
    let commit = head.get(..7)?.to_string();
    let behind = match clm_git::git(root, &["rev-list", "--count", &format!("HEAD..{main}")]) {
        Ok(n) => n.parse().ok(),
        Err(e) => {
            // The usual failure is that there is no `origin/main`: nothing to say then.
            if clm_git::git(root, &["rev-parse", "--verify", "-q", &main]).is_ok() {
                let _ = writeln!(err, "checkout not compared: {e}");
            }
            None
        }
    };
    Some(Checkout { commit, behind })
}

/// The size report `clm status` and `clm view spec` mark: the IDs of the nodes over their
/// budgets as `clm size` measures them, with `defaults` for a node that sets no
/// `size_budget` (`oversized-is-a-state`). When the files can't be measured, `None`, with
/// `size not checked: <error>` on `err`.
fn oversized(
    root: &Path,
    graph: &Graph,
    markers: &[Marker],
    defaults: Budgets,
    err: &mut dyn Write,
) -> Option<BTreeSet<String>> {
    match clm_check::size(root, graph, markers, defaults) {
        Ok(sizes) => Some(
            sizes
                .into_iter()
                .filter(Size::oversized)
                .map(|s| s.node)
                .collect(),
        ),
        Err(e) => {
            let _ = writeln!(err, "size not checked: {e}");
            None
        }
    }
}

/// `clm status`, or the spec graph's violations. With `budgets`, the defaults of a node
/// that sets no `size_budget`, it marks the oversized nodes; without, it marks none. What it
/// has to say beside the summary goes to the process's stderr.
pub fn status(root: &Path, budgets: Option<Budgets>) -> Result<String, Vec<Violation>> {
    status_to(root, budgets, &mut std::io::stderr())
}

/// [`status`], saying what it has to say beside the summary on `err`.
fn status_to(
    root: &Path,
    budgets: Option<Budgets>,
    err: &mut dyn Write,
) -> Result<String, Vec<Violation>> {
    let (graph, markers) = load(root)?;
    let oversized = budgets.and_then(|b| oversized(root, &graph, &markers, b, err));
    let refs = clm_orchestrator::claim_refs(root);
    let backlog = clm_orchestrator::propose_refs(root);
    Ok(status_text(
        &graph,
        &markers,
        &refs,
        &backlog,
        clm_git::now(),
        oversized.as_ref(),
        checkout(root, err).as_ref(),
    ))
}

/// Why `clm view` printed no view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The spec graph's violations.
    Violations(Vec<Violation>),
    /// `--at` named no commit, or its tree couldn't be read: why, naming the revision.
    At(String),
}

/// The defaults of a node that sets no `size_budget`, from a commit's `clm.toml` text
/// (`None` when it has none), or why they can't be read.
pub type BudgetsFrom<'a> = &'a dyn Fn(Option<&str>) -> Result<Budgets, String>;

/// Where `clm view` reads the repository from.
pub enum Source<'a> {
    /// The working tree; with budgets, as [`status`] takes them, the `spec` view marks the
    /// oversized nodes.
    Tree(Option<Budgets>),
    /// The commit `rev` names (`view-at-rev`); with `budgets`, the `spec` view marks the
    /// nodes oversized by that commit's `clm.toml` alone.
    At {
        rev: &'a str,
        budgets: Option<BudgetsFrom<'a>>,
    },
}

/// What a view describes: the graph and markers, the oversized nodes and the head the chart
/// begins with, from the working tree or from a commit.
type Read = (Graph, Vec<Marker>, Option<BTreeSet<String>>, Option<Head>);

fn read_source(root: &Path, source: Source, err: &mut dyn Write) -> Result<Read, Error> {
    Ok(match source {
        Source::Tree(budgets) => {
            let (graph, markers) = load(root).map_err(Error::Violations)?;
            let sized = budgets.and_then(|b| oversized(root, &graph, &markers, b, err));
            (
                graph,
                markers,
                sized,
                checkout(root, err).map(Head::Checkout),
            )
        }
        Source::At { rev, budgets } => {
            let at = at::read(root, rev, budgets, err)?;
            if !at.violations.is_empty() {
                return Err(Error::Violations(at.violations));
            }
            (at.graph, at.markers, at.oversized, Some(Head::At(at.head)))
        }
    })
}

/// `clm view <name> --format <format> --width <width> [--at <rev>]`, or the spec graph's
/// violations, or a revision that couldn't be read; what it has to say beside the view goes
/// to `err`. `width` cuts the charts only.
pub fn view(
    root: &Path,
    name: Name,
    format: Format,
    width: usize,
    source: Source,
    err: &mut dyn Write,
) -> Result<String, Error> {
    let (graph, markers, oversized, head) = read_source(root, source, err)?;
    let (refs, now) = (clm_orchestrator::claim_refs(root), clm_git::now());
    let flight = || flight_view(&graph, &markers, &refs, now, DEFAULT_EXPIRY_HOURS);
    let view = match name {
        // The spec chart shows the flight model's queue states, so the two never disagree.
        Name::Spec => View::Spec(
            spec_view(&graph, &markers, oversized.as_ref()),
            queue_states(&flight()),
        ),
        Name::Flight => View::Flight(flight()),
        Name::Blockers => View::Blockers(blockers_view(
            &graph,
            &markers,
            &refs,
            now,
            DEFAULT_EXPIRY_HOURS,
        )),
    };
    Ok(render(&view, format, width, head.as_ref()))
}

/// `clm view queue --format <format> --width <width> [--at <rev>]` (`view-queue`): the plan
/// the build loop's last step stored, with its runs as the loop file holds them now, never
/// fetching and writing no file; the checkout line is the only thing `source` changes. With
/// no plan, the queue's ready components in priority order, read from `source`, each with a
/// reason saying why no plan is shown: no loop file, a loop file with no plan yet (the loop
/// line then as the file stands), or a loop file that can't be read (a loop not held, and
/// why on `err`). A stored plan's time and `origin/main` commit go on `err`, which the
/// plan's model has no place for.
pub fn queue(
    root: &Path,
    format: Format,
    width: usize,
    source: Source,
    err: &mut dyn Write,
) -> Result<String, Error> {
    let (graph, markers, _, head) = read_source(root, source, err)?;
    let now = clm_git::now();
    let (file, reason) = match clm_loop::try_read(root) {
        Ok(Some(file)) => (Some(file), "no plan stored yet"),
        Ok(None) => (None, "no loop has run on this clone"),
        Err(why) => {
            let _ = writeln!(err, "loop file not read: {why}");
            (None, "loop file not read")
        }
    };
    let stored = file
        .as_ref()
        .and_then(|f| f.plan.as_ref())
        .and_then(|stored| Some((stored, clm_loop::stored_plan(root)?)));
    let view = if let Some((stored, plan)) = stored {
        let minutes = now.saturating_sub(stored.at) / 60;
        let _ = writeln!(err, "plan computed {minutes}m ago at {}", stored.main);
        plan
    } else {
        let refs = clm_orchestrator::claim_refs(root);
        let queue =
            clm_orchestrator::queue_from_refs(&graph, &markers, &refs, now, DEFAULT_EXPIRY_HOURS);
        let next = queue
            .next()
            .into_iter()
            .filter_map(|id| queue.node(&id).map(|n| (id, n.priority)))
            .map(|(id, priority)| clm_view::Next {
                name: format!("build:{id}"),
                kind: "build".to_string(),
                priority: priority.to_string(),
                reason: reason.to_string(),
            })
            .collect();
        clm_view::QueueView {
            view: "queue",
            // A loop that has started but stored no plan yet is still held.
            loop_state: match &file {
                Some(f) if f.stop == Some(clm_loop::StopMode::Drain) => clm_view::Loop::Stop {
                    mode: clm_view::Stop::Drain,
                },
                Some(f) if f.stop == Some(clm_loop::StopMode::Now) => clm_view::Loop::Stop {
                    mode: clm_view::Stop::Now,
                },
                Some(f) if f.held(now) => clm_view::Loop::Held {
                    renewed_minutes: now.saturating_sub(f.lease.unwrap_or(now)) / 60,
                },
                _ => clm_view::Loop::NotHeld,
            },
            running: Vec::new(),
            next,
            waiting_on_you: Vec::new(),
            waiting_on_work: Vec::new(),
        }
    };
    Ok(render(&View::Queue(view), format, width, head.as_ref()))
}

/// The first line of a chart that begins with `head`, as plain text: the line the views
/// print, taken from `clm-view` rather than formatted again here (`pending-names-its-commit`).
fn head_text(head: Head) -> String {
    let none = flight_view(&Graph::default(), &[], &[], 0, DEFAULT_EXPIRY_HOURS);
    render(&View::Flight(none), Format::Text, usize::MAX, Some(&head))
        .lines()
        .next()
        .unwrap_or_default()
        .to_string()
}

/// What `clm pending` prints: the line naming the commit for stderr, and the pending lines.
type PendingReport = (Option<String>, Result<Vec<clm_coverage::Pending>, Error>);

/// `clm pending [<component>] [--at <rev>]` (`pending-names-its-commit`): the line naming
/// the commit it describes, in plain text (none when `HEAD` can't be read, or when `rev`
/// names no commit), and the pending lines of the working tree or of `rev`'s tree, or the
/// spec graph's violations, or why `rev` couldn't be read.
/// Anything else it has to say goes to `err`.
pub fn pending(
    root: &Path,
    component: Option<&str>,
    at: Option<&str>,
    err: &mut dyn Write,
) -> PendingReport {
    let Some(rev) = at else {
        let line = checkout(root, err).map(|c| head_text(Head::Checkout(c)));
        return (
            line,
            clm_check::pending_in(root, component).map_err(Error::Violations),
        );
    };
    match at::read(root, rev, None, err) {
        Err(e) => (None, Err(e)),
        Ok(commit) => {
            let line = head_text(Head::At(commit.head));
            if !commit.violations.is_empty() {
                return (Some(line), Err(Error::Violations(commit.violations)));
            }
            let list = clm_check::pending_for(&commit.graph, &commit.markers, component)
                .map_err(Error::Violations);
            (Some(line), list)
        }
    }
}
