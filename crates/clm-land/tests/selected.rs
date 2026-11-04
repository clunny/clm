//! The tests the change can break, handed to `[land] check` (land/selected-tests).

mod common;

use common::Fixture;

#[test]
fn selection_handed_over() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.write("affected-out", "component a\nfile crates/a/src/lib.rs\n");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let copied = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert_eq!(copied, "component a\nfile crates/a/src/lib.rs\n");
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} code")]);
    // The selection's file went with the land's other temporary files.
    f.assert_no_scratch();
}

#[test]
fn base_is_the_fetched_main() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // `main` moves after the branch was made, so the fetched `main` isn't the merge base.
    let base = f.advance_main("c.txt", "c\n");
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("affected.log"), [format!("--base {base}")]);
    // In the worktree the check runs in, on the merge commit.
    assert_eq!(f.lines("affected-pwd.log"), f.lines("check-pwd.log"));
}

#[test]
fn selection_fails() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.write("affected-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn code_already_checked() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let base = f.remote("main~1");
    assert_eq!(
        f.lines("affected.log"),
        [format!("--base {base} --readers")]
    );
    assert!(f.lines("check.log").is_empty(), "the check ran");
    assert_eq!(f.landed().len(), 1);
}

#[test]
fn readers_of_checked_code() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.write("readers-out", "file crates/a/tests/r.rs\n");
    f.branch("spec/x", &[("ARCHITECTURE.md", "# x\n")]);
    let key = f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let copied = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert_eq!(copied, "file crates/a/tests/r.rs\n");
    // In the land tree, under the lock `check_dir` shows.
    assert_eq!(f.lines("check-dir.log"), ["land-tree"]);
    assert_eq!(f.landed().len(), 1);
    assert_eq!(f.checked(), [key]);
}

#[test]
fn a_reader_fails_on_checked_code() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.write("readers-out", "file crates/a/tests/r.rs\n");
    f.write("check-fails", "");
    f.branch("spec/x", &[("ARCHITECTURE.md", "# x\n")]);
    f.mark_checked("origin/main");
    let before = f.remote("main");
    assert!(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).is_err());
    assert_eq!(f.remote("main"), before);
}

#[test]
fn readers_selection_fails_on_checked_code() {
    // req:land/selected-tests@3c8754b6
    let f = Fixture::new("");
    f.write("affected-fails", "");
    f.branch("spec/x", &[("ARCHITECTURE.md", "# x\n")]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
    assert_eq!(f.landed().len(), 1);
}
