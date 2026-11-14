//! A land checks only where the branch and the `main` it landed on both can break
//! (land/overlap-recheck).

mod common;

use clm_ledger::{Basis, Body, Recheck};
use common::Fixture;

/// A branch `impl/a` changing a file of `a`, `main` moved on by a change of `b`, and a
/// `checked` record for the branch's head and for the moved `main`. Returns the head, the base
/// and the code keys of both.
fn moved(f: &Fixture) -> (String, String, Basis, String) {
    let merge_base = f.rev("main");
    let head = f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    let base = f.advance_main("crates/b/src/lib.rs", "// b\n");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    let basis = Basis {
        head: f.mark_checked("origin/impl/a"),
        base: f.mark_checked("origin/main"),
    };
    (head, base, basis, merge_base)
}

fn landed(f: &Fixture) -> (Option<String>, Option<Recheck>, Option<u64>) {
    match f.landed().pop().expect("a landed record") {
        Body::Landed {
            code_key,
            recheck,
            recheck_files,
            ..
        } => (code_key, recheck, recheck_files),
        other => panic!("{other:?}"),
    }
}

/// The `basis` of the `checked` record for `key`, if it has one.
fn basis_of(f: &Fixture, key: &str) -> Vec<Option<Basis>> {
    f.records()
        .into_iter()
        .filter_map(|r| match r.body {
            Body::Checked {
                code_key, basis, ..
            } if code_key == key => Some(basis),
            _ => None,
        })
        .collect()
}

fn has_recheck_line(warnings: &[String]) -> bool {
    warnings.iter().any(|w| w.starts_with("re-check:"))
}

#[test]
fn main_moved_on_an_unrelated_component() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    let (head, base, basis, merge_base) = moved(&f);
    // The check lock held by another process: the land takes neither lock, so it doesn't wait.
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/check.lock")).unwrap();
    held.lock().unwrap();
    // The land runs beside a deadline: one that waits for the lock is released after it and
    // fails the test, rather than hanging on a lock the test itself holds.
    let env = f.env(&[]);
    let (done, wait) = std::sync::mpsc::channel();
    let (out, waited) = std::thread::scope(|s| {
        let land = s.spawn(|| {
            let out = clm_land::land(&f.work(), Some("impl/a"), &env);
            done.send(()).unwrap();
            out
        });
        let waited = wait
            .recv_timeout(std::time::Duration::from_secs(30))
            .is_err();
        if waited {
            held.unlock().unwrap();
        }
        (land.join().unwrap().unwrap(), waited)
    });
    assert!(!waited, "the land waited for the check lock");
    assert_eq!(out.lines, [format!("landed {} code", f.remote("main"))]);
    assert!(f.lines("check.log").is_empty(), "the check ran");
    assert_eq!(
        f.lines("affected.log"),
        [format!("--overlap {head} {base}")]
    );
    let short = f.git(&f.work(), &["rev-parse", "--short", &merge_base]);
    let line =
        format!("re-check: none, the branch and main since {short} affect no test in common");
    assert!(out.warnings.contains(&line), "{:?}", out.warnings);
    // `clm check` ran with its detectors, in a temporary worktree.
    let clm_check = f.lines("clm-check.log");
    assert_eq!(clm_check.len(), 1);
    assert!(!clm_check[0].contains("--no-detectors"), "{clm_check:?}");
    assert!(f.lines("clm-check-pwd.log")[0].starts_with(f.tmpdir().to_str().unwrap()));
    f.assert_no_scratch();
    let (key, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::None), None));
    assert_eq!(basis_of(&f, &key.unwrap()), [Some(basis)]);
}

#[test]
fn a_dependant_of_both() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    let (head, base, basis, _) = moved(&f);
    f.write("overlap-out", "file crates/c/tests/t.rs\n");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let copied = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert_eq!(copied, "file crates/c/tests/t.rs\n");
    assert_eq!(
        f.lines("affected.log"),
        [format!("--overlap {head} {base}")]
    );
    assert!(
        out.warnings
            .contains(&"re-check: overlap, 1 files".to_string()),
        "{:?}",
        out.warnings
    );
    let (key, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::Overlap), Some(1)));
    assert_eq!(basis_of(&f, &key.unwrap()), [Some(basis)]);
    // The check ran under the clone's lock, in the land tree.
    assert_eq!(f.lines("check-lock.log"), ["held"]);
    assert_eq!(f.lines("check-dir.log"), ["land-tree"]);
    assert_eq!(f.lines("clm-check.log").len(), 1);
}

#[test]
fn an_all_line_runs_every_test() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    moved(&f);
    f.write("overlap-out", "all crates/c/Cargo.toml\n");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(
        out.warnings
            .contains(&"re-check: overlap, every test".to_string()),
        "{:?}",
        out.warnings
    );
    assert_eq!(landed(&f).1, Some(Recheck::Overlap));
    assert_eq!(landed(&f).2, None);
}

#[test]
fn the_overlap_fails() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    moved(&f);
    f.write("affected-fails", "");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!has_recheck_line(&out.warnings), "{:?}", out.warnings);
    // The overlap was tried first, then the land's own selection.
    let log = f.lines("affected.log");
    assert!(log[0].starts_with("--overlap "), "{log:?}");
    assert!(log[1].starts_with("--base "), "{log:?}");
    // Its own selection, which failed too: the check ran every test.
    assert_eq!(f.lines("check-selection.log"), ["CLM_SELECTION=unset"]);
    let (key, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::Selected), None));
    assert_eq!(basis_of(&f, &key.unwrap()), [None]);
}

