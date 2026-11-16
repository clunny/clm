//! Bringing a repository under CLM: `clm init` sets up its tooling, `clm survey` maps its
//! code, `clm guide` prints the guide (see specs/init/SPEC.md).

mod command;
mod guide;
mod init;
mod migrations;
mod nodes;
mod survey;

pub use command::{
    Env, GuideArgs, InitArgs, SurveyArgs, guide_command, init_command, survey_command,
};
use guide::guide;
use init::check_permissions;
pub use init::{Options, UNATTENDED_RULES, init};
pub use migrations::{migrations, own_migrations, supersedes};
pub use nodes::check_nodes;
pub use survey::import_edges;
use survey::survey;
