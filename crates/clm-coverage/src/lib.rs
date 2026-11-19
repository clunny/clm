//! How far the code has caught up with the spec: markers, each requirement's coverage
//! state, the pending list, `clm status`'s sections and each live change's progress
//! (specs/check/coverage/SPEC.md). Callers import it directly; `clm-check` re-exports
//! none of it (`check-coverage-split`).
//!
//! Without the default feature `checkout`, only what works on files already in memory is
//! built, so the crate builds for `wasm32-unknown-unknown` (`coverage-builds-for-wasm`).

mod coverage;
mod glob;
mod lists;
mod pending;
mod spans;
mod status;

pub use coverage::{
    Coverage, Marker, MarkerSpan, Proof, STATE_NAMES, State, marker_spans, markers_in, removed,
    report,
};
#[cfg(feature = "checkout")]
pub use coverage::{Scan, scan};
pub use glob::matches;
pub use lists::{
    DeletedTest, ObsoleteTest, UncitedTest, Uncounted, bare_name, deleted_tests, obsolete_tests,
    uncited_tests, uncounted,
};
pub use pending::{Pending, owner, pending, waits};
pub use spans::{TestSpan, markers_outside_tests, tests_in};
pub use status::{
    ChangeProgress, Row, StatusReport, change_progress, complete_changes, moved_to, status_report,
};
