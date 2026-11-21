//! Tests of `clm upgrade`'s flags over fixture lists.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use crate::commands::set_top_level;
use crate::{Kind, Level, Migration, Note};
use crate::{apply_in, decline_in, list_in, note_in};

fn make_m(root: &Path) -> Result<(), String> {
    fs::write(root.join("m"), "made").map_err(|e| e.to_string())
}
fn done_m(root: &Path) -> bool {
    root.join("m").exists()
}
fn never(_: &Path) -> bool {
    false
}
fn done_n(root: &Path) -> bool {
    fs::read_to_string(root.join("NOTE.md")).is_ok_and(|t| t == "new")
}
fn done_a(root: &Path) -> bool {
    root.join("a").exists()
}

fn apply_m() -> Migration {
    Migration {
        id: "m",
        level: Level::Required,
        done: done_m,
        kind: Kind::Apply(make_m, ""),
    }
}
fn required_r() -> Migration {
    Migration {
        id: "r",
        level: Level::Required,
        done: never,
        kind: Kind::Apply(|_| Ok(()), ""),
    }
}
fn note_n() -> Migration {
    Migration {
        id: "n",
        level: Level::Advised,
        done: done_n,
        kind: Kind::Note(Note {
            text: "first line\nmore",
            file: "NOTE.md",
            old: &["old", "older"],
            new: Some("new"),
        }),
    }
}
fn note_a() -> Migration {
    Migration {
        id: "a",
        level: Level::Advised,
        done: done_a,
        kind: Kind::Note(Note {
            text: "CLAUDE.md names /clm:help",
            file: "CLAUDE.md",
            old: &[],
            new: None,
        }),
    }
}

fn project(clm_toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("clm.toml"), clm_toml).unwrap();
    dir
}

fn read(dir: &TempDir, name: &str) -> String {
    fs::read_to_string(dir.path().join(name)).unwrap()
}

