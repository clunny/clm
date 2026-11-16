//! `clm build wait`: sleeping until a step would act (`build-wait`).

use std::thread::sleep;
use std::time::{Duration, Instant};

use clm_git::{Failure, now};

use crate::Options;
use crate::plan::{Snapshot, WAITS_FOR_DISK, plan_of};
use crate::state::{self, LoopFile, RunState, StopMode};
use crate::step::{fresh_snapshot, gone_builds, held_places};

/// How often the loop file and the places are read, which needs no network.
const POLL: Duration = Duration::from_secs(5);

/// The longest interval, in minutes, which stays inside the lease's 15 minutes.
const LONGEST: f64 = 7.0;

/// Why a step would act now, or none. `disk_held` records that the disk held work back at some
/// read of this wait, so the line that wakes it once the disk's room grows names the disk.
fn reason(
    options: &Options,
    snapshot: &mut Snapshot,
    first: &LoopFile,
    first_places: &[String],
    disk_held: &mut bool,
) -> Result<Option<String>, Failure> {
    let file = state::read(&options.root).unwrap_or_default();
    let places = held_places(options)?;
    snapshot.free_places = options.max_builds.saturating_sub(places.len() as u64);
    snapshot.now = now();
    snapshot.file = file.clone();
    let idle = !file
        .runs
        .iter()
        .any(|r| matches!(r.state, RunState::Planned | RunState::Running));
    // A stop is news when it is new, or when it was asked for earlier and nothing runs now,
    // so the step ends the loop.
    if let Some(mode) = file.stop
        && (first.stop != Some(mode) || idle)
    {
        let word = match mode {
            StopMode::Drain => "drain",
            StopMode::Now => "now",
        };
        return Ok(Some(format!("stop requested: {word}")));
    }
    // The next step finishes it, and finds its local branch only if no housekeeping ran first.
    if let Some(name) = gone_builds(options, &file).first() {
        return Ok(Some(format!("{name}'s claim has left origin")));
    }
    if first.conditions.disk && !file.conditions.disk {
        return Ok(Some("the disk is cleared".to_string()));
    }
    if let Some(until) = first.conditions.limit_until
        && snapshot.now >= until
    {
        return Ok(Some("the limit has lifted".to_string()));
    }
    // A held orphan whose branch has stayed still long enough is for the next step to release.
    if file.runs.iter().any(|r| {
        r.hold
            .as_ref()
            .is_some_and(|h| snapshot.now.saturating_sub(h.since) >= 15 * 60)
    }) {
        return Ok(Some("a held orphan's branch has stayed still".to_string()));
    }
    let plan = plan_of(snapshot);
    if plan.launches.is_empty() {
        *disk_held |= plan.next.iter().any(|n| n.reason == WAITS_FOR_DISK);
        return Ok(None);
    }
    let freed: Vec<&String> = first_places
        .iter()
        .filter(|p| !places.contains(p))
        .collect();
    let names = plan
        .next
        .iter()
        .filter(|n| n.reason == "launches now")
        .map(|n| n.name.clone())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Some(if freed.is_empty() && *disk_held {
        format!("the disk has room, and work is ready: {names}")
    } else if freed.is_empty() {
        format!("work is ready: {names}")
    } else {
        format!(
            "place {} freed, and work is ready: {names}",
            freed
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }))
}

/// Why the wait ends before a run of the housekeeping, which would remove the local branch the
/// step that finishes a landed build looks for (`build-wait`).
fn claim_left(options: &Options) -> Option<String> {
    let file = state::read(&options.root)?;
    let name = gone_builds(options, &file).into_iter().next()?;
    Some(format!("{name}'s claim has left origin"))
}

/// `clm build wait [--interval <minutes>]`: renews the lease every interval and returns the line
/// naming why it woke, as soon as a step would act.
pub fn wait(options: &Options, interval_minutes: f64) -> Result<String, Failure> {
    if !(interval_minutes > 0.0 && interval_minutes <= LONGEST) {
        return Err(Failure::Refused(format!(
            "--interval is a positive number of minutes of at most {LONGEST}"
        )));
    }
    let interval = Duration::from_secs_f64(interval_minutes * 60.0);
    let renew = || {
        state::update(&options.root, |f| {
            f.lease = Some(now());
            Ok(f.clone())
        })
    };
    let first = renew()?;
    let first_places = held_places(options)?;
    if let Some(why) = claim_left(options) {
        return Ok(why);
    }
    let mut snapshot = fresh_snapshot(options)?;
    let mut fetched = Instant::now();
    let mut disk_held = false;
    loop {
        if let Some(why) = reason(
            options,
            &mut snapshot,
            &first,
            &first_places,
            &mut disk_held,
        )? {
            return Ok(why);
        }
        sleep(POLL);
        if fetched.elapsed() >= interval {
            renew()?;
            if let Err(e) = clm_git::fetch(&options.root) {
                eprintln!("clm build wait: fetch failed: {e}");
            }
            if let Some(why) = claim_left(options) {
                return Ok(why);
            }
            snapshot = fresh_snapshot(options)?;
            fetched = Instant::now();
        }
    }
}
