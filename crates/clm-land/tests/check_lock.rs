//! One `[land] check` at a time on the clone (land/check-lock).

mod common;

use common::{Fixture, refused};
use std::time::{Duration, Instant};

/// A check that logs `start` and `end` with its directory around a sleep, to a log outside
/// every worktree.
fn logging_check(f: &Fixture) {
    let log = f.path("overlap.log");
    let script = format!(
        "#!/bin/sh\necho \"start $(pwd -P)\" >> '{log}'\nsleep 1\necho \"end $(pwd -P)\" >> '{log}'\n",
        log = log.display()
    );
    common::write_exec(&f.path("bin/check"), &script);
}

/// Every `start` is followed by the `end` of the same directory before any other `start`.
fn assert_serial(f: &Fixture, runs: usize) {
    let lines = f.lines("overlap.log");
    assert!(
        lines.len() >= runs * 2 && lines.len().is_multiple_of(2),
        "{lines:?}"
    );
    for pair in lines.chunks(2) {
        let dir = pair[0].strip_prefix("start ").expect("start");
        assert_eq!(pair[1], format!("end {dir}"), "{lines:?}");
    }
}

fn assert_unlocked(f: &Fixture) {
    let lock = std::fs::File::open(f.work().join(".clm/check.lock")).unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    while lock.try_lock().is_err() {
        assert!(Instant::now() < deadline, "the check lock is still held");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn overlapping_check_only_runs() {
    // req:land/check-lock@069468cd
    let f = Fixture::new("");
    logging_check(&f);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("wa", "impl/a");
    let b = f.linked("wb", "impl/b");
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let one = s.spawn(|| clm_land::check_only(&a, None, &env));
        let two = s.spawn(|| clm_land::check_only(&b, None, &env));
        for out in [one.join().unwrap().unwrap(), two.join().unwrap().unwrap()] {
            assert_eq!(out.lines, ["checks pass"]);
        }
    });
    assert_serial(&f, 2);
}

#[test]
fn a_check_only_run_beside_a_land() {
    // req:land/check-lock@069468cd
    let f = Fixture::new("");
    logging_check(&f);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let linked = f.linked("wb", "impl/b");
    let env = f.env(&[]);
    std::thread::scope(|s| {
        let land = s.spawn(|| clm_land::land(&f.work(), Some("impl/a"), &env));
        let only = s.spawn(|| clm_land::check_only(&linked, None, &env));
        let landed = land.join().unwrap().unwrap();
        assert!(landed.lines[0].starts_with("landed "), "{:?}", landed.lines);
        assert_eq!(only.join().unwrap().unwrap().lines, ["checks pass"]);
    });
    assert_serial(&f, 2);
}

#[test]
fn a_land_with_the_callers_target() {
    // req:land/check-lock@069468cd
    let f = Fixture::new("");
    logging_check(&f);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    let a = f.linked("wa", "impl/a");
    let b = f.linked("wb", "impl/b");
    let ta = f.path("target-a");
    let tb = f.path("target-b");
    let ea = f.env(&[("CARGO_TARGET_DIR", ta.to_str().unwrap())]);
    let eb = f.env(&[("CARGO_TARGET_DIR", tb.to_str().unwrap())]);
    std::thread::scope(|s| {
        let one = s.spawn(|| clm_land::land(&a, None, &ea));
        let two = s.spawn(|| clm_land::land(&b, None, &eb));
        for out in [one.join().unwrap().unwrap(), two.join().unwrap().unwrap()] {
            assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
        }
    });
    assert_serial(&f, 2);
}

#[test]
fn released_after_a_failing_check() {
    // req:land/check-lock@069468cd
    let f = Fixture::new("");
    f.write("check-fails", "");
    for _ in 0..2 {
        let why = refused(clm_land::check_only(&f.work(), None, &f.env(&[])));
        assert!(why.contains(&f.check_command()), "{why}");
        assert_unlocked(&f);
    }
    assert!(f.lines("clm-check.log").is_empty());
}

#[test]
fn checked_code_takes_no_check_lock() {
    // req:land/check-lock@069468cd
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    f.mark_checked("origin/main");
    std::fs::create_dir_all(f.work().join(".clm")).unwrap();
    let held = std::fs::File::create(f.work().join(".clm/check.lock")).unwrap();
    held.lock().unwrap();
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    assert!(f.lines("check.log").is_empty());
}
