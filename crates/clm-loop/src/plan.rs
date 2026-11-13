//! The plan: a pure function from what a step read to `view`'s queue model and the launches
//! the step carries out (`plan-snapshot`, `plan-order`).

use std::collections::BTreeMap;

use clm_view::{Loop, Next, QueueView, Running, Stop, WaitingOnYou};

use crate::state::{Kind, LoopFile, RunState, StopMode, StoredHuman, StoredNext, StoredPlan};

/// One `group` line of `clm triage`: items to draft as one change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub key: String,
    /// `P1` to `P5`.
    pub priority: String,
    pub items: Vec<String>,
}

/// A backlog item on `origin/main` the loop's plan reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BacklogRow {
    pub slug: String,
    /// `question`, `human-task` or another kind.
    pub kind: String,
    pub blocks: Vec<String>,
    /// The commits of red verify results the item took in.
    pub verify_failures: Vec<String>,
    /// When the commit that added it was made, Unix seconds.
    pub since: u64,
}

/// What a step read after its fetch.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub now: u64,
    /// The `origin/main` commit.
    pub main: String,
    /// `max_builds` less the places that count.
    pub free_places: u64,
    /// The components `clm next` printed, in its order.
    pub ready: Vec<String>,
    /// Each component's effective priority, from `clm view flight`.
    pub priorities: BTreeMap<String, String>,
    /// The `group` lines of `clm triage`, in its order.
    pub groups: Vec<Group>,
    /// The model builds launch on.
    pub build_model: String,
    /// The agent harness has no subagents: the loop launches one run at a time.
    pub one_at_a_time: bool,
    pub file: LoopFile,
    /// The backlog items of `origin/main` that block requirements or took in a red verify.
    pub backlog: Vec<BacklogRow>,
    /// The lines of `clm wrap-up` about the caller's own unsafe work and the ledger's findings.
    pub unsafe_work: Vec<String>,
    /// The migrations `clm upgrade` lists, by ID.
    pub upgrades: Vec<String>,
    /// The changes `clm wrap-up` names `complete`, in its order.
    pub complete: Vec<String>,
    /// The IDs of the `impl/archive/*` branches on `origin`.
    pub archive_branches: Vec<String>,
}

/// What the step does for one entry of the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// Relaunch an orphan run, by its name.
    Relaunch(String),
    Build {
        component: String,
        model: String,
    },
    Draft {
        key: String,
        items: Vec<String>,
        model: String,
    },
    /// Archive the complete change `id` on `impl/archive/<id>`; it takes no place.
    Archive {
        id: String,
        model: String,
    },
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub view: QueueView,
    pub launches: Vec<Launch>,
    /// The next-up entries, as the loop file stores them.
    pub(crate) next: Vec<StoredNext>,
    /// The human's queue, oldest first.
    pub(crate) humans: Vec<StoredHuman>,
}

/// `P2` as 2; a priority this can't read counts as the lowest, `P5`.
fn rank(priority: &str) -> u8 {
    priority
        .strip_prefix('P')
        .and_then(|n| n.parse().ok())
        .unwrap_or(5)
}

const LAUNCHES_NOW: &str = "launches now";
const WAITS_FOR_A_PLACE: &str = "waits for a place";

/// The queue model for `file` at `now`, with `next` as its next-up section.
pub(crate) fn view(
    file: &LoopFile,
    now: u64,
    next: &[StoredNext],
    humans: &[StoredHuman],
) -> QueueView {
    let loop_state = match file.stop {
        Some(StopMode::Drain) => Loop::Stop { mode: Stop::Drain },
        Some(StopMode::Now) => Loop::Stop { mode: Stop::Now },
        None => match file.lease {
            Some(renewed) if file.held(now) => Loop::Held {
                renewed_minutes: now.saturating_sub(renewed) / 60,
            },
            _ => Loop::NotHeld,
        },
    };
    let running = file
        .runs
        .iter()
        .filter(|r| r.state != RunState::Returned)
        .map(|r| Running {
            name: r.name.clone(),
            kind: r.kind.map_or("run", Kind::word).to_string(),
            model: r.model.clone(),
            round: r.round,
            minutes: now.saturating_sub(r.started) / 60,
            taken_over: false,
        })
        .collect();
    QueueView {
        view: "queue",
        loop_state,
        running,
        next: next
            .iter()
            .map(|n| Next {
                name: n.name.clone(),
                kind: n.kind.word().to_string(),
                priority: n.priority.clone(),
                reason: n.reason.clone(),
            })
            .collect(),
        waiting_on_you: humans
            .iter()
            .map(|h| WaitingOnYou {
                name: h.name.clone(),
                kind: h.kind.clone(),
                what: h.what.clone(),
                since_minutes: now.saturating_sub(h.since) / 60,
                action: h.action.clone(),
            })
            .collect(),
        waiting_on_work: Vec::new(),
    }
}

