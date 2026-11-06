//! `ARCHITECTURE.md` that copies CLM's carried constraints, as `clm init` wrote it up to `v0.17.0`,
//! becomes the pointer at `clm guide constraints`.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note};

const NOTE: &str = include_str!("note.md");
const OLD: &str = include_str!("old.md");
const NEW: &str = include_str!("new.md");

fn done(root: &Path) -> bool {
    fs::read_to_string(root.join("ARCHITECTURE.md")).map_or(true, |text| {
        !text.lines().any(|l| l.trim_end() == "## Enforced by clm")
    })
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261105-clm-upgrade",
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: "ARCHITECTURE.md",
        old: &[OLD],
        new: Some(NEW),
    }),
};
