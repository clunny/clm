//! A note to projects set up before a build's tests had to run against its own worktree's code
//! and dependencies. It edits no file, so its `done` reads the upgrade stamp
//! (`stamp-taken-notes`).

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note, migrations};

const NOTE: &str = include_str!("note.md");
const ID: &str = "20261109-a-build-s-tests";

/// True when `clm.toml` doesn't exist, or parses and its `upgraded` names this migration or one
/// after it in the list.
fn done(root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(root.join("clm.toml")) else {
        return true;
    };
    let Ok(config) = text.parse::<toml::Table>() else {
        return false;
    };
    let Some(stamp) = config.get("upgraded").and_then(|v| v.as_str()) else {
        return false;
    };
    let list = migrations();
    let at = |id: &str| list.iter().position(|m| m.id == id);
    matches!((at(stamp), at(ID)), (Some(stamped), Some(own)) if stamped >= own)
}

pub(crate) const MIGRATION: Migration = Migration {
    id: ID,
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: "clm.toml",
        old: &[],
        new: None,
    }),
};
