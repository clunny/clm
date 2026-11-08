//! `verify-due`: whether a trigger has fired since the last verified commit.

use clm_config::{VerifyMode, VerifyTrigger};
use clm_ledger::Body;

use crate::chain::State;
use crate::{Ctx, Error, Outcome, import, run, tree};

const DAY: u64 = 24 * 60 * 60;

pub(crate) fn due(
    ctx: &Ctx,
    state: &State,
    work_done: bool,
    dry_run: bool,
) -> Result<Outcome, Error> {
    let head = state.chain.len() - 1;
    let (since, lands) = state.unverified_lands();
    let not_due =
        |why: String| Outcome::new(format!("verify not-due {why}"), if dry_run { 3 } else { 0 });
    if lands.is_empty() {
        let at = ctx.short(&state.chain[head])?;
        return Ok(not_due(format!("nothing landed since {at}")));
    }
    let Some(trigger) = trigger(ctx, state, since, lands, work_done)? else {
        let n = lands.len();
        return Ok(not_due(format!("no trigger holds for {n} lands")));
    };
    if dry_run {
        return Ok(Outcome::new(format!("verify due {trigger}"), 0));
    }
    match ctx.verify.mode {
        VerifyMode::Import => import::import_head(ctx, state),
        VerifyMode::Local | VerifyMode::Pipeline => run::verify_at(ctx, state, head),
    }
}

/// The first trigger that holds over the unverified `lands`, after the last verified commit
/// at position `since`.
fn trigger(
    ctx: &Ctx,
    state: &State,
    since: Option<usize>,
    lands: &[String],
    work_done: bool,
) -> Result<Option<&'static str>, Error> {
    let red = since
        .and_then(|s| state.newest(&state.chain[s]))
        .is_some_and(|r| matches!(r.body, Body::Verified { passed: false, .. }));
    if red {
        return Ok(Some("red-again"));
    }
    let wants = |t: VerifyTrigger| ctx.verify.triggers.contains(&t);
    if wants(VerifyTrigger::ChangeDone) && finishes_a_change(ctx, lands)? {
        return Ok(Some("change-done"));
    }
    if wants(VerifyTrigger::QueueDrained)
        && queue_drained(ctx, &state.chain[state.chain.len() - 1])?
    {
        return Ok(Some("queue-drained"));
    }
    // Settings allow the trigger only in `local` mode.
    if work_done && wants(VerifyTrigger::WorkDone) {
        return Ok(Some("work-done"));
    }
    if ctx
        .verify
        .every_lands
        .is_some_and(|n| lands.len() as u64 >= n)
    {
        return Ok(Some("every.lands"));
    }
    if let Some(days) = ctx.verify.every_days {
        let oldest = ctx.git(&["log", "-1", "--format=%ct", &lands[0]])?;
        let committed: u64 = oldest.trim().parse().unwrap_or(u64::MAX);
        if clm_git::now().saturating_sub(committed) >= days.saturating_mul(DAY) {
            return Ok(Some("every.days"));
        }
    }
    Ok(None)
}

/// Whether one of `lands` adds a `changes/archive/<id>/change.md`.
fn finishes_a_change(ctx: &Ctx, lands: &[String]) -> Result<bool, Error> {
    let archive = format!("{}changes/archive/", ctx.prefix);
    for land in lands {
        let parents = ctx.git(&["rev-list", "--parents", "-n", "1", land])?;
        let added = match parents.split_whitespace().nth(1) {
            Some(parent) => {
                ctx.git(&["diff", "--name-only", "--diff-filter=A", "-z", parent, land])?
            }
            None => ctx.git(&[
                "diff-tree",
                "--root",
                "-r",
                "--no-commit-id",
                "--name-only",
                "--diff-filter=A",
                "-z",
                land,
            ])?,
        };
        let is_change = |path: &str| {
            path.strip_prefix(&archive)
                .and_then(|rest| rest.strip_suffix("/change.md"))
                .is_some_and(|id| !id.is_empty() && !id.contains('/'))
        };
        if added.split('\0').any(is_change) {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether no requirement in `head`'s tree is pending.
fn queue_drained(ctx: &Ctx, head: &str) -> Result<bool, Error> {
    let tree = tree::Tree::at(ctx, head)?;
    let (graph, violations) = clm_model::load(&tree.dir);
    // Specs that do not load leave the queue unknown, not drained.
    if !violations.is_empty() {
        return Ok(false);
    }
    let markers = clm_coverage::scan(&tree.dir).req;
    Ok(clm_coverage::pending(&graph, &markers).is_empty())
}
