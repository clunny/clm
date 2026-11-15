//! Bringing a repository under CLM: `clm init` sets up its tooling, `clm survey` maps its
//! code, `clm guide` prints the guide (see specs/init/SPEC.md).

mod guide;
mod init;
mod migrations;
mod nodes;
mod survey;

pub use guide::guide;
pub use init::{Options, UNATTENDED_RULES, check_permissions, init};
pub use migrations::{migrations, own_migrations, supersedes};
pub use nodes::check_nodes;
pub use survey::{import_edges, survey};
