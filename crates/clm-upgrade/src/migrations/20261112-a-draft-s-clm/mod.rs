//! The allow rules for `clm setup` that `clm init` seeds on a first run, offered to a project set
//! up before it did. A copy of the two rules as of this migration, since `upgrade` doesn't depend
//! on `init`, and `20261107-projects-set-up-before`, frozen once shipped, offers the others.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note};

const NOTE: &str = include_str!("note.md");

/// The rules this migration offers.
const RULES: [&str; 2] = ["Bash(clm setup)", "Bash(clm setup *)"];

/// The strings `permissions.allow` holds in a settings file, none for a file that is missing or
/// isn't JSON of that shape.
fn allowed(path: &Path) -> Vec<String> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return Vec::new();
    };
    json.pointer("/permissions/allow")
        .and_then(|a| a.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn done(root: &Path) -> bool {
    let mut allow = allowed(&root.join(".claude/settings.json"));
    allow.extend(allowed(&root.join(".claude/settings.local.json")));
    RULES.iter().all(|rule| allow.iter().any(|a| a == rule))
}

pub(crate) const MIGRATION: Migration = Migration {
    id: "20261112-a-draft-s-clm",
    level: Level::Advised,
    done,
    kind: Kind::Note(Note {
        text: NOTE,
        file: ".claude/settings.json",
        old: &[],
        new: None,
    }),
};
