//! Tests that `pending_in` applies the supersedes pairs its caller gives, and no others.

use std::fs;
use std::path::Path;

use crate::{Kind, Level, Migration, Note, pending_in, supersedes};

fn never(_: &Path) -> bool {
    false
}

fn always(_: &Path) -> bool {
    true
}

fn nothing(_: &Path) -> Result<(), String> {
    Ok(())
}

fn a_note(id: &'static str) -> Migration {
    Migration {
        id,
        level: Level::Advised,
        done: never,
        kind: Kind::Note(Note {
            text: "a note",
            file: "CLAUDE.md",
            old: &[],
            new: None,
        }),
    }
}

fn done_apply(id: &'static str) -> Migration {
    Migration {
        id,
        level: Level::Advised,
        done: always,
        kind: Kind::Apply(nothing, ""),
    }
}

fn ids(found: &[&Migration]) -> Vec<&'static str> {
    found.iter().map(|m| m.id).collect()
}

// req:upgrade/superseded-notes@bcf36436
#[test]
fn only_the_pairs_the_caller_gives_supersede() {
    let dir = tempfile::TempDir::new().unwrap();
    fs::write(dir.path().join("clm.toml"), "").unwrap();
    let list = [a_note("n"), done_apply("m")];
    assert!(
        pending_in(&list, &[("m", "n")], dir.path())
            .unwrap()
            .is_empty()
    );
    assert_eq!(ids(&pending_in(&list, &[], dir.path()).unwrap()), ["n"]);
    // The crate's own pair is not applied unless it is given.
    let own = [
        a_note("20261105-clm-upgrade-2"),
        done_apply("20261112-agents-md"),
    ];
    assert_eq!(
        ids(&pending_in(&own, &[], dir.path()).unwrap()),
        ["20261105-clm-upgrade-2"]
    );
    assert!(
        pending_in(&own, supersedes(), dir.path())
            .unwrap()
            .is_empty()
    );
}
