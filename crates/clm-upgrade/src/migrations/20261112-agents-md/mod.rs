//! A project's instructions moved to `AGENTS.md`, which every supported agent harness reads, with
//! `CLAUDE.md` importing it.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration};

/// The `CLAUDE.md` texts released versions of `clm init` wrote.
const RELEASED: [&str; 2] = [
    include_str!("../20261105-clm-upgrade-2/old.md"),
    include_str!("../20261105-clm-upgrade-2/new.md"),
];
/// The `AGENTS.md` `clm init` writes as of this migration.
const AGENTS: &str = include_str!("agents.md");
/// The line `CLAUDE.md` holds to import `AGENTS.md`, and the whole `CLAUDE.md` `clm init` writes.
const IMPORT: &str = "@AGENTS.md\n";

const DESCRIPTION: &str = "Move CLAUDE.md's instructions to AGENTS.md, which every agent harness reads\n\
    AGENTS.md holds your instructions, and CLAUDE.md imports it with the line @AGENTS.md.";

/// True when `CLAUDE.md` doesn't exist or holds the line `@AGENTS.md`.
fn done(root: &Path) -> bool {
    fs::read_to_string(root.join("CLAUDE.md")).map_or(!root.join("CLAUDE.md").exists(), |text| {
        text.lines().any(|l| l.trim_end() == "@AGENTS.md")
    })
}

fn apply(root: &Path) -> Result<(), String> {
    let claude = root.join("CLAUDE.md");
    let Ok(text) = fs::read_to_string(&claude) else {
        return Ok(());
    };
    let write = |path: &Path, content: &str| {
        fs::write(path, content).map_err(|e| format!("can't write {}: {e}", path.display()))
    };
    let agents = root.join("AGENTS.md");
    if agents.exists() {
        // The project's own `AGENTS.md` stays; `CLAUDE.md` keeps every line, after the import.
        write(&claude, &format!("{IMPORT}\n{text}"))
    } else {
        // An unedited `CLAUDE.md` becomes what `clm init` writes today; an edited one moves whole.
        let moved = if RELEASED.contains(&text.as_str()) {
            AGENTS
        } else {
            &text
        };
        write(&agents, moved)?;
        write(&claude, IMPORT)
    }
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261112-agents-md",
    level: Level::Advised,
    done,
    kind: Kind::Apply(apply, DESCRIPTION),
};
