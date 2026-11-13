//! The summary a drain or a stop ends with (`end-summary`).

use clm_git::Failure;
use clm_view::QueueView;

use crate::Options;
use crate::state::{LoopFile, RunState};

/// Kind of the human's queue entry that holds work `clm wrap-up` found unsafe.
pub(crate) const UNSAFE_WORK: &str = "wrap-up";

/// The `workaround` items and `reconsider-<component>-<slug>` questions added on `origin/main`
/// since `from`, by slug.
fn added_flags(options: &Options, from: &str) -> Result<Vec<String>, Failure> {
    let added = clm_git::git(
        &options.root,
        &[
            "diff",
            "--name-only",
            "--diff-filter=A",
            from,
            "origin/main",
            "--",
            "backlog/",
        ],
    )?;
    Ok(added
        .lines()
        .filter_map(|path| path.strip_prefix("backlog/")?.strip_suffix(".md"))
        .filter(|slug| slug.starts_with("workaround-") || slug.starts_with("reconsider-"))
        .map(str::to_string)
        .collect())
}

/// What the loop did since `start`, what is still open, and whether the session can end.
pub(crate) fn summary(
    options: &Options,
    file: &LoopFile,
    view: &QueueView,
) -> Result<String, Failure> {
    let mut out = String::from("The build loop is done.\n\nLanded since the loop started:\n");
    let landed: Vec<_> = file
        .runs
        .iter()
        .filter(|r| r.state == RunState::Returned && r.landed)
        .collect();
    if landed.is_empty() {
        out.push_str("  nothing\n");
    }
    for run in landed {
        out.push_str(&format!("  {}\n", run.name));
    }
    out.push_str("\nStill running, stopped or held:\n");
    let open: Vec<_> = file
        .runs
        .iter()
        .filter(|r| r.state != RunState::Returned)
        .collect();
    if open.is_empty() {
        out.push_str("  nothing\n");
    }
    for run in open {
        let state = match run.state {
            RunState::Stopped => "stopped, its claim, branch and place kept",
            _ => "running",
        };
        out.push_str(&format!("  {}: {state}\n", run.name));
    }
    out.push_str("\nWaiting on you:\n");
    if view.waiting_on_you.is_empty() {
        out.push_str("  nothing\n");
    }
    for entry in &view.waiting_on_you {
        out.push_str(&format!(
            "  {}: {} ({})\n",
            entry.name, entry.what, entry.action
        ));
    }
    let flags = match &file.started_main {
        Some(from) => added_flags(options, from)?,
        None => Vec::new(),
    };
    if !flags.is_empty() {
        out.push_str("\nAdded since the loop started; see /clm:health:\n");
        for slug in flags {
            out.push_str(&format!("  {slug}\n"));
        }
    }
    if !view.waiting_on_you.iter().any(|e| e.kind == UNSAFE_WORK) {
        out.push_str("\nNothing unsafe is left in your work: the session can end.\n");
    }
    Ok(out)
}
