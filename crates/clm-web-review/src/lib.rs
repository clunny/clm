//! A spec branch in the Worker (see specs/web/worker/review/SPEC.md): the spec-branch list,
//! the review, approve, comments and change requests. `web.worker`'s router makes the session
//! and access checks and passes the `Access` they give; this crate imports nothing of `clm-web`.

pub mod approve;
pub mod branches;
pub mod feedback;
pub mod review;
