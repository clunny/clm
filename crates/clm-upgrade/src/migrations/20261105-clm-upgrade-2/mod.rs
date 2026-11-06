//! `CLAUDE.md` as `clm init` wrote it before it named `/clm:help`.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note};

const NOTE: &str = include_str!("note.md");
const OLD: &str = include_str!("old.md");
const NEW: &str = include_str!("new.md");

fn done(root: &Path) -> bool {
    fs::read_to_string(root.join("CLAUDE.md")).map_or(true, |text| text.contains("/clm:help"))
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261105-clm-upgrade-2",
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: "CLAUDE.md",
        old: &[OLD],
        new: Some(NEW),
    }),
};
