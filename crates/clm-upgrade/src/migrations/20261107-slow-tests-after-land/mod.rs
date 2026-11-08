//! The commented-out `[verify]` section `clm init` writes, offered to a project set up before
//! `clm verify`. A copy of `clm-init`'s section as of this migration, since `upgrade` doesn't
//! depend on `init`. It changes no setting: the section stays commented until the human sets it.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration};

/// The section, as `clm init` writes it after one blank line.
const SECTION: &str = include_str!("verify.toml");

fn done(root: &Path) -> bool {
    match fs::read_to_string(root.join("clm.toml")) {
        Ok(text) => text
            .lines()
            .any(|line| matches!(line.trim(), "[verify]" | "# [verify]")),
        Err(_) => true,
    }
}

fn apply(root: &Path) -> Result<(), String> {
    let path = root.join("clm.toml");
    let mut text = fs::read_to_string(&path).map_err(|e| format!("can't read clm.toml: {e}"))?;
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    text.push('\n');
    text.push_str(SECTION);
    fs::write(&path, text).map_err(|e| format!("can't write clm.toml: {e}"))
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261107-slow-tests-after-land",
    level: Level::Advised,
    done,
    kind: Kind::Apply(
        apply,
        "Slow tests can run after the land: offer [verify]\n\
         `clm.toml` now ends with a commented-out `[verify]` section. Setting it moves slow tests \
         out of `[land] check` and runs them after the land (`clm guide ci-and-deployment`).",
    ),
};
