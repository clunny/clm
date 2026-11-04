//! How far the code has caught up with the spec: markers, each requirement's coverage
//! state, the pending list, `clm status`'s sections and each live change's progress
//! (specs/check/coverage/SPEC.md). `clm-check` re-exports this library unchanged.
//!
//! Without the default feature `checkout`, only what works on files already in memory is
//! built, so the crate builds for `wasm32-unknown-unknown` (`coverage-builds-for-wasm`).

mod coverage;
mod glob;
mod pending;
mod status;

pub use coverage::{
    Coverage, Marker, MarkerSpan, Proof, STATE_NAMES, State, marker_spans, markers_in, removed,
    report,
};
#[cfg(feature = "checkout")]
pub use coverage::{Scan, scan};
pub use glob::matches;
pub use pending::{Pending, owner, pending};
pub use status::{
    ChangeProgress, Row, StatusReport, change_progress, complete_changes, status_report,
};
