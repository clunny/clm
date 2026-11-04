//! The work each land spent, read from Claude Code's transcripts and kept as `effort` records
//! (specs/effort/SPEC.md, `effort-accounted`), and what each capability costs.

#[cfg(feature = "git")]
mod decide;
#[cfg(feature = "git")]
mod history;
mod prices;
#[cfg(feature = "git")]
mod record;
mod report;
mod text;
#[cfg(feature = "git")]
mod transcripts;

#[cfg(feature = "git")]
pub use decide::{decide, decide_architecture};
#[cfg(feature = "git")]
pub use history::{architecture_review, capabilities, history};
pub use prices::dollars;
#[cfg(feature = "git")]
pub use record::{Error, Options, Recorded, Unrecorded, record, unrecorded};
pub use report::{
    Constraint, Cost, Decided, Enforcing, Flag, History, Inputs, ItemLife, Overlap, Report, Spend,
    architecture, capabilities_of, flag_text, report, unsettled,
};
pub use text::{architecture_text, constraint_text, list_text, page_text};
#[cfg(feature = "git")]
pub use transcripts::{SessionWork, sessions};
