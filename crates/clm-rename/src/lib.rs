//! `clm rename`: a change's recorded renames carried out across specs, references and
//! test markers, and the replay the gate trusts (see specs/rename/SPEC.md).

mod build;
mod record;
mod replay;
mod text;

pub use build::rename;
pub use clm_git::Failure;
pub use replay::replays;
