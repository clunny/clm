//! The migrations the binary holds, in the order they landed on `main`. Each one is a directory
//! `src/migrations/<id>/` holding `mod.rs` and any text it embeds, pulled in here with
//! `#[path]` since an ID such as `20261105-clm-upgrade` isn't a Rust identifier.

use crate::Migration;

#[path = "migrations/20261112-agents-md/mod.rs"]
mod agents_md;
#[path = "migrations/20261109-a-build-s-tests/mod.rs"]
mod build_setup;
#[path = "migrations/20261106-corpus-entries-drift-from/mod.rs"]
mod check_script_from_root;
#[path = "migrations/20261105-clm-upgrade-2/mod.rs"]
mod claude_md_names_help;
#[path = "migrations/20261105-clm-upgrade/mod.rs"]
mod constraints_copy;
#[path = "migrations/20261108-toolchain-neutral-land-builds/mod.rs"]
mod land_dir;
#[path = "migrations/20261107-slow-tests-after-land/mod.rs"]
mod offer_verify;
#[path = "migrations/20261109-land-still-sets-cargo/mod.rs"]
mod starter_land_dir;
#[path = "migrations/20261107-projects-upgrading-past-the/mod.rs"]
mod tests_overlap;
#[path = "migrations/20261107-projects-set-up-before/mod.rs"]
mod unattended_allow_rules;

/// The migrations, oldest first.
pub(crate) static LIST: &[Migration] = &[
    constraints_copy::MIGRATION,
    claude_md_names_help::MIGRATION,
    check_script_from_root::MIGRATION,
    unattended_allow_rules::MIGRATION,
    offer_verify::MIGRATION,
    tests_overlap::MIGRATION,
    land_dir::MIGRATION,
    starter_land_dir::MIGRATION,
    build_setup::MIGRATION,
    agents_md::MIGRATION,
];

/// `(migration, note it supersedes)`: a note whose edit is to a text the later migration has
/// moved, so it is left out of `pending` once that migration is done.
pub(crate) static SUPERSEDES: &[(&str, &str)] = &[("20261112-agents-md", "20261105-clm-upgrade-2")];