/// Whether a run still holds its component or items: any run that hasn't returned.
fn active(file: &LoopFile, name: &str) -> bool {
    file.runs
        .iter()
        .any(|r| r.name == name && r.state != RunState::Returned)
}

/// The change to archive now: the first complete one, when every `impl/archive/*` branch on
/// `origin` is held by a run of the loop and that change's run isn't already going
/// (`plan-archive`).
fn archive_due(s: &Snapshot) -> Option<String> {
    let id = s.complete.first()?;
    let held = |id: &str| active(&s.file, &format!("archive:{id}"));
    (s.archive_branches.iter().all(|b| held(b)) && !held(id)).then(|| id.clone())
}

/// The plan for a snapshot: the orphans to relaunch, then, up to the free places, the builds
/// and drafts in one list sorted by priority, a draft before a build at equal priority, each
/// keeping its own command's order otherwise (`plan-order`).
pub fn plan_of(s: &Snapshot) -> Plan {
    let file = &s.file;
    let mut launches: Vec<Launch> = file
        .runs
        .iter()
        .filter(|r| r.orphan && file.stop != Some(StopMode::Now))
        .map(|r| Launch::Relaunch(r.name.clone()))
        .collect();
    let mut next: Vec<StoredNext> = Vec::new();
    if file.stop.is_none() {
        let held_items: Vec<&String> = file
            .runs
            .iter()
            .filter(|r| r.kind == Some(Kind::Draft) && r.state != RunState::Returned)
            .flat_map(|r| &r.items)
            .chain(&file.drafted)
            .collect();
        // (priority, is a build, entry, launch), then a stable sort keeps each list's order.
        let mut candidates: Vec<(u8, bool, StoredNext, Launch)> = Vec::new();
        for group in &s.groups {
            if group.items.iter().any(|i| held_items.contains(&i)) {
                continue;
            }
            candidates.push((
                rank(&group.priority),
                false,
                StoredNext {
                    name: format!("draft:{}", group.key),
                    kind: Kind::Draft,
                    priority: group.priority.clone(),
                    reason: String::new(),
                },
                Launch::Draft {
                    key: group.key.clone(),
                    items: group.items.clone(),
                    model: file.session_model.clone(),
                },
            ));
        }
        for component in &s.ready {
            let name = format!("build:{component}");
            if active(file, &name) {
                continue;
            }
            let priority = s
                .priorities
                .get(component)
                .cloned()
                .unwrap_or_else(|| "P5".to_string());
            candidates.push((
                rank(&priority),
                true,
                StoredNext {
                    name,
                    kind: Kind::Build,
                    priority,
                    reason: String::new(),
                },
                Launch::Build {
                    component: component.clone(),
                    model: s.build_model.clone(),
                },
            ));
        }
        if let Some(id) = archive_due(s) {
            next.push(StoredNext {
                name: format!("archive:{id}"),
                kind: Kind::Archive,
                priority: "-".to_string(),
                reason: LAUNCHES_NOW.to_string(),
            });
            launches.push(Launch::Archive {
                id,
                model: s.build_model.clone(),
            });
        }
        candidates.sort_by_key(|(rank, build, ..)| (*rank, *build));
        let busy = file
            .runs
            .iter()
            .filter(|r| matches!(r.state, RunState::Planned | RunState::Running))
            .count() as u64;
        let places = if s.one_at_a_time {
            s.free_places.min(1u64.saturating_sub(busy))
        } else {
            s.free_places
        };
        for (i, (_, _, mut entry, launch)) in candidates.into_iter().enumerate() {
            if (i as u64) < places {
                entry.reason = LAUNCHES_NOW.to_string();
                launches.push(launch);
            } else {
                entry.reason = WAITS_FOR_A_PLACE.to_string();
            }
            next.push(entry);
        }
    }
    let humans = crate::humans::queue(s);
    Plan {
        view: view(file, s.now, &next, &humans),
        launches,
        next,
        humans,
    }
}

/// The queue model for a snapshot: what the step acts on (`plan-snapshot`).
pub fn plan(snapshot: &Snapshot) -> QueueView {
    plan_of(snapshot).view
}

/// The plan the clone's last step stored, with its runs as the loop file holds them now, read
/// without fetching; none when the clone has no loop file or no step has stored a plan.
pub fn stored_plan(root: &std::path::Path) -> Option<QueueView> {
    let file = crate::state::read(root)?;
    let StoredPlan { next, humans, .. } = file.plan.clone()?;
    Some(view(&file, clm_git::now(), &next, &humans))
}
