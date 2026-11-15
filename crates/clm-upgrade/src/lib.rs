//! Bringing a project set up by an older `clm` up to date with the one that runs
//! (`upgrade-migrations`).

mod commands;
#[cfg(test)]
mod commands_tests;
mod lock;
#[cfg(test)]
mod lock_tests;
mod migrations;
#[cfg(test)]
mod open_list_tests;
mod pending;
#[cfg(test)]
mod shipped_tests;
#[cfg(test)]
mod tests;

use std::path::Path;

pub use commands::{apply, apply_in, decline, decline_in, list, list_in, note, note_in};
pub use lock::{append_lock, check_lock};
pub use pending::{TooNew, pending, pending_in, require_current, require_current_in};

/// Whether a migration blocks commands while it is pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Required,
    Advised,
}

/// A note migration: advice for a file the project owns in words.
#[derive(Clone, Copy, Debug)]
pub struct Note {
    /// The note, in Markdown.
    pub text: &'static str,
    /// The file the note is about, relative to the project root.
    pub file: &'static str,
    /// The exact texts `clm init` wrote before.
    pub old: &'static [&'static str],
    /// The exact text `clm init` writes since.
    pub new: Option<&'static str>,
}

/// What a migration does.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Code `clm upgrade --apply` runs, and its description, whose first line `clm upgrade`
    /// lists (empty for none). Every `Required` migration is one.
    Apply(fn(&Path) -> Result<(), String>, &'static str),
    /// Advice, always `Advised`.
    Note(Note),
}

/// One step that brings a project up to date.
#[derive(Clone, Copy, Debug)]
pub struct Migration {
    pub id: &'static str,
    pub level: Level,
    /// Whether the project already has what the migration brings; reads only files under the root.
    pub done: fn(&Path) -> bool,
    pub kind: Kind,
}

/// The crate's own migrations, in the order they landed. The list is closed: a migration written
/// since is `init`'s (`own-list-closed`).
pub fn migrations() -> &'static [Migration] {
    migrations::LIST
}

/// `(migration, note it supersedes)` for the crate's own migrations (`superseded-notes`).
pub fn supersedes() -> &'static [(&'static str, &'static str)] {
    migrations::SUPERSEDES
}

/// The ID of the last migration of `list`, which `clm init` writes as `upgraded`; none for an
/// empty list.
pub fn latest_in(list: &[Migration]) -> Option<&str> {
    list.last().map(|m| m.id)
}

/// The last of the crate's own migrations.
// clm:remove-by 20261114-migrations-with-templates
pub fn latest() -> Option<&'static str> {
    latest_in(migrations())
}
