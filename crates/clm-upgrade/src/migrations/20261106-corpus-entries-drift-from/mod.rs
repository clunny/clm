//! `scripts/clm-check.sh` as `clm init` wrote it before the script ran from the repository's root.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note};

const NOTE: &str = include_str!("note.md");
const OLD: &str = include_str!("old.sh");
const NEW: &str = include_str!("new.sh");

const ROOT_LINE: &str = "cd \"$(dirname \"${BASH_SOURCE[0]}\")/..\"";

fn done(root: &Path) -> bool {
    let script = root.join("scripts/clm-check.sh");
    !script.exists()
        || fs::read_to_string(script).is_ok_and(|text| text.lines().any(|line| line == ROOT_LINE))
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261106-corpus-entries-drift-from",
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: "scripts/clm-check.sh",
        old: &[OLD],
        new: Some(NEW),
    }),
};
