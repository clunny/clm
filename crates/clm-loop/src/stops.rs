//! What the loop does with a run that returned without landing, by its cause (`stop-causes`,
//! `repeated-stops-escalate`).

use crate::state::{LoopFile, Pending, Run, RunState};

/// The causes a session may report, as the Contract lists them.
pub(crate) const CAUSES: [&str; 9] = [
    "limit", "disk", "outage", "tool", "work", "context", "blocked", "stopped", "running",
];

/// How long a `limit` with no known end waits before the run is resumed: a constant, not a
/// setting (`yagni`).
pub(crate) const LIMIT_RETRY_SECONDS: u64 = 30 * 60;

/// What a stop comes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    /// Resume the same subagent, from `after` on, or once the disk is cleared.
    Pause(Pending),
    /// Finish the run and launch a new subagent on the session's model.
    Rebuild,
    /// Finish the run and file a question for the human.
    Escalate,
    /// Finish the run and launch nothing.
    Close,
}

/// The verdict on `run`, whose stops already count the one now reported with `cause`.
/// `outstanding` tells that a resume the session was to deliver hasn't been reported on.
pub(crate) fn verdict(
    run: &Run,
    cause: &str,
    until: Option<u64>,
    now: u64,
    outstanding: bool,
) -> Verdict {
    let count = run.stops.get(cause).copied().unwrap_or(0);
    let of_the_work = matches!(cause, "outage" | "tool" | "work" | "context");
    let repeated = run.round >= 2 && run.rebuilt_for.as_deref() == Some(cause);
    if of_the_work && (repeated || (cause == "context" && count >= 2)) {
        return Verdict::Escalate;
    }
    // A resume needs the task to send it to.
    let pause = |cause: &str, after: Option<u64>| {
        if run.task_id.is_some() {
            Verdict::Pause(Pending {
                cause: cause.to_string(),
                after,
            })
        } else {
            Verdict::Rebuild
        }
    };
    match cause {
        "limit" => pause(cause, Some(until.unwrap_or(now + LIMIT_RETRY_SECONDS))),
        "disk" => pause(cause, None),
        "outage" | "tool" | "work" if count >= 2 || (cause == "tool" && outstanding) => {
            Verdict::Rebuild
        }
        "outage" | "tool" | "work" => pause(cause, Some(now)),
        "context" => Verdict::Rebuild,
        _ => Verdict::Close,
    }
}

/// Whether a pending resume may go out at `now`.
fn ready(file: &LoopFile, pending: &Pending, now: u64) -> bool {
    match pending.cause.as_str() {
        "disk" => !file.conditions.disk,
        _ => pending.after.is_none_or(|after| now >= after),
    }
}

/// Whether a stop's cause is one that stops a recurring run of runs from resuming together.
fn serial_cause(cause: &str) -> bool {
    matches!(cause, "limit" | "disk")
}

/// Whether the runs a recurring `limit` or `disk` stopped are still being resumed one at a time:
/// no new run launches meanwhile.
pub(crate) fn serial_hold(file: &LoopFile) -> bool {
    file.conditions.serial
        && file.runs.iter().any(|r| {
            r.resume.as_ref().is_some_and(|p| serial_cause(&p.cause))
                || (r.serial && r.state == RunState::Running)
        })
}

/// The runs whose resume goes out at `now`, in the order they stopped. Where a recurring stop
/// holds, one at a time, and none while a run resumed that way still runs.
pub(crate) fn resumable(file: &LoopFile, now: u64) -> Vec<String> {
    let mut out = Vec::new();
    let one_at_a_time = file.conditions.serial;
    let going = file
        .runs
        .iter()
        .any(|r| r.serial && r.state == RunState::Running);
    let mut serial_taken = going;
    for run in &file.runs {
        let Some(pending) = &run.resume else { continue };
        if run.state != RunState::Stopped || run.orphan || !ready(file, pending, now) {
            continue;
        }
        if one_at_a_time && serial_cause(&pending.cause) {
            if serial_taken {
                continue;
            }
            serial_taken = true;
        }
        out.push(run.name.clone());
    }
    out
}

/// The message a resume carries.
pub(crate) fn resume_message(run: &Run, pending: &Pending) -> String {
    let why = match pending.cause.as_str() {
        "limit" => "hit a usage limit",
        "disk" => "ran out of disk",
        "outage" => "was cut off by an outage",
        "tool" => "failed on a tool",
        _ => "stopped on its work",
    };
    format!(
        "Your run {} {why}. That has cleared. Carry on where you left off, in your own worktree. \
         Run again any review or check subagent of your own that the stop killed, then finish as \
         your prompt says.\n",
        run.name
    )
}
