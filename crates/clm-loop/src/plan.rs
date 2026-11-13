//! The plan: a pure function from what a step read to `view`'s queue model and the launches
//! the step carries out (`plan-snapshot`, `plan-order`).

use std::collections::BTreeMap;

use clm_view::{Loop, Next, QueueView, Running, Stop};

use crate::state::{Kind, LoopFile, RunState, StopMode, StoredNext, StoredPlan};

/// One `group` line of `clm triage`: items to draft as one change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub key: String,
    /// `P1` to `P5`.
    pub priority: String,
    pub items: Vec<String>,
}

/// What a step read after its fetch.
#[derive(Debug, Clone)]
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
    pub file: LoopFile,
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
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub view: QueueView,
    pub launches: Vec<Launch>,
    /// The next-up entries, as the loop file stores them.
    pub(crate) next: Vec<StoredNext>,
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
pub(crate) fn view(file: &LoopFile, now: u64, next: &[StoredNext]) -> QueueView {
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
        waiting_on_you: Vec::new(),
        waiting_on_work: Vec::new(),
    }
}

/// Whether a run still holds its component or items: any run that hasn't returned.
fn active(file: &LoopFile, name: &str) -> bool {
    file.runs
        .iter()
        .any(|r| r.name == name && r.state != RunState::Returned)
}

/// The plan for a snapshot: the orphans to relaunch, then, up to the free places, the builds
/// and drafts in one list sorted by priority, a draft before a build at equal priority, each
/// keeping its own command's order otherwise (`plan-order`).
pub fn plan_of(s: &Snapshot) -> Plan {
    let file = &s.file;
    let mut launches: Vec<Launch> = file
        .runs
        .iter()
        .filter(|r| r.orphan)
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
        candidates.sort_by_key(|(rank, build, ..)| (*rank, *build));
        for (i, (_, _, mut entry, launch)) in candidates.into_iter().enumerate() {
            if (i as u64) < s.free_places {
                entry.reason = LAUNCHES_NOW.to_string();
                launches.push(launch);
            } else {
                entry.reason = WAITS_FOR_A_PLACE.to_string();
            }
            next.push(entry);
        }
    }
    Plan {
        view: view(file, s.now, &next),
        launches,
        next,
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
    let StoredPlan { next, .. } = file.plan.clone()?;
    Some(view(&file, clm_git::now(), &next))
}
