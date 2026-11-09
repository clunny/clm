//! `scripts/clm-check.sh` as `clm init --lang rust` wrote it before the script pointed cargo at
//! the place a land hands its check.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note};

const NOTE: &str = include_str!("note.md");
const OLD: &str = include_str!("old.sh");
const NEW: &str = include_str!("new.sh");

/// True when the script doesn't exist, holds `CLM_LAND_DIR`, or doesn't hold `cargo`.
fn done(root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(root.join("scripts/clm-check.sh")) else {
        return !root.join("scripts/clm-check.sh").exists();
    };
    text.contains("CLM_LAND_DIR") || !text.contains("cargo")
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261109-land-still-sets-cargo",
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: "scripts/clm-check.sh",
        old: &[OLD],
        new: Some(NEW),
    }),
};
