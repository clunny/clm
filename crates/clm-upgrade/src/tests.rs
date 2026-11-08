//! Tests of the migration list, what is pending and what blocks, over fixture lists.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::pending::{TooNew, pending_in, require_current_in};
use crate::{Kind, Level, Migration, Note, latest, migrations};

fn marker_done(name: &'static str) -> impl Fn(&Path) -> bool {
    move |root| root.join(name).exists()
}

fn done_a(root: &Path) -> bool {
    marker_done("a")(root)
}
fn done_b(root: &Path) -> bool {
    marker_done("b")(root)
}
fn done_c(root: &Path) -> bool {
    marker_done("c")(root)
}

fn nothing(_: &Path) -> Result<(), String> {
    Ok(())
}

fn apply(id: &'static str, level: Level, done: fn(&Path) -> bool) -> Migration {
    Migration {
        id,
        level,
        done,
        kind: Kind::Apply(nothing, ""),
    }
}

fn note(id: &'static str, done: fn(&Path) -> bool) -> Migration {
    Migration {
        id,
        level: Level::Advised,
        done,
        kind: Kind::Note(Note {
            text: "a note",
            file: "CLAUDE.md",
            old: &[],
            new: None,
        }),
    }
}

fn ids(list: &[&Migration]) -> Vec<&'static str> {
    list.iter().map(|m| m.id).collect()
}

fn project(clm_toml: Option<&str>) -> TempDir {
    let dir = TempDir::new().unwrap();
    if let Some(text) = clm_toml {
        fs::write(dir.path().join("clm.toml"), text).unwrap();
    }
    dir
}

/// Why a list is not well formed: a repeated ID, or a note that is not advised.
fn malformed(list: &[Migration]) -> Option<String> {
    for (i, m) in list.iter().enumerate() {
        if list[..i].iter().any(|earlier| earlier.id == m.id) {
            return Some(format!("two migrations share the ID {}", m.id));
        }
        if matches!(m.kind, Kind::Note(_)) && m.level == Level::Required {
            return Some(format!("{} is required with a note", m.id));
        }
    }
    None
}

// req:upgrade/migration-list@067d119c
#[test]
fn the_binarys_migrations_are_well_formed() {
    assert_eq!(malformed(migrations()), None);
    assert_eq!(latest(), migrations().last().map(|m| m.id));
}

