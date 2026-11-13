//! `clm build`: the clone's build loop (specs/orchestrator/loop/SPEC.md). Every decision is
//! code here; the session carries out the actions it prints with the agent harness's subagents.

use std::path::PathBuf;

mod cmd;
mod humans;
mod plan;
mod returned;
mod state;
mod step;
mod summary;
mod wait;

pub use plan::{BacklogRow, Group, Launch, Plan, Snapshot, plan, plan_of, stored_plan};
pub use state::{Batch, Escalation, Kind, LoopFile, Run, RunState, StopMode, read, update};

pub use returned::returned;
pub use step::{launched, start, step, stop};
pub use wait::wait;

/// What `clm build` was given.
pub struct Options {
    /// The program run as `clm`: `crates/clm` passes its own executable.
    pub clm: PathBuf,
    /// The CLM repository the loop builds; every subprocess runs in it.
    pub root: PathBuf,
    /// The directory `clm build` runs in, where `clm wrap-up` looks at the caller's own work.
    pub cwd: PathBuf,
    /// `config`'s `max_builds`: how many builds and drafts may hold a place at once.
    pub max_builds: u64,
    /// The agent harness the session runs under, whose abilities the loop follows.
    pub agent_harness: clm_agent_harness::AgentHarness,
}