// req:upgrade/upgrade-list@97f2a710
#[test]
fn lists_pending_migrations_in_order_and_writes_nothing() {
    let list = [required_r(), note_a()];
    let dir = project("");
    assert_eq!(
        list_in(&list, &[], dir.path()).unwrap(),
        "r required apply\na advised note CLAUDE.md names /clm:help\n"
    );
    assert_eq!(read(&dir, "clm.toml"), "");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

// req:upgrade/upgrade-list@97f2a710
#[test]
fn lists_up_to_date_and_refuses_too_new() {
    let dir = project("");
    assert_eq!(
        list_in(&[note_a()], &[], dir.path())
            .unwrap()
            .lines()
            .count(),
        1
    );
    fs::write(dir.path().join("a"), "").unwrap();
    assert_eq!(
        list_in(&[note_a()], &[], dir.path()).unwrap(),
        "up to date\n"
    );
    let stamped = project("upgraded = \"z\"\n");
    let err = list_in(&[note_a()], &[], stamped.path()).unwrap_err();
    assert!(err.contains("`z`") && err.contains("newer"), "{err}");
}

// req:upgrade/upgrade-list@97f2a710
#[test]
fn marks_a_skipped_required_migration() {
    let main = TempDir::new().unwrap();
    clm_git::git(main.path(), &["init", "-q"]).unwrap();
    fs::write(main.path().join("clm.toml"), "").unwrap();
    let list = [required_r()];
    assert_eq!(
        list_in(&list, &[], main.path()).unwrap(),
        "r required apply\n"
    );
    fs::write(
        main.path().join("clm.local.toml"),
        "skip_migrations = [\"r\"]\n",
    )
    .unwrap();
    assert_eq!(
        list_in(&list, &[], main.path()).unwrap(),
        "r required apply skipped\n"
    );
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn applies_stamps_and_leaves_an_edited_file() {
    let list = [apply_m(), note_n()];
    let dir = project("# top\nlanguage = \"rust\"\n\n[land]\ncheck = \"true\"\n");
    fs::write(dir.path().join("NOTE.md"), "edited").unwrap();
    assert_eq!(
        apply_in(&list, &[], dir.path()).unwrap(),
        "applied m\nnote n\n"
    );
    assert_eq!(read(&dir, "m"), "made");
    assert_eq!(read(&dir, "NOTE.md"), "edited");
    assert_eq!(
        read(&dir, "clm.toml"),
        "# top\nlanguage = \"rust\"\nupgraded = \"n\"\n\n[land]\ncheck = \"true\"\n"
    );
    let after = read(&dir, "clm.toml");
    assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "note n\n");
    assert_eq!(read(&dir, "clm.toml"), after);
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn rewrites_an_unedited_note_file() {
    let list = [note_n()];
    for old in ["old", "older"] {
        let dir = project("");
        fs::write(dir.path().join("NOTE.md"), old).unwrap();
        assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "applied n\n");
        assert_eq!(read(&dir, "NOTE.md"), "new");
        assert_eq!(list_in(&list, &[], dir.path()).unwrap(), "up to date\n");
    }
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn replaces_an_existing_stamp_and_writes_nothing_when_current() {
    let list = [apply_m(), note_n()];
    let dir = project("upgraded = \"m\"\nhost = \"none\"\n");
    fs::write(dir.path().join("m"), "").unwrap();
    fs::write(dir.path().join("NOTE.md"), "new").unwrap();
    assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "");
    assert_eq!(
        read(&dir, "clm.toml"),
        "upgraded = \"n\"\nhost = \"none\"\n"
    );
    let settled = project("upgraded = \"n\"\n");
    fs::write(settled.path().join("NOTE.md"), "new").unwrap();
    fs::write(settled.path().join("m"), "").unwrap();
    assert_eq!(apply_in(&list, &[], settled.path()).unwrap(), "");
    assert_eq!(read(&settled, "clm.toml"), "upgraded = \"n\"\n");
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn apply_refuses_too_new_and_writes_nothing() {
    let dir = project("upgraded = \"z\"\n");
    let err = apply_in(&[apply_m()], &[], dir.path()).unwrap_err();
    assert!(err.contains("`z`"), "{err}");
    assert!(!dir.path().join("m").exists());
    assert_eq!(read(&dir, "clm.toml"), "upgraded = \"z\"\n");
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn the_stamp_goes_in_as_a_top_level_key() {
    let list = [apply_m()];
    let dir = project("a = 1");
    assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "applied m\n");
    assert_eq!(read(&dir, "clm.toml"), "a = 1\nupgraded = \"m\"\n");
    // With no clm.toml, what is pending runs and no stamp is written.
    let bare = TempDir::new().unwrap();
    assert_eq!(apply_in(&list, &[], bare.path()).unwrap(), "applied m\n");
    assert_eq!(read(&bare, "m"), "made");
    assert!(!bare.path().join("clm.toml").exists());
    // An empty file, and one whose only table follows the key.
    assert_eq!(set_top_level("", "k", "1").unwrap(), "k = 1\n");
    assert_eq!(
        set_top_level("[t]\nk = 2\n", "k", "1").unwrap(),
        "k = 1\n\n[t]\nk = 2\n"
    );
    assert_eq!(
        set_top_level("a = 1\n[t]\nk = 2\n", "k", "1").unwrap(),
        "a = 1\nk = 1\n\n[t]\nk = 2\n"
    );
    // A multi-line array is replaced whole.
    assert_eq!(
        set_top_level("k = [\n  \"a\",\n]\nz = 1\n", "k", "[\"b\"]").unwrap(),
        "k = [\"b\"]\nz = 1\n"
    );
}

fn make_s(root: &Path) -> Result<(), String> {
    fs::write(root.join("s"), "made").map_err(|e| e.to_string())
}
fn done_s(root: &Path) -> bool {
    root.join("s").exists()
}
fn apply_s() -> Migration {
    Migration {
        id: "s",
        level: Level::Required,
        done: done_s,
        kind: Kind::Apply(make_s, ""),
    }
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn an_odd_stamp_line_is_replaced_alone() {
    let list = [apply_s(), apply_m()];
    let rest = "defects = \"ask\"\n\n[land]\ncheck = \"true\"\n";
    let commented = project(&format!("upgraded = \"s\" # was [old\n{rest}"));
    assert_eq!(
        apply_in(&list, &[], commented.path()).unwrap(),
        "applied m\n"
    );
    assert_eq!(
        read(&commented, "clm.toml"),
        format!("upgraded = \"m\"\n{rest}")
    );
    let quoted = project("\"upgraded\" = \"s\"\ndefects = \"ask\"\n");
    assert_eq!(apply_in(&list, &[], quoted.path()).unwrap(), "applied m\n");
    let parsed: toml::Table = read(&quoted, "clm.toml").parse().unwrap();
    assert_eq!(parsed.get("upgraded").and_then(|v| v.as_str()), Some("m"));
    assert_eq!(parsed.get("defects").and_then(|v| v.as_str()), Some("ask"));
    assert_eq!(parsed.len(), 2);
    let dotted = project(&format!("upgraded.x = 1\n{rest}"));
    assert_eq!(
        apply_in(&list, &[], dotted.path()).unwrap(),
        "applied s\napplied m\n"
    );
    assert_eq!(
        read(&dotted, "clm.toml"),
        format!("upgraded = \"m\"\n{rest}")
    );
    // Dotted lines apart from one another all go, the new line where the first was.
    assert_eq!(
        set_top_level("k.x = 1\na = 2\nk.y = [\n  3,\n]\nb = 4\n", "k", "5").unwrap(),
        "k = 5\na = 2\nb = 4\n"
    );
    // A string holding a bracket ends nothing, and a key that only starts with the name is
    // another key.
    assert_eq!(
        set_top_level("k = \"[\"\nkk = 1\nz = 2\n", "k", "5").unwrap(),
        "k = 5\nkk = 1\nz = 2\n"
    );
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn prints_a_migrations_note() {
    let dir = project("");
    assert_eq!(
        note_in(&[note_n()], &[], dir.path(), "n").unwrap(),
        "first line\nmore\n"
    );
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn note_refuses_unknown_ids_and_too_new() {
    let dir = project("");
    assert!(
        note_in(&[note_n()], &[], dir.path(), "nope")
            .unwrap_err()
            .contains("nope")
    );
    let stamped = project("upgraded = \"z\"\n");
    assert!(note_in(&[note_n()], &[], stamped.path(), "n").is_err());
    // An apply migration has no note.
    let err = note_in(&[required_r(), note_n()], &[], dir.path(), "r").unwrap_err();
    assert!(err.contains("`r`"), "{err}");
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn declining_a_pending_note_hides_it() {
    let list = [note_a(), note_n()];
    let dir = project("language = \"rust\"\n\n[land]\ncheck = \"true\"\n");
    assert_eq!(
        decline_in(&list, &[], dir.path(), "a").unwrap(),
        "declined a\n"
    );
    assert_eq!(
        read(&dir, "clm.toml"),
        "language = \"rust\"\ndeclined_migrations = [\"a\"]\n\n[land]\ncheck = \"true\"\n"
    );
    assert_eq!(
        list_in(&list, &[], dir.path()).unwrap(),
        "n advised note first line\n"
    );
    decline_in(&list, &[], dir.path(), "n").unwrap();
    assert!(read(&dir, "clm.toml").contains("declined_migrations = [\"a\", \"n\"]\n"));
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn decline_refuses_what_it_may_not_decline() {
    let list = [required_r(), note_a()];
    let dir = project("x = 1\n");
    let err = decline_in(&list, &[], dir.path(), "r").unwrap_err();
    assert!(err.contains("`r`"), "{err}");
    assert!(decline_in(&list, &[], dir.path(), "nope").is_err());
    fs::write(dir.path().join("a"), "").unwrap();
    assert!(decline_in(&list, &[], dir.path(), "a").is_err());
    assert_eq!(read(&dir, "clm.toml"), "x = 1\n");
    let stamped = project("upgraded = \"z\"\n");
    assert!(decline_in(&list, &[], stamped.path(), "a").is_err());
    assert_eq!(read(&stamped, "clm.toml"), "upgraded = \"z\"\n");
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn a_failing_migration_does_not_stop_the_others_or_stamp() {
    let bad = Migration {
        id: "bad",
        level: Level::Required,
        done: never,
        kind: Kind::Apply(|_| Err("boom".to_string()), ""),
    };
    let list = [bad, apply_m()];
    let dir = project("");
    let err = apply_in(&list, &[], dir.path()).unwrap_err();
    assert!(
        err.contains("applied m") && err.contains("`bad` failed: boom"),
        "{err}"
    );
    assert!(dir.path().join("m").exists());
    assert_eq!(read(&dir, "clm.toml"), "");
}

fn done_by_stamp(root: &Path) -> bool {
    fs::read_to_string(root.join("clm.toml")).is_ok_and(|t| t.contains("upgraded = \"s\""))
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn a_note_the_stamp_takes_is_shown_on_the_run_that_takes_it() {
    let a = Migration {
        id: "a",
        level: Level::Required,
        done: never,
        kind: Kind::Apply(|_| Ok(()), ""),
    };
    let s = Migration {
        id: "s",
        level: Level::Advised,
        done: done_by_stamp,
        kind: Kind::Note(Note {
            text: "read me",
            file: "clm.toml",
            old: &[],
            new: None,
        }),
    };
    let list = [a, s];
    let dir = project("upgraded = \"a\"\n");
    assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "note s\n");
    assert_eq!(read(&dir, "clm.toml"), "upgraded = \"s\"\n");
    assert_eq!(apply_in(&list, &[], dir.path()).unwrap(), "");
    assert_eq!(list_in(&list, &[], dir.path()).unwrap(), "up to date\n");
}

// req:upgrade/upgrade-list@97f2a710
#[test]
fn an_advised_migration_is_never_marked_skipped() {
    let main = TempDir::new().unwrap();
    clm_git::git(main.path(), &["init", "-q"]).unwrap();
    fs::write(
        main.path().join("clm.local.toml"),
        "skip_migrations = [\"a\"]\n",
    )
    .unwrap();
    assert_eq!(
        list_in(&[note_a()], &[], main.path()).unwrap(),
        "a advised note CLAUDE.md names /clm:help\n"
    );
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn decline_refuses_a_clm_toml_it_cannot_read() {
    let dir = project("declined_migrations = [\"x\"\n");
    let err = decline_in(&[note_a()], &[], dir.path(), "a").unwrap_err();
    assert!(err.contains("clm.toml"), "{err}");
    assert_eq!(read(&dir, "clm.toml"), "declined_migrations = [\"x\"\n");
    fs::write(dir.path().join("clm.toml"), b"\xff\xfe = 1\n").unwrap();
    let err = decline_in(&[note_a()], &[], dir.path(), "a").unwrap_err();
    assert!(err.contains("clm.toml"), "{err}");
    assert_eq!(
        fs::read(dir.path().join("clm.toml")).unwrap(),
        b"\xff\xfe = 1\n"
    );
    fs::remove_file(dir.path().join("clm.toml")).unwrap();
    let err = decline_in(&[note_a()], &[], dir.path(), "a").unwrap_err();
    assert!(err.contains("clm.toml"), "{err}");
    assert!(!dir.path().join("clm.toml").exists());
}

fn note_b() -> Migration {
    Migration {
        id: "b",
        level: Level::Advised,
        done: never,
        kind: Kind::Note(Note {
            text: "b",
            file: "B.md",
            old: &[],
            new: None,
        }),
    }
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn an_odd_declined_line_is_replaced_alone() {
    let list = [note_a(), note_b()];
    let rest = "defects = \"ask\"\n\n[land]\ncheck = \"true\"\n";
    let commented = project(&format!("declined_migrations = [\"b\"] # was [old\n{rest}"));
    assert_eq!(
        decline_in(&list, &[], commented.path(), "a").unwrap(),
        "declined a\n"
    );
    assert_eq!(
        read(&commented, "clm.toml"),
        format!("declined_migrations = [\"b\", \"a\"]\n{rest}")
    );
    let quoted = project("\"declined_migrations\" = [\"b\"]\ndefects = \"ask\"\n");
    assert_eq!(
        decline_in(&list, &[], quoted.path(), "a").unwrap(),
        "declined a\n"
    );
    let parsed: toml::Table = read(&quoted, "clm.toml").parse().unwrap();
    assert_eq!(
        parsed.get("declined_migrations"),
        Some(&toml::Value::Array(vec!["b".into(), "a".into()]))
    );
    assert_eq!(parsed.get("defects").and_then(|v| v.as_str()), Some("ask"));
    assert_eq!(parsed.len(), 2);
    let dotted = project("declined_migrations.x = 1\ndefects = \"ask\"\n");
    assert_eq!(
        decline_in(&list, &[], dotted.path(), "a").unwrap(),
        "declined a\n"
    );
    assert_eq!(
        read(&dotted, "clm.toml"),
        "declined_migrations = [\"a\"]\ndefects = \"ask\"\n"
    );
}

// req:upgrade/upgrade-apply@e3cdf9ee
#[test]
fn the_stamp_goes_above_a_commented_header() {
    let list = [apply_m()];
    let dir = project("host = \"github\"\n\n# Run before clm check.\n[land]\ncheck = \"true\"\n");
    apply_in(&list, &[], dir.path()).unwrap();
    assert_eq!(
        read(&dir, "clm.toml"),
        "host = \"github\"\nupgraded = \"m\"\n\n# Run before clm check.\n[land]\ncheck = \"true\"\n"
    );
    // A comment with no blank line before it, and one reaching the top of the file.
    assert_eq!(
        set_top_level("a = 1\n# c1\n# c2\n[t]\n", "k", "1").unwrap(),
        "a = 1\nk = 1\n\n# c1\n# c2\n[t]\n"
    );
    assert_eq!(
        set_top_level("# c\n[t]\n", "k", "1").unwrap(),
        "k = 1\n\n# c\n[t]\n"
    );
    // Several blank lines belong to the lead-in.
    assert_eq!(
        set_top_level("a = 1\n\n\n# c\n[t]\n", "k", "1").unwrap(),
        "a = 1\nk = 1\n\n\n# c\n[t]\n"
    );
}

// req:upgrade/upgrade-note-and-decline@727f6913
#[test]
fn declining_goes_above_a_commented_header() {
    let dir = project("# Run before clm check.\n[land]\ncheck = \"true\"\n");
    decline_in(&[note_a()], &[], dir.path(), "a").unwrap();
    assert_eq!(
        read(&dir, "clm.toml"),
        "declined_migrations = [\"a\"]\n\n# Run before clm check.\n[land]\ncheck = \"true\"\n"
    );
}
