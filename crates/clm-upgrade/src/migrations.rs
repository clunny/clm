//! The migrations the binary holds, in the order they landed on `main`. Each one is a directory
//! `src/migrations/<id>/` holding `mod.rs` and any text it embeds, pulled in here with
//! `#[path]` since an ID such as `20261105-clm-upgrade` isn't a Rust identifier.

use crate::Migration;

#[path = "migrations/20261106-corpus-entries-drift-from/mod.rs"]
mod check_script_from_root;
#[path = "migrations/20261105-clm-upgrade-2/mod.rs"]
mod claude_md_names_help;
#[path = "migrations/20261105-clm-upgrade/mod.rs"]
mod constraints_copy;
#[path = "migrations/20261107-projects-set-up-before/mod.rs"]
mod unattended_allow_rules;

/// The migrations, oldest first.
pub(crate) static LIST: &[Migration] = &[
    constraints_copy::MIGRATION,
    claude_md_names_help::MIGRATION,
    check_script_from_root::MIGRATION,
    unattended_allow_rules::MIGRATION,
];