// req:upgrade/migration-list@067d119c
#[test]
fn a_repeated_id_or_a_required_note_is_malformed() {
    let twice = [
        apply("a", Level::Required, done_a),
        apply("a", Level::Advised, done_b),
    ];
    assert!(malformed(&twice).unwrap().contains("share the ID a"));
    let mut required_note = note("n", done_a);
    required_note.level = Level::Required;
    assert!(malformed(&[required_note]).unwrap().contains("required"));
    assert_eq!(
        malformed(&[apply("a", Level::Required, done_a), note("n", done_b)]),
        None
    );
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn stamped_projects_get_the_apply_migrations_after_the_stamp() {
    let list = [
        apply("a", Level::Required, done_a),
        apply("b", Level::Required, done_b),
        apply("c", Level::Required, done_c),
    ];
    let dir = project(Some("upgraded = \"a\"\n"));
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["b", "c"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn a_migration_whose_files_are_right_is_not_pending() {
    let list = [
        apply("a", Level::Required, done_a),
        apply("b", Level::Required, done_b),
        apply("c", Level::Required, done_c),
    ];
    let dir = project(Some("upgraded = \"a\"\n"));
    fs::write(dir.path().join("c"), "").unwrap();
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["b"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn an_unstamped_project_is_judged_by_done_alone() {
    let list = [
        apply("a", Level::Required, done_a),
        apply("b", Level::Required, done_b),
        apply("c", Level::Required, done_c),
    ];
    let dir = project(Some("defects = \"ask\"\n"));
    fs::write(dir.path().join("a"), "").unwrap();
    fs::write(dir.path().join("c"), "").unwrap();
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["b"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn a_clm_toml_that_does_not_parse_counts_as_unstamped() {
    let list = [apply("a", Level::Required, done_a)];
    let dir = project(Some("upgraded = [unclosed\n"));
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["a"]);
    let none = project(None);
    assert_eq!(ids(&pending_in(&list, none.path()).unwrap()), ["a"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn a_declined_note_is_not_pending() {
    let list = [note("n", done_a), note("m", done_b)];
    let dir = project(Some("declined_migrations = [\"n\"]\n"));
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["m"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn a_note_before_the_stamp_is_still_pending() {
    let list = [note("n", done_a), apply("b", Level::Required, done_b)];
    let dir = project(Some("upgraded = \"b\"\n"));
    assert_eq!(ids(&pending_in(&list, dir.path()).unwrap()), ["n"]);
}

// req:upgrade/pending-migrations@dbcfa176
#[test]
fn a_stamp_the_list_lacks_is_too_new() {
    let list = [apply("a", Level::Required, done_a)];
    let dir = project(Some("upgraded = \"z\"\n"));
    assert_eq!(
        pending_in(&list, dir.path()).unwrap_err(),
        TooNew("z".into())
    );
}

// req:upgrade/require-current@d9893e22
#[test]
fn a_pending_required_migration_blocks_and_an_advised_one_is_not_named() {
    let list = [
        apply("r", Level::Required, done_a),
        apply("adv", Level::Advised, done_b),
    ];
    let dir = project(None);
    let err = require_current_in(&list, dir.path()).unwrap_err();
    assert!(err.contains("`r`") && err.contains("clm upgrade"), "{err}");
    assert!(err.contains("/clm:upgrade"), "{err}");
    assert!(!err.contains("adv"), "{err}");
}

// req:upgrade/require-current@d9893e22
#[test]
fn every_blocking_migration_is_named() {
    let list = [
        apply("first", Level::Required, done_a),
        apply("second", Level::Required, done_b),
    ];
    let dir = project(None);
    let err = require_current_in(&list, dir.path()).unwrap_err();
    assert!(err.contains("`first`") && err.contains("`second`"), "{err}");
}

// req:upgrade/require-current@d9893e22
#[test]
fn advised_migrations_alone_do_not_block() {
    let list = [apply("adv", Level::Advised, done_a), note("n", done_b)];
    let dir = project(None);
    assert_eq!(require_current_in(&list, dir.path()), Ok(()));
}

// req:upgrade/require-current@d9893e22
#[test]
fn a_project_upgraded_by_a_newer_clm_says_so() {
    let list = [apply("a", Level::Required, done_a)];
    let dir = project(Some("upgraded = \"z\"\n"));
    let err = require_current_in(&list, dir.path()).unwrap_err();
    assert!(err.contains("newer `clm`") && err.contains("`z`"), "{err}");
}

fn git(dir: &Path, args: &[&str]) {
    let out = std::process::Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// req:upgrade/require-current@d9893e22
#[test]
fn a_skip_in_the_main_worktrees_local_settings_reaches_a_linked_worktree() {
    let list = [apply("r", Level::Required, done_a)];
    let dir = TempDir::new().unwrap();
    let base = dir.path().canonicalize().unwrap();
    let main = base.join("main");
    fs::create_dir_all(&main).unwrap();
    git(&main, &["init", "-q"]);
    fs::write(main.join("clm.toml"), "defects = \"ask\"\n").unwrap();
    git(&main, &["add", "."]);
    git(
        &main,
        &[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "init",
        ],
    );
    let linked = base.join("linked");
    git(&main, &["worktree", "add", "-q", linked.to_str().unwrap()]);
    assert!(require_current_in(&list, &linked).is_err());
    fs::write(main.join("clm.local.toml"), "skip_migrations = [\"r\"]\n").unwrap();
    assert_eq!(require_current_in(&list, &linked), Ok(()));
    // A skip names one migration only.
    let other = [
        apply("r", Level::Required, done_a),
        apply("s", Level::Required, done_b),
    ];
    let err = require_current_in(&other, &linked).unwrap_err();
    assert!(err.contains("`s`") && !err.contains("`r`"), "{err}");
}
