//! Orchestrator: the node queue and claims (see specs/orchestrator/SPEC.md).
//!
//! The queue ([`queue_from_refs`]) and what holds backlog items ([`taken_from_refs`]) need
//! neither git nor a checkout. Everything that does (the `next`, `claim` and `proposals`
//! commands, `taken_items`, and the `claims` rows of `clm status`) sits behind the default
//! feature `git`, so with default features off the crate builds for
//! `wasm32-unknown-unknown` (`orchestrator-builds-for-wasm`).

#[cfg(feature = "git")]
mod places;
#[cfg(feature = "git")]
mod propose;
mod queue;
#[cfg(feature = "git")]
mod repo;
mod taken;

#[cfg(feature = "git")]
pub use places::{
    Place, PlaceFile, PlaceKind, Placed, SweepOutcome, Swept, free_place, free_stale_places,
    place_slot, places, rename_place, take_place,
};
#[cfg(feature = "git")]
pub use propose::{Proposals, proposals, taken_items};
pub use queue::{
    Block, ClaimRef, Node, Queue, Requirement, State, claim_state, claims_section, queue_from_refs,
};
#[cfg(feature = "git")]
pub use repo::{
    Claim, claim, claim_fix, claim_local, claim_refs, next, next_wait, propose_refs, refusal,
    status_claims,
};
pub use taken::{BareClaim, Holding, HoldingChange, SpecRef, Taken, taken_from_refs};

use clm_model::Violation;

#[cfg(feature = "git")]
pub use clm_git::REMOTE;
pub const DEFAULT_EXPIRY_HOURS: u64 = 4;

#[derive(Debug)]
pub enum Error {
    /// The work isn't possible; the message says why.
    Refused(String),
    /// The specs don't load.
    Spec(Vec<Violation>),
    /// Git itself failed.
    Git(String),
}

#[cfg(feature = "git")]
impl From<clm_git::Error> for Error {
    fn from(e: clm_git::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// Whether a claim whose tip was committed at `tip_time` has lapsed by `now`; every release
/// of a claim goes through here.
fn lapsed(tip_time: u64, now: u64, expiry_hours: u64) -> bool {
    now.saturating_sub(tip_time) > expiry_hours.saturating_mul(3600)
}