#[test]
fn the_head_was_never_checked_whole() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    let base = f.advance_main("crates/b/src/lib.rs", "// b\n");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    f.mark_checked("origin/main");
    f.write("affected-out", "file crates/a/tests/t.rs\n");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!has_recheck_line(&out.warnings), "{:?}", out.warnings);
    assert_eq!(f.lines("affected.log"), [format!("--base {base}")]);
    let copied = std::fs::read_to_string(f.path("selection-copy")).unwrap();
    assert_eq!(copied, "file crates/a/tests/t.rs\n");
    let (key, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::Selected), Some(1)));
    assert_eq!(basis_of(&f, &key.unwrap()), [None]);
}

#[test]
fn mains_head_was_not_checked() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    let base = f.advance_main("crates/b/src/lib.rs", "// b\n");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    f.mark_checked("origin/impl/a");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!has_recheck_line(&out.warnings), "{:?}", out.warnings);
    assert_eq!(f.lines("affected.log"), [format!("--base {base}")]);
    assert_eq!(landed(&f).1, Some(Recheck::Selected));
}

#[test]
fn a_head_that_contains_main_is_not_composed() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    f.mark_checked("origin/impl/a");
    f.mark_checked("origin/main");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!has_recheck_line(&out.warnings), "{:?}", out.warnings);
    assert!(
        f.lines("affected.log")[0].starts_with("--base "),
        "{:?}",
        f.lines("affected.log")
    );
}

#[test]
fn checks_made_with_another_command_do_not_count() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    let base = f.advance_main("crates/b/src/lib.rs", "// b\n");
    f.git(&f.work(), &["fetch", "-q", "origin"]);
    for rev in ["origin/impl/a", "origin/main"] {
        f.append(
            &f.work(),
            Body::Checked {
                code_key: clm_ledger::code_key(&f.work(), rev).unwrap(),
                command: "some other command".into(),
                basis: None,
            },
        );
    }
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(!has_recheck_line(&out.warnings), "{:?}", out.warnings);
    assert_eq!(f.lines("affected.log"), [format!("--base {base}")]);
}

#[test]
fn a_land_says_how_it_checked() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let (_, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::Checked), None));
}

/// `impl/a` unchecked and in its check, which waits for the file `release`, while `impl/b`,
/// whose code is checked, lands on `main`; then the check ends. Returns the outcome of
/// `impl/a`'s land.
fn main_moves_during_the_check(f: &Fixture, first: &str, other: &str) -> clm_land::Output {
    f.write("hold", "");
    let (a, b) = (f.linked("wa", first), f.linked("wb", other));
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let land = s.spawn(|| clm_land::land(&a, None, &env));
        let started = std::time::Instant::now();
        while f.lines("check-order.log").is_empty() {
            assert!(started.elapsed().as_secs() < 30, "the check never started");
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        // The other branch changes no file the readers select.
        let _ = std::fs::remove_file(f.path("readers-out"));
        let landed = clm_land::land(&b, None, &env).unwrap();
        assert!(landed.lines[0].starts_with("landed "), "{:?}", landed.lines);
        f.write("release", "");
        land.join().unwrap().unwrap()
    })
}

#[test]
fn main_moved_during_the_check_on_an_unrelated_component() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "// b\n")]);
    f.mark_checked("origin/impl/b");
    let out = main_moves_during_the_check(&f, "impl/a", "impl/b");
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert_eq!(f.lines("check-order.log").len(), 2, "one start and its end");
    assert_eq!(f.lines("check.log").len(), 1);
    assert!(
        out.warnings.iter().any(|w| w.starts_with("re-check: none")),
        "{:?}",
        out.warnings
    );
    // The record for the tree it landed names the merge commit it checked and the `main` it
    // landed on.
    let main = f.remote("main");
    let key = clm_ledger::code_key(&f.path("origin.git"), &main).unwrap();
    let checked_keys: Vec<String> = f
        .records()
        .into_iter()
        .filter_map(|r| match r.body {
            Body::Checked {
                code_key,
                basis: None,
                ..
            } => Some(code_key),
            _ => None,
        })
        .collect();
    let [Some(basis)] = &basis_of(&f, &key)[..] else {
        panic!("{:?}", basis_of(&f, &key))
    };
    assert!(checked_keys.contains(&basis.head), "{basis:?}");
    let base = f.git(&f.work(), &["rev-parse", &format!("{main}^1")]);
    assert_eq!(
        basis.base,
        clm_ledger::code_key(&f.path("origin.git"), &base).unwrap()
    );
    assert_ne!(basis.head, basis.base);
    let (_, recheck, files) = landed(&f);
    assert_eq!((recheck, files), (Some(Recheck::None), None));
}

#[test]
fn main_moved_during_a_readers_check() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "// b\n")]);
    f.mark_checked("origin/main");
    f.mark_checked("origin/impl/b");
    f.write("readers-out", "file crates/a/tests/t.rs\n");
    let out = main_moves_during_the_check(&f, "spec/x", "impl/b");
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert_eq!(f.lines("check.log").len(), 1);
}

#[test]
fn main_moved_during_the_check_with_a_dependant_in_common() {
    // req:land/overlap-recheck@0431f136
    let f = Fixture::new("");
    f.branch("impl/a", &[("crates/a/src/lib.rs", "// a\n")]);
    f.branch("impl/b", &[("crates/b/src/lib.rs", "// b\n")]);
    f.mark_checked("origin/impl/b");
    f.write("overlap-out", "file crates/c/tests/t.rs\n");
    let out = main_moves_during_the_check(&f, "impl/a", "impl/b");
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert_eq!(f.lines("check.log").len(), 2, "its check ran twice");
    assert_eq!(
        std::fs::read_to_string(f.path("selection-copy")).unwrap(),
        "file crates/c/tests/t.rs\n"
    );
    assert!(
        out.warnings
            .contains(&"re-check: overlap, 1 files".to_string()),
        "{:?}",
        out.warnings
    );
}
