//! `clm land --check-only` (land/check-only): the land's checks on the working tree, landing
//! nothing, recording a clean committed `HEAD` that contains `origin/main` as checked.

mod common;

use common::{Fixture, refused, write_exec};

/// The lines `--check-only` printed.
fn lines(f: &Fixture) -> Vec<String> {
    clm_land::check_only(&f.work(), None, &f.env(&[]))
        .unwrap()
        .lines
}

/// `impl/a` with one code commit, checked out and clean in the work clone.
fn on_branch(f: &Fixture) -> String {
    let head = f.branch("impl/a", &[("crates/a/src/lib.rs", "a\n")]);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    head
}

#[test]
fn checks_pass_on_an_uncommitted_edit() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    std::fs::write(f.work().join("a.txt"), "edited\n").unwrap();
    let head = f.rev("HEAD");
    let records = f.records().len();
    let lines = lines(&f);
    assert_eq!(
        lines,
        [
            "not recorded: the working tree has uncommitted changes",
            "checks pass"
        ]
    );
    assert_eq!(f.rev("HEAD"), head);
    assert_eq!(f.records().len(), records);
    assert!(f.checked().is_empty());
    // Every test runs: the check gets no selection, whatever the caller's environment.
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
}

#[test]
fn an_untracked_file_is_an_uncommitted_change() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    std::fs::write(f.work().join("new.txt"), "new\n").unwrap();
    let lines = lines(&f);
    assert_eq!(
        lines[0],
        "not recorded: the working tree has uncommitted changes"
    );
    assert!(f.checked().is_empty());
}

#[test]
fn a_clean_committed_head_is_recorded_and_the_land_runs_no_check() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    let head = on_branch(&f);
    let lines = lines(&f);
    assert_eq!(
        lines,
        [format!("recorded {head} as checked"), "checks pass".into()]
    );
    let key = clm_ledger::code_key(&f.work(), &head).unwrap();
    assert_eq!(f.checked(), [key]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].ends_with(" code"), "{:?}", out.lines);
    // The one line is the check-only run's.
    assert_eq!(f.lines("check.log").len(), 1);
}

#[test]
fn the_record_names_the_command_a_land_names() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    on_branch(&f);
    lines(&f);
    let commands: Vec<String> = f
        .records()
        .into_iter()
        .filter_map(|r| match r.body {
            clm_ledger::Body::Checked { command, .. } => Some(command),
            _ => None,
        })
        .collect();
    assert_eq!(commands, [f.check_command()]);
}

#[test]
fn with_no_check_command_the_record_names_clm_check() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    let toml = "";
    f.commit_files(&[("clm.toml", toml)], "no check");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let lines = lines(&f);
    assert!(lines[0].starts_with("recorded "), "{lines:?}");
    let commands: Vec<String> = f
        .records()
        .into_iter()
        .filter_map(|r| match r.body {
            clm_ledger::Body::Checked { command, .. } => Some(command),
            _ => None,
        })
        .collect();
    assert_eq!(commands, ["clm check"]);
}

#[test]
fn main_moved_after_the_record() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    on_branch(&f);
    lines(&f);
    f.advance_main("crates/b/src/lib.rs", "b\n");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines[0].ends_with(" code"), "{:?}", out.lines);
    // The check ran again, on the merge's tree.
    assert_eq!(f.lines("check.log").len(), 2);
}

#[test]
fn head_behind_main() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    on_branch(&f);
    f.advance_main("crates/b/src/lib.rs", "b\n");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let lines = lines(&f);
    assert_eq!(
        lines,
        [
            "not recorded: HEAD doesn't contain origin/main",
            "checks pass"
        ]
    );
    assert!(f.checked().is_empty());
}

#[test]
fn a_commit_during_the_check() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    let commits = f.path("bin/commits");
    write_exec(
        &commits,
        "#!/bin/sh\necho x > made.txt\ngit add made.txt\ngit commit -q -m made\n",
    );
    let toml = format!("[land]\ncheck = \"{}\"\n", commits.display());
    f.commit_files(&[("clm.toml", &toml)], "commits");
    f.git(&f.work(), &["push", "-q", "origin", "main"]);
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let lines = lines(&f);
    assert_eq!(
        lines,
        ["not recorded: HEAD moved during the check", "checks pass"]
    );
    assert!(f.checked().is_empty());
}

#[test]
fn check_command_fails() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    f.write("check-fails", "");
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains(&f.check_command()), "{why}");
    assert!(why.contains("fails"), "{why}");
    assert_eq!(f.lines("check.log").len(), 1);
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
    assert!(f.checked().is_empty());
}

#[test]
fn clm_check_fails() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    f.write("clm-check-fails", "");
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains("`clm check` fails"), "{why}");
    assert!(f.checked().is_empty());
}

#[test]
fn no_check_command() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    std::fs::write(f.work().join("clm.toml"), "").unwrap();
    let out = clm_land::check_only(&f.work(), None, &f.env(&[])).unwrap();
    assert_eq!(out.lines.last().unwrap(), "checks pass");
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert_eq!(f.lines("clm-check.log").len(), 1);
}

#[test]
fn with_a_branch() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::check_only(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("--check-only"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
}

#[test]
fn unreadable_settings() {
    // req:land/check-only@938e24c2
    let f = Fixture::new("");
    std::fs::write(f.work().join("clm.toml"), "not = [toml").unwrap();
    let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
    assert!(why.contains("clm.toml"), "{why}");
    assert!(f.lines("check.log").is_empty(), "the check command ran");
    assert!(f.lines("clm-check.log").is_empty(), "`clm check` ran");
}
