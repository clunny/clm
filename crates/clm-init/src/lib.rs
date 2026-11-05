//! Bringing a repository under CLM: `clm init` sets up its tooling, `clm survey` maps its
//! code, `clm guide` prints the guide (see specs/init/SPEC.md).

mod guide;
mod init;
mod nodes;
mod survey;

pub use guide::guide;
pub use init::{Options, UNATTENDED_RULES, check_permissions, init};
pub use nodes::check_nodes;
pub use survey::{import_edges, survey};
