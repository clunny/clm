//! `clm rename`: a change's recorded renames carried out across specs, references and
//! test markers, and the replay the gate trusts (see specs/rename/SPEC.md).

mod build;
#[cfg(not(target_arch = "wasm32"))]
mod command;
mod record;
mod replay;
mod text;

pub use build::rename;
pub use clm_git::Failure;
#[cfg(not(target_arch = "wasm32"))]
pub use command::{RenameArgs, rename_command};
pub use replay::replays;
