//! Bringing a project set up by an older `clm` up to date with the one that runs
//! (`upgrade-migrations`).

mod commands;
#[cfg(test)]
mod commands_tests;
mod lock;
#[cfg(test)]
mod lock_tests;
mod migrations;
mod pending;
#[cfg(test)]
mod shipped_tests;
#[cfg(test)]
mod tests;

use std::path::Path;

pub use commands::{apply, decline, list, note};
pub use lock::{append_lock, check_lock};
pub use pending::{TooNew, pending, require_current};

/// Whether a migration blocks commands while it is pending.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    Required,
    Advised,
}

/// A note migration: advice for a file the project owns in words.
#[derive(Debug)]
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
#[derive(Debug)]
pub enum Kind {
    /// Code `clm upgrade --apply` runs. Every `Required` migration is one.
    Apply(fn(&Path) -> Result<(), String>),
    /// Advice, always `Advised`.
    Note(Note),
}

/// One step that brings a project up to date.
#[derive(Debug)]
pub struct Migration {
    pub id: &'static str,
    pub level: Level,
    /// Whether the project already has what the migration brings; reads only files under the root.
    pub done: fn(&Path) -> bool,
    pub kind: Kind,
}

/// The migrations the binary holds, in the order they landed.
pub fn migrations() -> &'static [Migration] {
    migrations::LIST
}

/// The last migration's ID, which `clm init` writes as `upgraded`; none while the binary holds no
/// migration.
pub fn latest() -> Option<&'static str> {
    migrations().last().map(|m| m.id)
}
